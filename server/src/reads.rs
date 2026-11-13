//! Непрочитанные: SQL для счётчиков и отметка «прочитано».

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::post,
};
use diesel::sql_types::{BigInt, Int4, Text};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    AppState,
    auth::AuthUser,
    error::AppResult,
    events::{Notice, notify},
    protocol::{Channel, ChannelKind, MarkReadRequest, Role, Server},
    servers::channel_access,
};

/// Чужие сообщения канала `c` после отметки прочтения из `rs` и после вступления `mem`.
/// Считаем максимум до 100: больше клиент всё равно покажет как «99+».
/// Ожидает в запросе алиасы `c`, `rs`, `mem` и пользователя в параметре `$1`.
const UNREAD: &str = "(SELECT count(*) FROM (
    SELECT 1 FROM messages m
    WHERE m.channel_id = c.id
      AND m.author_id <> $1
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
}

pub async fn servers_with_unread(
    conn: &mut AsyncPgConnection,
    user_id: i64,
) -> AppResult<Vec<Server>> {
    let rows: Vec<ServerRow> = diesel::sql_query(format!(
        "SELECT s.id, s.name, s.owner_id, mem.role,
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
    Ok(rows
        .into_iter()
        .filter_map(|r| {
            Some(Server {
                id: r.id,
                name: r.name,
                owner_id: r.owner_id,
                my_role: Role::parse(&r.role)?,
                unread_count: r.unread_count,
            })
        })
        .collect())
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/channels/{id}/read", post(mark_read))
}

/// Отметка только растёт: запоздавший запрос со старым id ничего не откатит.
async fn mark_read(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
    Json(req): Json<MarkReadRequest>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let server_id = channel_access(&mut conn, channel_id, user_id)
        .await?
        .server_id;
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
