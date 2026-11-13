//! Личные сообщения: канал без сервера на двоих. Писать можно любому пользователю по имени.

use axum::{Json, Router, extract::State, routing::get};
use diesel::{
    prelude::*,
    sql_types::{BigInt, Int4, Nullable, Text},
};
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    presence::STATUS_SQL,
    protocol::{DmChannel, OpenDmRequest, Presence},
    schema::{channels, dm_participants, users},
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/dms", get(list).post(open))
}

#[derive(diesel::QueryableByName)]
struct DmRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
    #[diesel(sql_type = BigInt)]
    user_id: i64,
    #[diesel(sql_type = Text)]
    username: String,
    #[diesel(sql_type = Text)]
    status: String,
    #[diesel(sql_type = Int4)]
    unread_count: i32,
}

/// Диалоги пользователя, свежие сверху. `only` — вернуть один конкретный диалог.
async fn load(
    conn: &mut AsyncPgConnection,
    user_id: i64,
    only: Option<i64>,
) -> AppResult<Vec<DmChannel>> {
    let rows: Vec<DmRow> = diesel::sql_query(format!(
        "SELECT c.id, u.id AS user_id, u.username, {STATUS_SQL} AS status,
                (SELECT count(*) FROM (
                    SELECT 1 FROM messages m
                    WHERE m.channel_id = c.id AND m.author_id <> $1
                      AND m.id > COALESCE(rs.last_read_message_id, 0)
                    LIMIT 100) unread)::int AS unread_count
         FROM dm_participants me
         JOIN channels c ON c.id = me.channel_id
         JOIN dm_participants other ON other.channel_id = c.id AND other.user_id <> $1
         JOIN users u ON u.id = other.user_id
         LEFT JOIN read_states rs ON rs.channel_id = c.id AND rs.user_id = $1
         WHERE me.user_id = $1 AND ($2::bigint IS NULL OR c.id = $2)
         ORDER BY (SELECT max(m.id) FROM messages m WHERE m.channel_id = c.id) DESC NULLS LAST, c.id DESC"
    ))
    .bind::<BigInt, _>(user_id)
    .bind::<Nullable<BigInt>, _>(only)
    .load(conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| DmChannel {
            id: r.id,
            user_id: r.user_id,
            username: r.username,
            status: Presence::parse(&r.status),
            unread_count: r.unread_count,
        })
        .collect())
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> AppResult<Json<Vec<DmChannel>>> {
    let mut conn = state.pool.get().await?;
    Ok(Json(load(&mut conn, user_id, None).await?))
}

/// Открывает диалог с пользователем или возвращает уже существующий.
async fn open(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<OpenDmRequest>,
) -> AppResult<Json<DmChannel>> {
    let mut conn = state.pool.get().await?;
    let other: i64 = users::table
        .filter(users::username.eq(req.username.trim()))
        .select(users::id)
        .first(&mut conn)
        .await
        .optional()?
        .ok_or_else(|| AppError::not_found("user"))?;
    if other == user_id {
        return Err(AppError::bad_request("cannot message yourself"));
    }

    let key = dm_key(user_id, other);
    let channel_id = conn
        .transaction(async move |conn| -> AppResult<i64> {
            // ON CONFLICT: одновременное открытие с обеих сторон даёт один и тот же канал.
            diesel::insert_into(channels::table)
                .values((
                    channels::name.eq(""),
                    channels::position.eq(0),
                    channels::dm_key.eq(&key),
                ))
                .on_conflict(channels::dm_key)
                .do_nothing()
                .execute(conn)
                .await?;
            let id: i64 = channels::table
                .filter(channels::dm_key.eq(&key))
                .select(channels::id)
                .first(conn)
                .await?;
            diesel::insert_into(dm_participants::table)
                .values(&[
                    (
                        dm_participants::channel_id.eq(id),
                        dm_participants::user_id.eq(user_id),
                    ),
                    (
                        dm_participants::channel_id.eq(id),
                        dm_participants::user_id.eq(other),
                    ),
                ])
                .on_conflict_do_nothing()
                .execute(conn)
                .await?;
            Ok(id)
        })
        .await?;

    let dm = load(&mut conn, user_id, Some(channel_id))
        .await?
        .pop()
        .ok_or_else(|| AppError::not_found("channel"))?;
    Ok(Json(dm))
}

fn dm_key(a: i64, b: i64) -> String {
    format!("{}:{}", a.min(b), a.max(b))
}

#[cfg(test)]
mod tests {
    #[test]
    fn dm_key_is_symmetric() {
        assert_eq!(super::dm_key(7, 3), "3:7");
        assert_eq!(super::dm_key(3, 7), super::dm_key(7, 3));
    }
}
