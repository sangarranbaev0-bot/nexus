//! Drafts: auto-save unsent message content per user-channel pair.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{AppState, auth::AuthUser, error::{AppError, AppResult}, protocol::{Draft, SaveDraftRequest}, schema::{channels, drafts}};

type DraftRow = (i64, i64, i64, String, DateTime<Utc>, DateTime<Utc>);

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/channels/{id}/draft", get(get_draft).post(save_draft))
        .route("/api/channels/{id}/draft", delete(clear_draft))
}

async fn get_draft(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
) -> AppResult<Json<Option<Draft>>> {
    let mut conn = state.pool.get().await?;
    let channel_exists: bool = channels::table
        .filter(channels::id.eq(channel_id))
        .select(channels::id)
        .first::<i64>(&mut conn)
        .await
        .optional()?
        .is_some();
    if !channel_exists {
        return Err(AppError::not_found("channel"));
    }
    let row: Option<DraftRow> = drafts::table
        .filter(drafts::user_id.eq(user_id))
        .filter(drafts::channel_id.eq(channel_id))
        .select((drafts::id, drafts::user_id, drafts::channel_id, drafts::content,
                  drafts::created_at, drafts::updated_at))
        .first(&mut conn)
        .await
        .optional()?;
    Ok(Json(row.map(|(id, _, cid, content, ca, ua)| Draft {
        id, user_id, channel_id: cid, content,
        created_at: ca.timestamp_millis(), updated_at: ua.timestamp_millis(),
    })))
}

async fn save_draft(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
    Json(req): Json<SaveDraftRequest>,
) -> AppResult<Json<Draft>> {
    let content = req.content.trim().to_string();
    if content.len() > 4000 {
        return Err(AppError::bad_request("draft too long (max 4000 chars)"));
    }
    let mut conn = state.pool.get().await?;
    let now = Utc::now();
    // Сначала пытаемся вставить. При unique violation — обновляем.
    let draft = diesel::insert_into(drafts::table)
        .values((
            drafts::user_id.eq(user_id),
            drafts::channel_id.eq(channel_id),
            drafts::content.eq(&content),
        ))
        .on_conflict((drafts::user_id, drafts::channel_id))
        .do_update()
        .set((
            drafts::content.eq(&content),
            drafts::updated_at.eq(now),
        ))
        .returning((
            drafts::id,
            drafts::user_id,
            drafts::channel_id,
            drafts::content,
            drafts::created_at,
            drafts::updated_at,
        ))
        .get_result::<DraftRow>(&mut conn)
        .await;
    let draft = match draft {
        Ok(r) => r,
        Err(diesel::result::Error::DatabaseError(_, info))
            if info.message().contains("unique") || info.message().contains("UniqueViolation") => {
                let rows: Vec<DraftRow> = diesel::update(
                    drafts::table.filter(
                        drafts::user_id.eq(user_id).and(drafts::channel_id.eq(channel_id)),
                    ),
                )
                .set((
                    drafts::content.eq(&content),
                    drafts::updated_at.eq(now),
                ))
                .returning((
                    drafts::id,
                    drafts::user_id,
                    drafts::channel_id,
                    drafts::content,
                    drafts::created_at,
                    drafts::updated_at,
                ))
                .load(&mut conn)
                .await?;
                rows.into_iter().next().ok_or_else(|| AppError::not_found("draft"))?
            }
        Err(e) => return Err(e.into()),
    };
    Ok(Json(Draft {
        id: draft.0, user_id: draft.1, channel_id: draft.2,
        content: draft.3,
        created_at: draft.4.timestamp_millis(), updated_at: draft.5.timestamp_millis(),
    }))
}

async fn clear_draft(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let deleted = diesel::delete(
        drafts::table.filter(drafts::user_id.eq(user_id).and(drafts::channel_id.eq(channel_id))),
    )
    .execute(&mut conn)
    .await?;
    if deleted == 0 {
        return Err(AppError::not_found("draft"));
    }
    Ok(StatusCode::NO_CONTENT)
}
