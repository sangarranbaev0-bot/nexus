//! Scheduled messages: create, list pending, and a periodic sender task.

use std::time::Duration;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use tokio::time::interval;

use crate::{
    AppState, auth::AuthUser, error::{AppError, AppResult},
    events::{Notice, notify},
    messages::validate,
    protocol::{ChannelKind, CreateScheduledMessageRequest, ScheduledMessage},
    schema::{channels, messages as msg_rows, scheduled_messages as sched_rows},
    servers::channel_access,
};

const POLL_INTERVAL_SECS: u64 = 10;

type ScheduledRow = (
    i64, i64, i64, String,
    DateTime<Utc>, String,
    Option<DateTime<Utc>>, Option<String>, DateTime<Utc>,
);

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/scheduled-messages", get(list_scheduled).post(create_scheduled))
        .route("/api/scheduled-messages/{id}", delete(delete_scheduled))
}

/// Запланировать отправку сообщения. Статус → 'pending'.
async fn create_scheduled(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<CreateScheduledMessageRequest>,
) -> AppResult<(StatusCode, Json<ScheduledMessage>)> {
    let channel_id = req.channel_id;
    let access = {
        let mut conn = state.pool.get().await?;
        channel_access(&mut conn, channel_id, user_id).await?
    };
    if access.kind == ChannelKind::Voice {
        return Err(AppError::bad_request("voice channels have no scheduled messages"));
    }
    let _content = validate(&req.content, false)?;

    let scheduled_at = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(req.scheduled_for)
        .ok_or_else(|| AppError::bad_request("scheduled_for must be a valid unix ms timestamp"))?;
    if scheduled_at <= Utc::now() {
        return Err(AppError::bad_request("scheduled time must be in the future"));
    }

    let mut conn = state.pool.get().await?;
    let id: i64 = diesel::insert_into(sched_rows::table)
        .values((
            sched_rows::author_id.eq(user_id),
            sched_rows::channel_id.eq(channel_id),
            sched_rows::content.eq(&req.content),
            sched_rows::scheduled_at.eq(scheduled_at),
            sched_rows::status.eq("pending"),
        ))
        .returning(sched_rows::id)
        .get_result(&mut conn)
        .await?;
    let row: ScheduledRow = sched_rows::table.find(id)
        .select((
            sched_rows::id, sched_rows::author_id, sched_rows::channel_id,
            sched_rows::content, sched_rows::scheduled_at, sched_rows::status,
            sched_rows::sent_at, sched_rows::error_msg, sched_rows::created_at,
        ))
        .first(&mut conn)
        .await?;
    Ok((StatusCode::CREATED, Json(ScheduledMessage {
        id: row.0, author_id: row.1, channel_id: row.2, content: row.3,
        scheduled_at: row.4.timestamp_millis(),
        status: row.5, sent_at: row.6.map(|t| t.timestamp_millis()),
        error_msg: row.7, created_at: row.8.timestamp_millis(),
    })))
}

async fn list_scheduled(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> AppResult<Json<Vec<ScheduledMessage>>> {
    let mut conn = state.pool.get().await?;
    let rows: Vec<ScheduledRow> = sched_rows::table
        .filter(sched_rows::author_id.eq(user_id))
        .order(sched_rows::scheduled_at.desc())
        .select((
            sched_rows::id, sched_rows::author_id, sched_rows::channel_id,
            sched_rows::content, sched_rows::scheduled_at, sched_rows::status,
            sched_rows::sent_at, sched_rows::error_msg, sched_rows::created_at,
        ))
        .load(&mut conn)
        .await?;
    Ok(Json(rows.into_iter().map(|r| ScheduledMessage {
        id: r.0, author_id: r.1, channel_id: r.2, content: r.3,
        scheduled_at: r.4.timestamp_millis(),
        status: r.5, sent_at: r.6.map(|t| t.timestamp_millis()),
        error_msg: r.7, created_at: r.8.timestamp_millis(),
    }).collect()))
}

async fn delete_scheduled(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let deleted = diesel::delete(
        sched_rows::table.find(id).filter(sched_rows::author_id.eq(user_id)),
    )
    .execute(&mut conn)
    .await?;
    if deleted == 0 {
        return Err(AppError::not_found("scheduled message"));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Периодически проверяет pending-сообщения и отправляет их.
pub async fn spawn_sender_task(state: AppState) {
    let mut interval = interval(Duration::from_secs(POLL_INTERVAL_SECS));
    loop {
        interval.tick().await;
        if let Err(e) = tick_send_pending(&state).await {
            tracing::warn!("scheduled message sender tick failed: {e:?}");
        }
    }
}

async fn tick_send_pending(state: &AppState) -> AppResult<()> {
    let now = Utc::now();
    let mut conn = state.pool.get().await?;
    let ids: Vec<i64> = sched_rows::table
        .filter(sched_rows::status.eq("pending"))
        .filter(sched_rows::scheduled_at.le(now))
        .select(sched_rows::id)
        .load(&mut conn)
        .await?;
    if ids.is_empty() {
        return Ok(());
    }
    for id in ids {
        if let Err(e) = try_send_one(&mut conn, state, id).await {
            tracing::warn!("failed to send scheduled message {id}: {e:?}");
            diesel::update(sched_rows::table.find(id))
                .set((
                    sched_rows::status.eq("failed"),
                    sched_rows::error_msg.eq(format!("{e:?}")),
                ))
                .execute(&mut conn)
                .await?;
        }
    }
    Ok(())
}

async fn try_send_one(conn: &mut AsyncPgConnection, _state: &AppState, id: i64) -> AppResult<()> {
    let row: (i64, i64, String) = sched_rows::table.find(id)
        .select((sched_rows::author_id, sched_rows::channel_id, sched_rows::content))
        .first(conn)
        .await?;
    let (author_id, channel_id, content) = row;
    let (_server_id, kind): (Option<i64>, String) = channels::table
        .find(channel_id)
        .select((channels::server_id, channels::kind))
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| AppError::not_found("channel"))?;
    if ChannelKind::parse(&kind) == ChannelKind::Voice {
        return Err(AppError::bad_request("voice channels cannot receive scheduled messages"));
    }
    let validated = validate(&content, false)?;
    let msg_id: i64 = diesel::insert_into(msg_rows::table)
        .values((
            msg_rows::channel_id.eq(channel_id),
            msg_rows::author_id.eq(author_id),
            msg_rows::content.eq(&validated),
            msg_rows::has_embed.eq(false),
        ))
        .returning(msg_rows::id)
        .get_result(conn)
        .await?;
    diesel::update(sched_rows::table.find(id))
        .set((
            sched_rows::status.eq("sent"),
            sched_rows::sent_at.eq(Some(chrono::Utc::now())),
        ))
        .execute(conn)
        .await?;
    if let Some(sid) = _server_id {
        notify(conn, &Notice::MessageCreated { server_id: Some(sid), message_id: msg_id }).await?;
    }
    Ok(())
}
