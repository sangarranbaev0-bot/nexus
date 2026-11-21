//! Центр уведомлений: упоминания, ответы, слова-триггеры и @everyone.
//!
//! При создании сообщения `dispatch` решает, кому показать уведомление, и вставляет записи.
//! Учитываются личные настройки (уровень, mute, подавление @everyone), блокировки и видимость канала.

use std::collections::{HashMap, HashSet};

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use diesel::{
    prelude::*,
    sql_types::{Array, BigInt, Bool, Int4, Nullable, Text, Timestamptz},
};
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use serde::Deserialize;

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    events::{Notice, notify},
    permissions::{self, MENTION_EVERYONE, VIEW_CHANNEL},
    protocol::{AddNotifyWord, Notification, NotificationKind, NotifyLevel, NotifyWord},
    schema::{dm_participants, messages, notification_words, notifications, users},
    servers::ChannelAccess,
};

const MAX_WORDS: usize = 20;
const PREVIEW_LEN: i32 = 200;
const LIST_LIMIT: i64 = 50;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/me/notifications", get(list))
        .route("/api/me/notifications/read", post(mark_read))
        .route("/api/me/notification-words", get(list_words).post(add_word))
        .route("/api/me/notification-words/{word}", delete(remove_word))
}

// ---- Генерация при создании сообщения -------------------------------------

#[derive(Clone, Copy)]
struct Setting {
    level: NotifyLevel,
    muted: bool,
    suppress_everyone: bool,
}

#[derive(diesel::QueryableByName)]
struct SettingRow {
    #[diesel(sql_type = BigInt)]
    user_id: i64,
    #[diesel(sql_type = Nullable<BigInt>)]
    channel_id: Option<i64>,
    #[diesel(sql_type = Text)]
    level: String,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    muted_until: Option<chrono::DateTime<chrono::Utc>>,
    #[diesel(sql_type = Bool)]
    suppress_everyone: bool,
}

/// Находит @упоминания по именам и @everyone/@here в тексте.
fn mentioned_usernames(content: &str) -> (HashSet<String>, bool) {
    let mut names = HashSet::new();
    let mut everyone = false;
    for raw in content.split_whitespace() {
        let Some(word) = raw.strip_prefix('@') else {
            continue;
        };
        // Допустимые символы имени — как в клиентском Markdown.
        let name: String = word
            .chars()
            .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-'))
            .collect();
        if name.is_empty() {
            continue;
        }
        match name.to_lowercase().as_str() {
            "everyone" | "here" => everyone = true,
            _ => {
                names.insert(name.to_lowercase());
            }
        }
    }
    (names, everyone)
}

/// Встречается ли слово в тексте (с границами слова), с учётом регистра.
fn contains_word(content: &str, word: &str, case_sensitive: bool) -> bool {
    let (haystack, needle) = if case_sensitive {
        (content.to_string(), word.to_string())
    } else {
        (content.to_lowercase(), word.to_lowercase())
    };
    if needle.is_empty() {
        return false;
    }
    let bytes = haystack.as_bytes();
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(&needle) {
        let at = start + pos;
        let end = at + needle.len();
        let before_ok = at == 0 || !is_word_byte(bytes[at - 1]);
        let after_ok = end == bytes.len() || !is_word_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        start = at + 1;
        if start >= haystack.len() {
            break;
        }
    }
    false
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[derive(diesel::QueryableByName)]
struct WordRow {
    #[diesel(sql_type = BigInt)]
    user_id: i64,
    #[diesel(sql_type = Text)]
    word: String,
    #[diesel(sql_type = Bool)]
    case_sensitive: bool,
}

#[derive(Debug)]
struct Member {
    id: i64,
    username: String,
}

#[derive(diesel::QueryableByName)]
struct UserRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
    #[diesel(sql_type = Text)]
    username: String,
}

/// Вызывается после создания сообщения. Ошибки не должны мешать сохранению сообщения.
pub async fn dispatch(
    conn: &mut AsyncPgConnection,
    access: &ChannelAccess,
    channel_id: i64,
    message_id: i64,
    author_id: i64,
    content: &str,
    reply_to_id: Option<i64>,
) -> AppResult<()> {
    if content.is_empty() {
        return Ok(());
    }
    let (names, everyone) = mentioned_usernames(content);

    // Получатель → причина. Один человек получает одно уведомление, приоритет:
    // reply > mention > everyone > word.
    let mut targets: HashMap<i64, NotificationKind> = HashMap::new();

    if let Some(reply_id) = reply_to_id {
        let reply_author: Option<i64> = messages::table
            .find(reply_id)
            .select(messages::author_id)
            .first(conn)
            .await
            .optional()?;
        if let Some(author) = reply_author.filter(|a| *a != author_id) {
            targets.insert(author, NotificationKind::Reply);
        }
    }

    // Кто вообще состоит в канале (и видит его, если это сервер).
    let members = channel_members(conn, access, channel_id, author_id).await?;

    if !names.is_empty() {
        for m in &members {
            if names.contains(&m.username.to_lowercase()) {
                targets.entry(m.id).or_insert(NotificationKind::Mention);
            }
        }
    }

    if everyone && let Some(server_id) = access.server_id {
        // @everyone срабатывает, только если у автора есть право.
        let perms = permissions::member_perms(conn, server_id, author_id).await?;
        if permissions::has(perms.bits, MENTION_EVERYONE) {
            for m in &members {
                targets.entry(m.id).or_insert(NotificationKind::Everyone);
            }
        }
    }

    // Слова-триггеры.
    let words: Vec<WordRow> =
        diesel::sql_query("SELECT user_id, word, case_sensitive FROM notification_words")
            .load(conn)
            .await?;
    let member_ids: HashSet<i64> = members.iter().map(|m| m.id).collect();
    for w in words {
        if w.user_id == author_id || !member_ids.contains(&w.user_id) {
            continue;
        }
        if contains_word(content, &w.word, w.case_sensitive) {
            targets.entry(w.user_id).or_insert(NotificationKind::Word);
        }
    }

    if targets.is_empty() {
        return Ok(());
    }

    let ids: Vec<i64> = targets.keys().copied().collect();
    let settings = load_settings(conn, &ids, access.server_id, channel_id).await?;
    let blocked = blocked_with(conn, author_id, &ids).await?;

    let mut accepted: Vec<(i64, NotificationKind)> = Vec::new();
    for (user_id, kind) in targets {
        if blocked.contains(&user_id) {
            continue;
        }
        if should_notify(kind, settings.get(&user_id)) {
            accepted.push((user_id, kind));
        }
    }
    if accepted.is_empty() {
        return Ok(());
    }

    let inserted: Vec<(i64, i64)> = conn
        .transaction(async move |conn| -> AppResult<Vec<(i64, i64)>> {
            let mut rows = Vec::with_capacity(accepted.len());
            for (user_id, kind) in &accepted {
                let id: i64 = diesel::insert_into(notifications::table)
                    .values((
                        notifications::user_id.eq(user_id),
                        notifications::kind.eq(kind.as_str()),
                        notifications::server_id.eq(access.server_id),
                        notifications::channel_id.eq(channel_id),
                        notifications::message_id.eq(message_id),
                        notifications::actor_id.eq(author_id),
                    ))
                    .returning(notifications::id)
                    .get_result(conn)
                    .await?;
                rows.push((id, *user_id));
            }
            Ok(rows)
        })
        .await?;

    for (notification_id, user_id) in inserted {
        notify(
            conn,
            &Notice::NotificationCreated {
                user_id,
                notification_id,
            },
        )
        .await?;
    }
    Ok(())
}

/// Участники канала, кроме автора: для сервера — с правом просмотра, для ЛС — собеседники.
async fn channel_members(
    conn: &mut AsyncPgConnection,
    access: &ChannelAccess,
    channel_id: i64,
    author_id: i64,
) -> AppResult<Vec<Member>> {
    match access.server_id {
        Some(server_id) => {
            let rows: Vec<UserRow> = diesel::sql_query(
                "SELECT u.id, u.username
                 FROM members m JOIN users u ON u.id = m.user_id
                 WHERE m.server_id = $1 AND m.user_id <> $2",
            )
            .bind::<BigInt, _>(server_id)
            .bind::<BigInt, _>(author_id)
            .load(conn)
            .await?;
            let mut result = Vec::with_capacity(rows.len());
            for row in rows {
                let member = permissions::member_perms(conn, server_id, row.id).await?;
                let bits = permissions::channel_bits(conn, &member, channel_id).await?;
                if permissions::has(bits, VIEW_CHANNEL) {
                    result.push(Member {
                        id: row.id,
                        username: row.username,
                    });
                }
            }
            Ok(result)
        }
        None => {
            let rows: Vec<(i64, String)> = dm_participants::table
                .inner_join(users::table.on(users::id.eq(dm_participants::user_id)))
                .filter(dm_participants::channel_id.eq(channel_id))
                .filter(dm_participants::user_id.ne(author_id))
                .select((users::id, users::username))
                .load(conn)
                .await?;
            Ok(rows
                .into_iter()
                .map(|(id, username)| Member { id, username })
                .collect())
        }
    }
}

async fn load_settings(
    conn: &mut AsyncPgConnection,
    user_ids: &[i64],
    server_id: Option<i64>,
    channel_id: i64,
) -> AppResult<HashMap<i64, Setting>> {
    let rows: Vec<SettingRow> = diesel::sql_query(
        "SELECT user_id, channel_id, level, muted_until, suppress_everyone
         FROM notify_settings
         WHERE user_id = ANY($1) AND (channel_id = $2 OR server_id IS NOT DISTINCT FROM $3)",
    )
    .bind::<Array<BigInt>, _>(user_ids.to_vec())
    .bind::<BigInt, _>(channel_id)
    .bind::<Nullable<BigInt>, _>(server_id)
    .load(conn)
    .await?;

    let now = chrono::Utc::now();
    let mut result = HashMap::new();
    // Сначала серверные, затем канальные перекрывают их.
    for (channel_first, rows) in [(false, &rows), (true, &rows)] {
        for r in rows.iter() {
            if (r.channel_id == Some(channel_id)) != channel_first {
                continue;
            }
            let Some(level) = NotifyLevel::parse(&r.level) else {
                continue;
            };
            result.insert(
                r.user_id,
                Setting {
                    level,
                    muted: r.muted_until.is_some_and(|t| t > now),
                    suppress_everyone: r.suppress_everyone,
                },
            );
        }
    }
    #[derive(diesel::QueryableByName)]
    struct DndRow {
        #[diesel(sql_type = BigInt)]
        id: i64,
    }
    let dnd: Vec<DndRow> =
        diesel::sql_query("SELECT id FROM users WHERE id = ANY($1) AND dnd = TRUE")
            .bind::<Array<BigInt>, _>(user_ids.to_vec())
            .load(conn)
            .await?;
    for r in dnd {
        let entry = result.entry(r.id).or_insert(Setting {
            level: NotifyLevel::None,
            muted: false,
            suppress_everyone: false,
        });
        entry.muted = true;
    }
    Ok(result)
}

/// Блокировки между автором и получателями (в любую сторону).
async fn blocked_with(
    conn: &mut AsyncPgConnection,
    author_id: i64,
    user_ids: &[i64],
) -> AppResult<HashSet<i64>> {
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = BigInt)]
        user_id: i64,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT CASE WHEN b.user_id = $1 THEN b.blocked_id ELSE b.user_id END AS user_id
         FROM blocks b
         WHERE (b.user_id = $1 AND b.blocked_id = ANY($2))
            OR (b.blocked_id = $1 AND b.user_id = ANY($2))",
    )
    .bind::<BigInt, _>(author_id)
    .bind::<Array<BigInt>, _>(user_ids.to_vec())
    .load(conn)
    .await?;
    Ok(rows.into_iter().map(|r| r.user_id).collect())
}

fn should_notify(kind: NotificationKind, setting: Option<&Setting>) -> bool {
    let Some(setting) = setting else {
        return true;
    };
    if setting.muted {
        return false;
    }
    if matches!(kind, NotificationKind::Everyone) && setting.suppress_everyone {
        return false;
    }
    match setting.level {
        NotifyLevel::All => true,
        NotifyLevel::Mentions => matches!(
            kind,
            NotificationKind::Mention | NotificationKind::Reply | NotificationKind::Word
        ),
        NotifyLevel::None => false,
    }
}

// ---- API центра уведомлений ----------------------------------------------

#[derive(diesel::QueryableByName)]
struct NotifyRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
    #[diesel(sql_type = Text)]
    kind: String,
    #[diesel(sql_type = Nullable<BigInt>)]
    server_id: Option<i64>,
    #[diesel(sql_type = BigInt)]
    channel_id: i64,
    #[diesel(sql_type = BigInt)]
    message_id: i64,
    #[diesel(sql_type = BigInt)]
    actor_id: i64,
    #[diesel(sql_type = Text)]
    actor_username: String,
    #[diesel(sql_type = Text)]
    preview: String,
    #[diesel(sql_type = Timestamptz)]
    created_at: chrono::DateTime<chrono::Utc>,
    #[diesel(sql_type = Bool)]
    read: bool,
    #[diesel(sql_type = Nullable<Text>)]
    server_name: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    channel_name: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    dm_username: Option<String>,
}

const SELECT_ROWS: &str =
    "SELECT n.id, n.kind, n.server_id, n.channel_id, n.message_id, n.actor_id,
        u.username AS actor_username,
        left(m.content, $2) AS preview,
        n.created_at,
        (n.read_at IS NOT NULL) AS read,
        s.name AS server_name,
        c.name AS channel_name,
        (SELECT du.username FROM dm_participants dp
           JOIN users du ON du.id = dp.user_id
           WHERE dp.channel_id = n.channel_id AND dp.user_id <> n.user_id
           ORDER BY du.username
           LIMIT 1) AS dm_username
 FROM notifications n
 JOIN users u ON u.id = n.actor_id
 JOIN messages m ON m.id = n.message_id
 LEFT JOIN servers s ON s.id = n.server_id
 JOIN channels c ON c.id = n.channel_id";

fn to_notification(r: NotifyRow) -> Option<Notification> {
    let kind = NotificationKind::parse(&r.kind)?;
    let location = r
        .server_name
        .or_else(|| r.channel_name.filter(|n| !n.is_empty()))
        .or(r.dm_username)
        .unwrap_or_default();
    Some(Notification {
        id: r.id,
        kind,
        server_id: r.server_id,
        channel_id: r.channel_id,
        message_id: r.message_id,
        actor_id: r.actor_id,
        actor_username: r.actor_username,
        preview: r.preview,
        created_at: r.created_at.timestamp_millis(),
        read: r.read,
        location,
    })
}

#[derive(Deserialize, Default)]
struct ListQuery {
    #[serde(default)]
    unread: bool,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<Notification>>> {
    let mut conn = state.pool.get().await?;
    let sql = format!(
        "{SELECT_ROWS}
         WHERE n.user_id = $1 AND ($4 = false OR n.read_at IS NULL)
         ORDER BY n.created_at DESC
         LIMIT $3"
    );
    let rows: Vec<NotifyRow> = diesel::sql_query(sql)
        .bind::<BigInt, _>(user_id)
        .bind::<Int4, _>(PREVIEW_LEN)
        .bind::<BigInt, _>(LIST_LIMIT)
        .bind::<Bool, _>(q.unread)
        .load(&mut conn)
        .await?;
    Ok(Json(rows.into_iter().filter_map(to_notification).collect()))
}

/// Загружает одно уведомление для realtime-события.
pub async fn load_one(
    conn: &mut AsyncPgConnection,
    user_id: i64,
    notification_id: i64,
) -> AppResult<Option<Notification>> {
    let sql = format!("{SELECT_ROWS} WHERE n.user_id = $1 AND n.id = $3");
    let rows: Vec<NotifyRow> = diesel::sql_query(sql)
        .bind::<BigInt, _>(user_id)
        .bind::<Int4, _>(PREVIEW_LEN)
        .bind::<BigInt, _>(notification_id)
        .load(conn)
        .await?;
    Ok(rows.into_iter().next().and_then(to_notification))
}

#[derive(Deserialize)]
struct ReadRequest {
    /// `null`/отсутствует — отметить все.
    #[serde(default)]
    id: Option<i64>,
}

async fn mark_read(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<ReadRequest>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    match req.id {
        Some(id) => {
            diesel::sql_query(
                "UPDATE notifications SET read_at = now()
                 WHERE id = $1 AND user_id = $2 AND read_at IS NULL",
            )
            .bind::<BigInt, _>(id)
            .bind::<BigInt, _>(user_id)
            .execute(&mut conn)
            .await?;
        }
        None => {
            diesel::sql_query(
                "UPDATE notifications SET read_at = now()
                 WHERE user_id = $1 AND read_at IS NULL",
            )
            .bind::<BigInt, _>(user_id)
            .execute(&mut conn)
            .await?;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---- Слова-триггеры -------------------------------------------------------

async fn list_words(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> AppResult<Json<Vec<NotifyWord>>> {
    let mut conn = state.pool.get().await?;
    let rows: Vec<(String, bool)> = notification_words::table
        .filter(notification_words::user_id.eq(user_id))
        .select((notification_words::word, notification_words::case_sensitive))
        .order(notification_words::created_at.asc())
        .load(&mut conn)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(word, case_sensitive)| NotifyWord {
                word,
                case_sensitive,
            })
            .collect(),
    ))
}

async fn add_word(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<AddNotifyWord>,
) -> AppResult<StatusCode> {
    let word = req.word.trim().to_string();
    if !(2..=32).contains(&word.chars().count()) {
        return Err(AppError::bad_request("word must be 2-32 characters"));
    }
    let mut conn = state.pool.get().await?;
    let count: i64 = notification_words::table
        .filter(notification_words::user_id.eq(user_id))
        .count()
        .get_result(&mut conn)
        .await?;
    if count >= MAX_WORDS as i64 {
        return Err(AppError::bad_request("too many notification words"));
    }
    diesel::insert_into(notification_words::table)
        .values((
            notification_words::user_id.eq(user_id),
            notification_words::word.eq(&word),
            notification_words::case_sensitive.eq(req.case_sensitive),
        ))
        .on_conflict((notification_words::user_id, notification_words::word))
        .do_update()
        .set(notification_words::case_sensitive.eq(req.case_sensitive))
        .execute(&mut conn)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_word(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(word): Path<String>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    diesel::delete(
        notification_words::table
            .filter(notification_words::user_id.eq(user_id))
            .filter(notification_words::word.eq(&word)),
    )
    .execute(&mut conn)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
