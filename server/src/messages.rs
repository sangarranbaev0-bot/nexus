use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch, post, put},
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_types::{Array, BigInt, Text};
use regex::Regex;
use reqwest::header::ACCEPT;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

use crate::{
    AppState, attachments,
    auth::AuthUser,
    error::{AppError, AppResult},
    events::{Notice, notify},
    permissions,
    protocol::{
        Attachment, ChannelKind, EmbedPreview, ForwardPreview, ForwardRequest, Message, MessageContent,
        MessageEdit, Reaction, ReplyPreview, SearchParams, SearchResponse, SearchResult,
    },
    schema::{attachments as attachment_rows, channels, message_edits, messages, users},
    servers::{ChannelAccess, channel_access},
};

const MAX_LEN: usize = 2000;
const DEFAULT_PAGE: i64 = 50;
const MAX_PAGE: i64 = 100;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/channels/{id}/messages", get(list).post(create))
        .route("/api/channels/{id}/messages/search", get(search))
        .route("/api/channels/{id}/pins", get(pins))
        .route("/api/messages/{id}", patch(update).delete(remove))
        .route("/api/messages/{id}/pin", put(pin).delete(unpin))
        .route("/api/messages/{id}/edits", get(edits))
        .route("/api/messages/{id}/forward", post(forward))
        .route("/api/messages/{id}/embed", get(embed_preview))
}

type Row = (
    i64,
    i64,
    i64,
    String,
    String,
    String,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<i64>,
    Option<DateTime<Utc>>,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<String>,
    Option<String>,
    Option<i64>,
    bool,
    bool,
    bool,
);

type Columns = (
    messages::id,
    messages::channel_id,
    messages::author_id,
    users::username,
    users::badge,
    messages::content,
    messages::created_at,
    messages::edited_at,
    messages::reply_to_id,
    messages::pinned_at,
    messages::system_kind,
    messages::system_detail,
    messages::forwarded_from_id,
    messages::forwarded_channel_id,
    messages::forwarded_author_id,
    messages::forwarded_author,
    messages::forwarded_content,
    messages::thread_root_id,
    messages::thread_archived,
    messages::thread_locked,
    messages::has_embed,
);

fn columns() -> Columns {
    (
        messages::id,
        messages::channel_id,
        messages::author_id,
        users::username,
        users::badge,
        messages::content,
        messages::created_at,
        messages::edited_at,
        messages::reply_to_id,
        messages::pinned_at,
        messages::system_kind,
        messages::system_detail,
        messages::forwarded_from_id,
        messages::forwarded_channel_id,
        messages::forwarded_author_id,
        messages::forwarded_author,
        messages::forwarded_content,
            messages::thread_root_id,
            messages::thread_archived,
            messages::thread_locked,
            messages::has_embed,
        )
}

fn to_message(
    (
        id,
        channel_id,
        author_id,
        author_username,
        author_badge,
        content,
        created,
        edited,
        _,
        pinned_at,
        system_kind,
        system_detail,
        forwarded_from_id,
        forwarded_channel_id,
        forwarded_author_id,
        forwarded_author,
        forwarded_content,
        thread_root_id,
        thread_archived,
        thread_locked,
        has_embed,
    ): Row,
    attachments: &mut HashMap<i64, Vec<Attachment>>,
    reactions: Vec<Reaction>,
    reply_to: Option<ReplyPreview>,
) -> Message {
    Message {
        id,
        channel_id,
        author_id,
        author_username,
        author_badge,
        content,
        created_at: created.timestamp_millis(),
        edited_at: edited.map(|t| t.timestamp_millis()),
        attachments: attachments.remove(&id).unwrap_or_default(),
        reactions,
        reply_to,
         pinned: pinned_at.is_some(),
         system_kind,
         system_detail,
         forwarded_from: forwarded_author.map(|author_username| ForwardPreview {
             message_id: forwarded_from_id,
             author_id: forwarded_author_id,
             author_username,
             content: forwarded_content.unwrap_or_default(),
             channel_id: forwarded_channel_id,
         }),
         thread_root_id,
          thread_archived,
          thread_locked,
          has_embed,
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
    let reply_ids: Vec<i64> = rows.iter().filter_map(|r| r.8).collect();
    let replies = reply_previews(conn, &reply_ids).await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let reactions = reactions.remove(&r.0).unwrap_or_default();
            let reply_to = r.8.and_then(|id| replies.get(&id).cloned());
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
pub fn validate(content: &str, has_files: bool) -> AppResult<String> {
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

/// Поиск сообщений в канале с фильтрами.
async fn search(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
    Query(params): Query<SearchParams>,
) -> AppResult<Json<SearchResponse>> {
    let limit = params.limit.unwrap_or(50).clamp(1, 100);
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

    // Разбор фильтров: несколько токенов работают вместе, например `from:ann has:file отчёт`.
    let mut author: Option<String> = None;
    let mut has_file = false;
    let mut has_link = false;
    let mut terms: Vec<String> = Vec::new();
    for token in params.q.split_whitespace() {
        if let Some(name) = token.strip_prefix("from:").filter(|s| !s.is_empty()) {
            author = Some(name.to_string());
        } else if token == "has:file" || token == "has:attachment" {
            has_file = true;
        } else if token == "has:link" {
            has_link = true;
        } else {
            terms.push(token.to_string());
        }
    }

    let mut query = messages::table
        .inner_join(users::table)
        .filter(messages::channel_id.eq(channel_id))
        .order(messages::id.desc())
        .select(columns())
        .into_boxed();

    if let Some(name) = author {
        query = query.filter(users::username.ilike(name));
    }
    if has_file {
        // EXISTS по таблице вложений: не зависит от текста сообщения и не даёт ложных срабатываний.
        query =
            query
                .filter(diesel::dsl::exists(attachment_rows::table.filter(
                    attachment_rows::message_id.eq(messages::id.nullable()),
                )));
    }
    if has_link {
        query = query.filter(messages::content.ilike("%http%"));
    }
    for term in terms {
        query = query.filter(messages::content.ilike(format!("%{}%", term)));
    }
    if let Some(before) = params.before {
        query = query.filter(messages::id.lt(before));
    }
    if let Some(after) = params.after {
        query = query.filter(messages::id.gt(after));
    }
    // Берём на одну строку больше, чтобы has_more был точен без второго запроса COUNT.
    let mut rows: Vec<Row> = query.limit(limit + 1).load(&mut conn).await?;
    let has_more = rows.len() as i64 > limit;
    rows.truncate(limit as usize);
    rows.reverse();

    let channel_name: String = channels::table
        .find(channel_id)
        .select(channels::name)
        .first(&mut conn)
        .await
        .unwrap_or_default();

    let messages = with_extras(&mut conn, &state.file_key, rows).await?;
    let results = messages
        .into_iter()
        .map(|message| SearchResult {
            message_id: message.id,
            channel_id,
            server_id: access.server_id,
            channel_name: channel_name.clone(),
            message,
        })
        .collect();

    Ok(Json(SearchResponse { results, has_more }))
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
    headers: axum::http::HeaderMap,
    Json(req): Json<MessageContent>,
) -> AppResult<(StatusCode, Json<Message>)> {
    crate::rate_limit::messages(&headers).map_err(|(code, msg)| AppError(code, msg))?;
    let mut files = req.attachment_ids.unwrap_or_default();
    files.sort_unstable();
    files.dedup();
    if files.len() > attachments::MAX_PER_MESSAGE {
        return Err(AppError::bad_request("too many attachments"));
    }
    let content = validate(&req.content, !files.is_empty())?;
    let mut conn = state.pool.get().await?;
    let access = channel_access(&mut conn, channel_id, user_id).await?;
    if access.server_id.is_some() {
        let mentions = content.chars().filter(|c| *c == '@').count();
        if mentions > 5 {
            return Err(AppError::bad_request("too many mentions"));
        }
        let links = content.matches("http://").count() + content.matches("https://").count();
        if links > 3 {
            return Err(AppError::bad_request("too many links"));
        }
    }
    if access.kind == ChannelKind::Voice {
        return Err(AppError::bad_request("voice channels have no messages"));
    }
    if access.server_id.is_none()
        && crate::friends::dm_has_block(&mut conn, channel_id, user_id).await?
    {
        return Err(AppError::forbidden_with("blocked"));
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

    // Применить правила автомодерации сервера
    if let Some(sid) = server_id {
        #[derive(diesel::QueryableByName)]
        struct AutomodRow {
            #[diesel(sql_type = Text)]
            pattern: String,
            #[diesel(sql_type = Text)]
            action: String,
            #[diesel(sql_type = Text)]
            note: String,
        }
        let rules: Vec<AutomodRow> = diesel::sql_query(
            "SELECT pattern, action, COALESCE(note,'') AS note FROM automod_rules WHERE server_id=$1 ORDER BY id",
        )
        .bind::<BigInt, _>(sid)
        .load(&mut conn)
        .await?;

        for rule in &rules {
            let lower_pattern = rule.pattern.to_lowercase();
            let lower_content = content.to_lowercase();
            // Try regex first; fall back to simple substring match on failure
            let is_match = match Regex::new(&format!("(?i){}", regex::escape(&rule.pattern))) {
                Ok(re) => re.is_match(&content),
                Err(_) => lower_content.contains(&lower_pattern),
            };
            if is_match {
                if rule.action == "delete" {
                    return Err(AppError::bad_request(format!(
                        "сообщение нарушает правило модерации: {}",
                        if rule.note.is_empty() { "нарушение правил".to_string() } else { rule.note.clone() }
                    )));
                }
                tracing::info!(
                    "automod warn: user {} channel {} matched rule",
                    user_id, channel_id,
                );
            }
        }
    }

    // Сообщение и привязка файлов — одна транзакция: чужой или уже занятый файл откатывает всё.
    let content_for_tx = content.clone();
    let has_embed = !extract_urls(&content).is_empty();
    let id = conn
        .transaction(async move |conn| -> AppResult<i64> {
            let id: i64 = diesel::insert_into(messages::table)
                .values((
                    messages::channel_id.eq(channel_id),
                    messages::author_id.eq(user_id),
                    messages::content.eq(&content_for_tx),
                    messages::reply_to_id.eq(req.reply_to_id),
                    messages::has_embed.eq(has_embed),
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
    // Центр уведомлений: не мешает доставке, ошибки только логируем.
    if let Err(e) = crate::notifications::dispatch(
        &mut conn,
        &access,
        channel_id,
        id,
        user_id,
        &content,
        req.reply_to_id,
    )
    .await
    {
        tracing::warn!("notification dispatch failed for message {id}: {e:?}");
    }
    let message = load_message(&mut conn, &state.file_key, id)
        .await?
        .ok_or_else(|| AppError::not_found("message"))?;
    Ok((StatusCode::CREATED, Json(message)))
}

/// Пересылка: копия сообщения в другой канал со снимком исходного автора и текста.
/// Вложения не копируются — в пересланной копии остаётся только блок «Переслано от».
async fn forward(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(message_id): Path<i64>,
    Json(req): Json<ForwardRequest>,
) -> AppResult<(StatusCode, Json<Message>)> {
    let mut conn = state.pool.get().await?;
    let (source_channel, _, _) = locate(&mut conn, message_id, user_id).await?;
    let source: (i64, String, String, Option<String>, Option<String>) = messages::table
        .inner_join(users::table)
        .filter(messages::id.eq(message_id))
        .select((
            messages::author_id,
            users::username,
            messages::content,
            messages::system_kind,
            messages::system_detail,
        ))
        .first(&mut conn)
        .await?;
    if source.3.is_some() {
        return Err(AppError::bad_request("cannot forward a system message"));
    }
    if source_channel == req.channel_id {
        return Err(AppError::bad_request(
            "forward target must be another channel",
        ));
    }
    let target = channel_access(&mut conn, req.channel_id, user_id).await?;
    if target.kind == ChannelKind::Voice {
        return Err(AppError::bad_request("voice channels have no messages"));
    }
    if target.server_id.is_none()
        && crate::friends::dm_has_block(&mut conn, req.channel_id, user_id).await?
    {
        return Err(AppError::forbidden_with("blocked"));
    }
    if let Some(server_id) = target.server_id {
        permissions::require_channel_permission(
            &mut conn,
            server_id,
            user_id,
            req.channel_id,
            permissions::SEND_MESSAGES,
        )
        .await?;
    }
    let snapshot = source.2.chars().take(MAX_LEN).collect::<String>();
    let id: i64 = diesel::insert_into(messages::table)
        .values((
            messages::channel_id.eq(req.channel_id),
            messages::author_id.eq(user_id),
            messages::content.eq(""),
            messages::forwarded_from_id.eq(message_id),
            messages::forwarded_channel_id.eq(Some(source_channel)),
            messages::forwarded_author_id.eq(Some(source.0)),
            messages::forwarded_author.eq(source.1),
            messages::forwarded_content.eq(&snapshot),
        ))
        .returning(messages::id)
        .get_result(&mut conn)
        .await?;
    notify(
        &mut conn,
        &Notice::MessageCreated {
            server_id: target.server_id,
            message_id: id,
        },
    )
    .await?;
    let message = load_message(&mut conn, &state.file_key, id)
        .await?
        .ok_or_else(|| AppError::not_found("message"))?;
    Ok((StatusCode::CREATED, Json(message)))
}

/// Системное сообщение сервера: вставляет запись от лица `actor_id` с типом события
/// и рассылает `MessageCreated`. `content` пустой — клиент строит текст по `system_kind`.
pub async fn post_system(
    conn: &mut AsyncPgConnection,
    server_id: i64,
    channel_id: i64,
    actor_id: i64,
    kind: &str,
    detail: Option<&str>,
    content: &str,
) -> AppResult<i64> {
    let id: i64 = diesel::insert_into(messages::table)
        .values((
            messages::channel_id.eq(channel_id),
            messages::author_id.eq(actor_id),
            messages::content.eq(content),
            messages::system_kind.eq(kind),
            messages::system_detail.eq(detail),
        ))
        .returning(messages::id)
        .get_result(conn)
        .await?;
    notify(
        conn,
        &Notice::MessageCreated {
            server_id: Some(server_id),
            message_id: id,
        },
    )
    .await?;
    Ok(id)
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
    let previous: String = messages::table
        .find(message_id)
        .select(messages::content)
        .first(&mut conn)
        .await?;
    if previous != content {
        diesel::insert_into(message_edits::table)
            .values((
                message_edits::message_id.eq(message_id),
                message_edits::content.eq(&previous),
            ))
            .execute(&mut conn)
            .await?;
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

async fn edits(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(message_id): Path<i64>,
) -> AppResult<Json<Vec<MessageEdit>>> {
    let mut conn = state.pool.get().await?;
    let (channel_id, _, access) = locate(&mut conn, message_id, user_id).await?;
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
    let rows: Vec<(i64, String, DateTime<Utc>)> = message_edits::table
        .filter(message_edits::message_id.eq(message_id))
        .select((
            message_edits::id,
            message_edits::content,
            message_edits::edited_at,
        ))
        .order(message_edits::edited_at.desc())
        .load(&mut conn)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(id, content, edited_at)| MessageEdit {
                id,
                content,
                edited_at: edited_at.timestamp_millis(),
            })
            .collect(),
    ))
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

/// Кэш превью ссылок: url → (превью, unix-время в мс).
type EmbedCache = RwLock<HashMap<String, (EmbedPreview, u64)>>;
static EMBED_CACHE: once_cell::sync::Lazy<EmbedCache> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

const EMBED_TTL_MS: u64 = 10 * 60 * 1_000;

fn extract_urls(content: &str) -> Vec<String> {
    static RE: once_cell::sync::Lazy<Regex> =
        once_cell::sync::Lazy::new(|| Regex::new(r#"https?://[^\s<>"')},;:]+"#).unwrap());
    RE.find_iter(content)
        .map(|m| m.as_str().to_string())
        .take(1)
        .collect()
}

async fn fetch_embed(url: &str) -> Option<EmbedPreview> {
    if url.starts_with("http://") || url.starts_with("https://") {
        // Уже абсолютный.
    } else {
        return None;
    }
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (NexusChat Bot/1.0)")
        .redirect(reqwest::redirect::Policy::limited(3))
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client.get(url).header(ACCEPT, "text/html").send().await.ok()?;
    let body = resp.text().await.ok()?;
    if !body.contains("<meta") {
        return None;
    }
    let title = extract_meta_attr(&body, "og:title", Some("title"))
        .or_else(|| extract_title_tag(&body));
    let desc = extract_meta_attr(&body, "og:description", Some("description"));
    let image = extract_meta_attr(&body, "og:image", Some("image"));
    let site_name = extract_meta_attr(&body, "og:site_name", None);
    Some(EmbedPreview {
        url: url.to_string(),
        title,
        description: desc,
        image_url: image,
        site_name,
    })
}

fn extract_meta_attr(html: &str, prop: &str, _attr: Option<&str>) -> Option<String> {
    let needle = format!("property=\"{}\"", prop);
    let i = html.find(&needle)?;
    let rest = &html[i + needle.len()..];
    let quote = if let Some(i) = rest.find("content=") {
        let after = &rest[i + 8..];
        after.find('"').map(|j| &after[..j])
    } else if let Some(i) = rest.find("content='") {
        let after = &rest[i + 9..];
        after.find('\'').map(|j| &after[..j])
    } else if let Some(i) = rest.find("content=\"") {
        let after = &rest[i + 10..];
        after.find('"').map(|j| &after[..j])
    } else {
        None
    }?;
    quote.trim().to_string().into()
}

fn extract_title_tag(html: &str) -> Option<String> {
    let open = html.find("<title")?;
    let close = html[open..].find("</title>")?;
    let tag_end = html[open..].find('>')?;
    Some(html[open + tag_end + 1..open + close].trim().to_string())
}

/// Извлекает OpenGraph-превью для первого URL в сообщении.
async fn embed_preview(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(message_id): Path<i64>,
) -> AppResult<Json<Option<EmbedPreview>>> {
    let mut conn = state.pool.get().await?;
    let (_, _, access) = locate(&mut conn, message_id, user_id).await?;
    let server_id = access.server_id;
    let channel_id: i64 = messages::table
        .find(message_id)
        .select(messages::channel_id)
        .first(&mut conn)
        .await
        .optional()?
        .ok_or_else(|| AppError::not_found("message"))?;
    if let Some(server_id) = server_id {
        permissions::require_channel_permissions(
            &mut conn, server_id, user_id, channel_id,
            permissions::VIEW_CHANNEL | permissions::READ_MESSAGE_HISTORY,
        )
        .await?;
    }
    let content: String = messages::table
        .find(message_id)
        .select(messages::content)
        .first(&mut conn)
        .await
        .optional()?
        .ok_or_else(|| AppError::not_found("message"))?;
    let urls = extract_urls(&content);
    if urls.is_empty() {
        return Ok(Json(None));
    }
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    // Проверяем кэш (без удержания блокировки по await).
    let mut cached = None;
    {
        let cache = EMBED_CACHE.read().map_err(|_| AppError::bad_request("cache lock"))?;
        for url in &urls {
            if let Some((prev, ts)) = cache.get(url)
                && now_ms - ts < EMBED_TTL_MS {
                    cached = Some(prev.clone());
                    break;
                }
        }
    }
    if let Some(cached) = cached {
        return Ok(Json(Some(cached)));
    }
    // Загружаем первое невалидное/отсутствующее превью.
    for url in &urls {
        if let Some(embed) = fetch_embed(url).await {
            // Обновляем кэш.
            {
                let mut cache = EMBED_CACHE.write().map_err(|_| AppError::bad_request("cache lock"))?;
                cache.insert(url.clone(), (embed.clone(), now_ms));
            }
            return Ok(Json(Some(embed)));
        }
    }
    Ok(Json(None))
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
