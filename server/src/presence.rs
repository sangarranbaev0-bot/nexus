//! Присутствие: online / idle / offline, общее для всех экземпляров сервера.
//!
//! Каждое WebSocket-подключение — строка в `presence_connections` с пульсом.
//! online — есть живая строка без idle, idle — живые строки есть, но все idle, offline — живых нет.
//! Экземпляр, упавший без уборки, перестаёт обновлять пульс, и его строки убирает `sweep`.

use std::time::Duration;

use chrono::Utc;
use diesel::{
    prelude::*,
    sql_types::{BigInt, Text},
};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    AppState,
    error::AppResult,
    events::{Notice, notify},
    protocol::Presence,
    schema::presence_connections as pc,
};

pub const HEARTBEAT_EVERY: Duration = Duration::from_secs(30);
/// Три пропущенных пульса — подключение мёртвое.
pub const STALE_AFTER_SECS: i32 = 90;

/// SQL-выражение статуса пользователя `u.id`.
pub const STATUS_SQL: &str = "CASE
    WHEN EXISTS (SELECT 1 FROM presence_connections p WHERE p.user_id = u.id
                 AND p.heartbeat_at > now() - interval '90 seconds' AND NOT p.idle) THEN 'online'
    WHEN EXISTS (SELECT 1 FROM presence_connections p WHERE p.user_id = u.id
                 AND p.heartbeat_at > now() - interval '90 seconds') THEN 'idle'
    ELSE 'offline' END";

#[derive(diesel::QueryableByName)]
struct StatusRow {
    #[diesel(sql_type = Text)]
    status: String,
}

pub async fn status_of(conn: &mut AsyncPgConnection, user_id: i64) -> AppResult<Presence> {
    let row: StatusRow = diesel::sql_query(format!(
        "SELECT {STATUS_SQL} AS status FROM users u WHERE u.id = $1"
    ))
    .bind::<BigInt, _>(user_id)
    .get_result(conn)
    .await?;
    Ok(Presence::parse(&row.status))
}

/// Регистрирует подключение и возвращает id его строки.
pub async fn connect(conn: &mut AsyncPgConnection, user_id: i64) -> AppResult<i64> {
    let id = diesel::insert_into(pc::table)
        .values(pc::user_id.eq(user_id))
        .returning(pc::id)
        .get_result(conn)
        .await?;
    notify(conn, &Notice::PresenceChanged { user_id }).await?;
    Ok(id)
}

pub async fn heartbeat(conn: &mut AsyncPgConnection, row_id: i64) -> AppResult<()> {
    diesel::update(pc::table.find(row_id))
        .set(pc::heartbeat_at.eq(Utc::now()))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn set_idle(
    conn: &mut AsyncPgConnection,
    row_id: i64,
    user_id: i64,
    idle: bool,
) -> AppResult<()> {
    diesel::update(pc::table.find(row_id))
        .set((pc::idle.eq(idle), pc::heartbeat_at.eq(Utc::now())))
        .execute(conn)
        .await?;
    notify(conn, &Notice::PresenceChanged { user_id }).await
}

pub async fn disconnect(conn: &mut AsyncPgConnection, row_id: i64, user_id: i64) -> AppResult<()> {
    diesel::delete(pc::table.find(row_id)).execute(conn).await?;
    notify(conn, &Notice::PresenceChanged { user_id }).await
}

/// Убирает строки без пульса и оповещает об их владельцах. DELETE … RETURNING
/// гарантирует, что одну строку уберёт и объявит только один экземпляр.
pub async fn sweep(state: &AppState) -> AppResult<usize> {
    let mut conn = state.pool.get().await?;
    let cutoff = Utc::now() - chrono::Duration::seconds(STALE_AFTER_SECS.into());
    // Сначала голос: его строки ссылаются на подключения и иначе исчезли бы молча.
    crate::voice::drop_stale(&mut conn, cutoff).await?;
    let mut users: Vec<i64> = diesel::delete(pc::table.filter(pc::heartbeat_at.lt(cutoff)))
        .returning(pc::user_id)
        .get_results(&mut conn)
        .await?;
    users.sort_unstable();
    users.dedup();
    for user_id in &users {
        notify(&mut conn, &Notice::PresenceChanged { user_id: *user_id }).await?;
    }
    Ok(users.len())
}

pub fn spawn_sweeper(state: AppState) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(HEARTBEAT_EVERY);
        loop {
            tick.tick().await;
            if let Err(e) = sweep(&state).await {
                tracing::error!("presence sweep failed: {e:?}");
            }
        }
    });
}
