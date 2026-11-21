use axum::{
    Json, Router,
    extract::{Path, State},
    routing::put,
};
use diesel::sql_types::{BigInt, Text};
use diesel_async::RunQueryDsl;

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    events::{Notice, notify},
    messages::{load_message, locate},
    permissions,
    protocol::Message,
};

pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/api/messages/{id}/reactions/{emoji}",
        put(add).delete(remove),
    )
}

type MessagePath = (i64, String);

/// Эмодзи-последовательность до 64 символов без пробелов и управляющих символов.
pub fn validate(raw: &str) -> AppResult<&str> {
    let emoji = raw.trim();
    let len = emoji.chars().count();
    if len == 0 || len > 64 || emoji.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(AppError::bad_request("invalid emoji"));
    }
    Ok(emoji)
}

async fn authorize(
    conn: &mut diesel_async::AsyncPgConnection,
    server_id: Option<i64>,
    channel_id: i64,
    user_id: i64,
) -> AppResult<()> {
    if let Some(server_id) = server_id {
        permissions::require_channel_permissions(
            conn,
            server_id,
            user_id,
            channel_id,
            permissions::VIEW_CHANNEL | permissions::ADD_REACTIONS,
        )
        .await?;
    }
    Ok(())
}

async fn add(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((message_id, emoji)): Path<MessagePath>,
) -> AppResult<Json<Message>> {
    let emoji = validate(&emoji)?.to_string();
    let mut conn = state.pool.get().await?;
    let (channel_id, _author_id, access) = locate(&mut conn, message_id, user_id).await?;
    authorize(&mut conn, access.server_id, channel_id, user_id).await?;

    diesel::sql_query(
        "INSERT INTO message_reactions (message_id, user_id, emoji) VALUES ($1, $2, $3) \
         ON CONFLICT DO NOTHING",
    )
    .bind::<BigInt, _>(message_id)
    .bind::<BigInt, _>(user_id)
    .bind::<Text, _>(&emoji)
    .execute(&mut conn)
    .await?;

    notify(
        &mut conn,
        &Notice::MessageUpdated {
            server_id: access.server_id,
            message_id,
        },
    )
    .await?;
    let message = load_message(&mut conn, &state.file_key, message_id)
        .await?
        .ok_or_else(|| AppError::not_found("message"))?;
    Ok(Json(message))
}

async fn remove(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((message_id, emoji)): Path<MessagePath>,
) -> AppResult<Json<Message>> {
    let emoji = validate(&emoji)?.to_string();
    let mut conn = state.pool.get().await?;
    let (channel_id, _author_id, access) = locate(&mut conn, message_id, user_id).await?;
    authorize(&mut conn, access.server_id, channel_id, user_id).await?;

    diesel::sql_query(
        "DELETE FROM message_reactions WHERE message_id = $1 AND user_id = $2 AND emoji = $3",
    )
    .bind::<BigInt, _>(message_id)
    .bind::<BigInt, _>(user_id)
    .bind::<Text, _>(&emoji)
    .execute(&mut conn)
    .await?;

    if let Some(server_id) = access.server_id {
        crate::reaction_roles::apply(&mut conn, server_id, message_id, &emoji, user_id, false)
            .await?;
    }

    notify(
        &mut conn,
        &Notice::MessageUpdated {
            server_id: access.server_id,
            message_id,
        },
    )
    .await?;
    let message = load_message(&mut conn, &state.file_key, message_id)
        .await?
        .ok_or_else(|| AppError::not_found("message"))?;
    Ok(Json(message))
}
