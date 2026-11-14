use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch, put},
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_types::{Array, BigInt, Text};
use std::collections::HashMap;

use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use serde::Deserialize;

use crate::{
    AppState, attachments,
    auth::AuthUser,
    error::{AppError, AppResult},
    events::{Notice, notify},
    permissions,
    protocol::{Attachment, ChannelKind, Message, MessageContent, Reaction, ReplyPreview},
    schema::{messages, users},
    servers::{ChannelAccess, channel_access},
};

const MAX_LEN: usize = 2000;
const DEFAULT_PAGE: i64 = 50;
const MAX_PAGE: i64 = 100;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/channels/{id}/messages", get(list).post(create))
        .route("/api/channels/{id}/pins", get(pins))
        .route("/api/messages/{id}", patch(update).delete(remove))
        .route("/api/messages/{id}/pin", put(pin).delete(unpin))
}

type Row = (
    i64,
    i64,
    i64,
    String,
    String,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<i64>,
    Option<DateTime<Utc>>,
);

fn columns() -> (
    messages::id,
    messages::channel_id,
    messages::author_id,
    users::username,
    messages::content,
    messages::created_at,
    messages::edited_at,
    messages::reply_to_id,
    messages::pinned_at,
) {
    (
        messages::id,
        messages::channel_id,
        messages::author_id,
        users::username,
        messages::content,
        messages::created_at,
        messages::edited_at,
        messages::reply_to_id,
        messages::pinned_at,
    )
}

fn to_message(
    (id, channel_id, author_id, author_username, content, created, edited, _, pinned_at): Row,
    attachments: &mut HashMap<i64, Vec<Attachment>>,
    reactions: Vec<Reaction>,
    reply_to: Option<ReplyPreview>,
) -> Message {
    Message {
        id,
        channel_id,
        author_id,
        author_username,
        content,
        created_at: created.timestamp_millis(),
        edited_at: edited.map(|t| t.timestamp_millis()),
        attachments: attachments.remove(&id).unwrap_or_default(),
        reactions,
        reply_to,
        pinned: pinned_at.is_some(),
    }
}

#[derive(QueryableByName)]
struct ReactionRow {
    #[diesel(sql_type = BigInt)]
    message_id: i64,
    #[diesel(sql_type = Text)]
    emoji: String,
    #[diesel(sql_type = Array<BigInt>)]
    user_ids: Vec<i64>,
}

/// Реакции сразу для всех сообщений страницы: один запрос, порядок по первому появлению.
async fn reactions_for(
    conn: &mut AsyncPgConnection,
    ids: &[i64],
) -> AppResult<HashMap<i64, Vec<Reaction>>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<ReactionRow> = diesel::sql_query(
        "SELECT message_id, emoji, array_agg(user_id ORDER BY created_at, user_id) AS user_ids \
         FROM message_reactions WHERE message_id = ANY($1) \
         GROUP BY message_id, emoji ORDER BY min(created_at), emoji",
    )
    .bind::<Array<BigInt>, _>(ids.to_vec())
    .load(conn)
    .await?;
    let mut out: HashMap<i64, Vec<Reaction>> = HashMap::new();
    for row in rows {
        out.entry(row.message_id).or_default().push(Reaction {
            emoji: row.emoji,
            user_ids: row.user_ids,
        });
    }
    Ok(out)
}

/// Строки сообщений вместе с вложениями и реакциями (по запросу на страницу).
async fn with_extras(
    conn: &mut AsyncPgConnection,
    key: &[u8],
    rows: Vec<Row>,
) -> AppResult<Vec<Message>> {
    let ids: Vec<i64> = rows.iter().map(|r| r.0).collect();
    let mut files = attachments::for_messages(conn, key, &ids).await?;
    let mut reactions = reactions_for(conn, &ids).await?;
    let reply_ids: Vec<i64> = rows.iter().filter_map(|r| r.7).collect();
    let replies = reply_previews(conn, &reply_ids).await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let reactions = reactions.remove(&r.0).unwrap_or_default();
            let reply_to = r.7.and_then(|id| replies.get(&id).cloned());
            to_message(r, &mut files, reactions, reply_to)
        })
        .collect())
}

#[derive(QueryableByName)]
struct ReplyRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
    #[diesel(sql_type = BigInt)]
    author_id: i64,
    #[diesel(sql_type = Text)]
    author_username: String,
    #[diesel(sql_type = Text)]
    content: String,
}

/// Цитаты для ответов: одним запросом на всю страницу, текст обрезан до 200 символов.
async fn reply_previews(
    conn: &mut AsyncPgConnection,
    ids: &[i64],
) -> AppResult<HashMap<i64, ReplyPreview>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<ReplyRow> = diesel::sql_query(
        "SELECT m.id, m.author_id, u.username AS author_username, m.content \
         FROM messages m JOIN users u ON u.id = m.author_id WHERE m.id = ANY($1)",
    )
    .bind::<Array<BigInt>, _>(ids.to_vec())
    .load(conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            (
                r.id,
                ReplyPreview {
                    id: r.id,
                    author_id: r.author_id,
                    author_username: r.author_username,
                    content: r.content.chars().take(200).collect(),
                },
            )
        })
        .collect())
}

pub async fn load_message(
    conn: &mut AsyncPgConnection,
    key: &[u8],
    id: i64,
) -> AppResult<Option<Message>> {
    let row: Option<Row> = messages::table
        .inner_join(users::table)
        .filter(messages::id.eq(id))
        .select(columns())
        .first(conn)
        .await
        .optional()?;
    Ok(with_extras(conn, key, row.into_iter().collect())
        .await?
        .pop())
}

/// Текст до 2000 символов; пустой допустим, только если в сообщении есть файлы.
fn validate(content: &str, has_files: bool) -> AppResult<String> {
    let content = content.trim();
    if (content.is_empty() && !has_files) || content.chars().count() > MAX_LEN {
        return Err(AppError::bad_request("message must be 1-2000 characters"));
    }
    Ok(content.to_string())
}

#[derive(Deserialize)]
struct Page {
    before: Option<i64>,
    limit: Option<i64>,
}

/// Сообщения старше `before` (или последние), в хронологическом порядке.
/// Если вернулось ровно `limit`, клиент может просить следующую страницу.
async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
    Query(page): Query<Page>,
) -> AppResult<Json<Vec<Message>>> {
    let limit = page.limit.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE);
    let mut conn = state.pool.get().await?;
    let access = channel_access(&mut conn, channel_id, user_id).await?;
    if let Some(server_id) = access.server_id {
        permissions::require_channel_permission(
            &mut conn,
            server_id,
            user_id,
            channel_id,
            permissions::VIEW_CHANNEL,
        )
        .await?;
    }

    let mut query = messages::table
        .inner_join(users::table)
        .filter(messages::channel_id.eq(channel_id))
        .order(messages::id.desc())
        .limit(limit)
        .select(columns())
        .into_boxed();
    if let Some(before) = page.before {
        query = query.filter(messages::id.lt(before));
    }
    let mut rows: Vec<Row> = query.load(&mut conn).await?;
    rows.reverse();
    Ok(Json(with_extras(&mut conn, &state.file_key, rows).await?))
}

/// Закреплённые сообщения канала, свежие сверху.
async fn pins(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
) -> AppResult<Json<Vec<Message>>> {
    let mut conn = state.pool.get().await?;
    let access = channel_access(&mut conn, channel_id, user_id).await?;
    if let Some(server_id) = access.server_id {
        permissions::require_channel_permissions(
            &mut conn,
            server_id,
            user_id,
            channel_id,
            permissions::VIEW_CHANNEL | permissions::READ_MESSAGE_HISTORY,
        )
        .await?;
    }
    let rows: Vec<Row> = messages::table
        .inner_join(users::table)
        .filter(messages::channel_id.eq(channel_id))
        .filter(messages::pinned_at.is_not_null())
        .order(messages::pinned_at.desc())
        .limit(50)
        .select(columns())
        .load(&mut conn)
        .await?;
    Ok(Json(with_extras(&mut conn, &state.file_key, rows).await?))
}

async fn pin(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(message_id): Path<i64>,
) -> AppResult<Json<Message>> {
    set_pinned(&state, user_id, message_id, true).await
}

async fn unpin(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(message_id): Path<i64>,
) -> AppResult<Json<Message>> {
    set_pinned(&state, user_id, message_id, false).await
}

async fn set_pinned(
    state: &AppState,
    user_id: i64,
    message_id: i64,
    value: bool,
) -> AppResult<Json<Message>> {
    let mut conn = state.pool.get().await?;
    let (channel_id, _author_id, access) = locate(&mut conn, message_id, user_id).await?;
    if let Some(server_id) = access.server_id {
        permissions::require_channel_permissions(
            &mut conn,
            server_id,
            user_id,
            channel_id,
            permissions::VIEW_CHANNEL | permissions::PIN_MESSAGES,
        )
        .await?;
    }
    let pinned_at = if value { Some(Utc::now()) } else { None };
    diesel::update(messages::table.find(message_id))
        .set(messages::pinned_at.eq(pinned_at))
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

async fn create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
    Json(req): Json<MessageContent>,
) -> AppResult<(StatusCode, Json<Message>)> {
    let mut files = req.attachment_ids.unwrap_or_default();
    files.sort_unstable();
    files.dedup();
    if files.len() > attachments::MAX_PER_MESSAGE {
        return Err(AppError::bad_request("too many attachments"));
    }
    let content = validate(&req.content, !files.is_empty())?;
    let mut conn = state.pool.get().await?;
    let access = channel_access(&mut conn, channel_id, user_id).await?;
    if access.kind == ChannelKind::Voice {
        return Err(AppError::bad_request("voice channels have no messages"));
    }
    let server_id = access.server_id;
    if let Some(reply_id) = req.reply_to_id {
        let target: Option<i64> = messages::table
            .find(reply_id)
            .select(messages::channel_id)
            .first(&mut conn)
            .await
            .optional()?;
        if target != Some(channel_id) {
            return Err(AppError::bad_request(
                "reply target must be in the same channel",
            ));
        }
    }
    if let Some(server_id) = server_id {
        permissions::require_channel_permission(
            &mut conn,
            server_id,
            user_id,
            channel_id,
            permissions::SEND_MESSAGES,
        )
        .await?;
        if !files.is_empty() {
            permissions::require_channel_permission(
                &mut conn,
                server_id,
                user_id,
                channel_id,
                permissions::ATTACH_FILES,
            )
            .await?;
        }
    }

    // Сообщение и привязка файлов — одна транзакция: чужой или уже занятый файл откатывает всё.
    let id = conn
        .transaction(async move |conn| -> AppResult<i64> {
            let id: i64 = diesel::insert_into(messages::table)
                .values((
                    messages::channel_id.eq(channel_id),
                    messages::author_id.eq(user_id),
                    messages::content.eq(&content),
                    messages::reply_to_id.eq(req.reply_to_id),
                ))
                .returning(messages::id)
                .get_result(conn)
                .await?;
            if !files.is_empty() {
                attachments::attach(conn, &files, id, channel_id, user_id).await?;
            }
            Ok(id)
        })
        .await?;
    notify(
        &mut conn,
        &Notice::MessageCreated {
            server_id,
            message_id: id,
        },
    )
    .await?;
    let message = load_message(&mut conn, &state.file_key, id)
        .await?
        .ok_or_else(|| AppError::not_found("message"))?;
    Ok((StatusCode::CREATED, Json(message)))
}

/// Канал, автор и доступ к сообщению. Посторонним 404, как и для серверов.
pub(crate) async fn locate(
    conn: &mut AsyncPgConnection,
    message_id: i64,
    user_id: i64,
) -> AppResult<(i64, i64, ChannelAccess)> {
    let (channel_id, author_id): (i64, i64) = messages::table
        .filter(messages::id.eq(message_id))
        .select((messages::channel_id, messages::author_id))
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| AppError::not_found("message"))?;
    let access = channel_access(conn, channel_id, user_id)
        .await
        .map_err(|_| AppError::not_found("message"))?;
    Ok((channel_id, author_id, access))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(message_id): Path<i64>,
    Json(req): Json<MessageContent>,
) -> AppResult<Json<Message>> {
    let mut conn = state.pool.get().await?;
    let (_, author_id, access) = locate(&mut conn, message_id, user_id).await?;
    let has_files = !attachments::for_messages(&mut conn, &state.file_key, &[message_id])
        .await?
        .is_empty();
    let content = validate(&req.content, has_files)?;
    let server_id = access.server_id;
    // Править может только автор, даже owner не меняет чужие слова.
    if author_id != user_id {
        return Err(AppError::forbidden());
    }
    diesel::update(messages::table.find(message_id))
        .set((
            messages::content.eq(&content),
            messages::edited_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .await?;
    notify(
        &mut conn,
        &Notice::MessageUpdated {
            server_id,
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
    Path(message_id): Path<i64>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let (channel_id, author_id, access) = locate(&mut conn, message_id, user_id).await?;
    // Чужое сообщение удаляет участник с правом MANAGE_MESSAGES; в ЛС — никто.
    if author_id != user_id {
        let allowed = match access.server_id {
            Some(server_id) => permissions::require_channel_permission(
                &mut conn,
                server_id,
                user_id,
                channel_id,
                permissions::MANAGE_MESSAGES,
            )
            .await
            .is_ok(),
            None => false,
        };
        if !allowed {
            return Err(AppError::forbidden());
        }
    }
    let server_id = access.server_id;
    let keys = conn
        .transaction(async move |conn| -> AppResult<Vec<String>> {
            let keys = attachments::detach_for_delete(conn, message_id).await?;
            diesel::delete(messages::table.find(message_id))
                .execute(conn)
                .await?;
            Ok(keys)
        })
        .await?;
    notify(
        &mut conn,
        &Notice::MessageDeleted {
            server_id,
            channel_id,
            message_id,
        },
    )
    .await?;
    drop(conn);
    attachments::delete_objects(&state, &keys).await;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_limits() {
        assert_eq!(validate("  привет  ", false).unwrap(), "привет");
        assert!(validate("   ", false).is_err());
        assert_eq!(validate("   ", true).unwrap(), "", "only files");
        assert!(validate(&"я".repeat(2000), false).is_ok());
        assert!(validate(&"я".repeat(2001), true).is_err());
    }
}
