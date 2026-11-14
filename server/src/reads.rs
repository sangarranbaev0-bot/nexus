//! Непрочитанные: SQL для счётчиков и отметка «прочитано».

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use diesel::sql_types::{BigInt, Int4, Nullable, Text, Timestamptz};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    events::{Notice, notify},
    permissions::{self, VIEW_CHANNEL},
    protocol::{Channel, ChannelKind, MarkReadRequest, NotifyLevel, NotifySetting, Role, Server},
    servers::{channel_access, role_in},
};

/// Чужие сообщения канала `c` после отметки прочтения из `rs` и после вступления `mem`.
/// Считаем максимум до 100: больше клиент всё равно покажет как «99+».
/// Ожидает в запросе алиасы `c`, `rs`, `mem` и пользователя в параметре `$1`.
const UNREAD: &str = "(SELECT count(*) FROM (
    SELECT 1 FROM messages m
    WHERE m.channel_id = c.id
      AND m.author_id <> $1
      AND m.system_kind IS NULL
      AND m.id > COALESCE(rs.last_read_message_id, 0)
      AND m.created_at > mem.joined_at
    LIMIT 100) unread)::int";

#[derive(diesel::QueryableByName)]
struct ChannelRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
    #[diesel(sql_type = BigInt)]
    server_id: i64,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Int4)]
    position: i32,
    #[diesel(sql_type = Text)]
    kind: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<BigInt>)]
    category_id: Option<i64>,
    #[diesel(sql_type = Int4)]
    unread_count: i32,
}

pub async fn channels_with_unread(
    conn: &mut AsyncPgConnection,
    user_id: i64,
    server_id: i64,
) -> AppResult<Vec<Channel>> {
    let rows: Vec<ChannelRow> = diesel::sql_query(format!(
        "SELECT c.id, c.server_id, c.name, c.position, c.kind, c.category_id, {UNREAD} AS unread_count
         FROM channels c
         JOIN members mem ON mem.server_id = c.server_id AND mem.user_id = $1
         LEFT JOIN read_states rs ON rs.channel_id = c.id AND rs.user_id = $1
         WHERE c.server_id = $2
         ORDER BY c.position, c.id"
    ))
    .bind::<BigInt, _>(user_id)
    .bind::<BigInt, _>(server_id)
    .load(conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| Channel {
            id: r.id,
            server_id: r.server_id,
            name: r.name,
            position: r.position,
            kind: ChannelKind::parse(&r.kind),
            category_id: r.category_id,
            unread_count: r.unread_count,
        })
        .collect())
}

#[derive(diesel::QueryableByName)]
struct ServerRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = BigInt)]
    owner_id: i64,
    #[diesel(sql_type = Text)]
    role: String,
    #[diesel(sql_type = Int4)]
    unread_count: i32,
    #[diesel(sql_type = Nullable<BigInt>)]
    system_channel_id: Option<i64>,
}

pub async fn servers_with_unread(
    conn: &mut AsyncPgConnection,
    user_id: i64,
) -> AppResult<Vec<Server>> {
    let rows: Vec<ServerRow> = diesel::sql_query(format!(
        "SELECT s.id, s.name, s.owner_id, mem.role, s.system_channel_id,
                COALESCE((SELECT sum({UNREAD})
                          FROM channels c
                          LEFT JOIN read_states rs ON rs.channel_id = c.id AND rs.user_id = $1
                          WHERE c.server_id = s.id), 0)::int AS unread_count
         FROM servers s
         JOIN members mem ON mem.server_id = s.id AND mem.user_id = $1
         ORDER BY mem.joined_at"
    ))
    .bind::<BigInt, _>(user_id)
    .load(conn)
    .await?;
    let mut servers = Vec::with_capacity(rows.len());
    for r in rows {
        let Some(my_role) = Role::parse(&r.role) else {
            continue;
        };
        let my_permissions = permissions::member_perms(conn, r.id, user_id).await?.bits;
        servers.push(Server {
            id: r.id,
            name: r.name,
            owner_id: r.owner_id,
            my_role,
            my_permissions,
            unread_count: r.unread_count,
            system_channel_id: r.system_channel_id,
        });
    }
    Ok(servers)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/channels/{id}/read", post(mark_read))
        .route("/api/me/notify-settings", get(notify_list).put(notify_set))
}

#[derive(diesel::QueryableByName)]
struct NotifyRow {
    #[diesel(sql_type = Nullable<BigInt>)]
    server_id: Option<i64>,
    #[diesel(sql_type = Nullable<BigInt>)]
    channel_id: Option<i64>,
    #[diesel(sql_type = Text)]
    level: String,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    muted_until: Option<chrono::DateTime<chrono::Utc>>,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    suppress_everyone: bool,
}

/// Все настройки уведомлений пользователя (для серверов и каналов).
async fn notify_list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> AppResult<Json<Vec<NotifySetting>>> {
    let mut conn = state.pool.get().await?;
    let rows: Vec<NotifyRow> = diesel::sql_query(
        "SELECT server_id, channel_id, level, muted_until, suppress_everyone
         FROM notify_settings WHERE user_id = $1",
    )
    .bind::<BigInt, _>(user_id)
    .load(&mut conn)
    .await?;
    Ok(Json(
        rows.into_iter()
            .filter_map(|r| {
                let level = NotifyLevel::parse(&r.level)?;
                Some(NotifySetting {
                    server_id: r.server_id,
                    channel_id: r.channel_id,
                    level,
                    muted_until: r.muted_until.map(|t| t.timestamp_millis()),
                    suppress_everyone: r.suppress_everyone,
                })
            })
            .collect(),
    ))
}

/// Ставит настройку. Чужой сервер или невидимый канал — 404.
async fn notify_set(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<NotifySetting>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let (col, target) = match (req.server_id, req.channel_id) {
        (Some(server_id), None) => {
            role_in(&mut conn, server_id, user_id).await?;
            ("server_id", server_id)
        }
        (None, Some(channel_id)) => {
            channel_access(&mut conn, channel_id, user_id).await?;
            ("channel_id", channel_id)
        }
        _ => {
            return Err(AppError::bad_request(
                "set exactly one of serverId and channelId",
            ));
        }
    };
    diesel::sql_query(format!(
        "DELETE FROM notify_settings WHERE user_id = $1 AND {col} = $2"
    ))
    .bind::<BigInt, _>(user_id)
    .bind::<BigInt, _>(target)
    .execute(&mut conn)
    .await?;
    diesel::sql_query(format!(
        "INSERT INTO notify_settings (user_id, {col}, level, muted_until, suppress_everyone)
         VALUES ($1, $2, $3, $4, $5)"
    ))
    .bind::<BigInt, _>(user_id)
    .bind::<BigInt, _>(target)
    .bind::<Text, _>(req.level.as_str())
    .bind::<Nullable<Timestamptz>, _>(
        req.muted_until
            .and_then(chrono::DateTime::from_timestamp_millis),
    )
    .bind::<diesel::sql_types::Bool, _>(req.suppress_everyone)
    .execute(&mut conn)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Отметка только растёт: запоздавший запрос со старым id ничего не откатит.
async fn mark_read(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
    Json(req): Json<MarkReadRequest>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let access = channel_access(&mut conn, channel_id, user_id).await?;
    let server_id = access.server_id;
    if let Some(server_id) = access.server_id {
        permissions::require_channel_permission(
            &mut conn,
            server_id,
            user_id,
            channel_id,
            VIEW_CHANNEL,
        )
        .await?;
    }
    diesel::sql_query(
        "INSERT INTO read_states (user_id, channel_id, last_read_message_id) VALUES ($1, $2, $3)
         ON CONFLICT (user_id, channel_id) DO UPDATE
         SET last_read_message_id = GREATEST(read_states.last_read_message_id, EXCLUDED.last_read_message_id)",
    )
    .bind::<BigInt, _>(user_id)
    .bind::<BigInt, _>(channel_id)
    .bind::<BigInt, _>(req.message_id)
    .execute(&mut conn)
    .await?;
    notify(
        &mut conn,
        &Notice::ChannelRead {
            user_id,
            server_id,
            channel_id,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
