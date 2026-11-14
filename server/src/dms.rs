//! Личные сообщения: канал без сервера на двоих или группа до 10 человек.
//! Писать можно любому пользователю по имени, если он не заблокирован.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
};
use diesel::{
    prelude::*,
    sql_types::{Array, BigInt, Bool, Int4, Nullable, Text},
};
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    events::{Notice, notify},
    presence::STATUS_SQL,
    protocol::{CreateGroupRequest, DmChannel, DmMember, NameRequest, OpenDmRequest, Presence},
    schema::{channels, dm_participants, users},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/dms", get(list).post(open))
        .route("/api/dms/group", post(create_group))
        .route("/api/dms/{id}", patch(rename_group))
        .route("/api/dms/{id}/members", post(add_member))
        .route("/api/dms/{id}/members/me", delete(leave_group))
}

const MAX_GROUP: usize = 10;

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
    #[diesel(sql_type = Bool)]
    is_group: bool,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Array<BigInt>)]
    member_ids: Vec<i64>,
    #[diesel(sql_type = Array<Text>)]
    member_names: Vec<String>,
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
                    LIMIT 100) unread)::int AS unread_count,
                c.dm_key LIKE 'g:%' AS is_group, c.name, o.ids AS member_ids, o.names AS member_names
         FROM dm_participants me
         JOIN channels c ON c.id = me.channel_id
         CROSS JOIN LATERAL (
             SELECT array_agg(p.user_id ORDER BY p.user_id) AS ids,
                    array_agg(uu.username ORDER BY p.user_id) AS names
             FROM dm_participants p JOIN users uu ON uu.id = p.user_id
             WHERE p.channel_id = c.id AND p.user_id <> $1) o
         JOIN users u ON u.id = o.ids[1]
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
            is_group: r.is_group,
            name: r.name,
            members: r
                .member_ids
                .into_iter()
                .zip(r.member_names)
                .map(|(user_id, username)| DmMember { user_id, username })
                .collect(),
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
    if crate::friends::blocked_between(&mut conn, user_id, other).await? {
        return Err(AppError::forbidden_with("blocked"));
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

async fn user_by_name(conn: &mut AsyncPgConnection, name: &str) -> AppResult<i64> {
    users::table
        .filter(users::username.eq(name.trim()))
        .select(users::id)
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| AppError::not_found("user"))
}

fn group_name(name: Option<String>) -> AppResult<String> {
    let name = name.unwrap_or_default().trim().to_string();
    if name.chars().count() > 100 {
        return Err(AppError::bad_request("group name is too long"));
    }
    Ok(name)
}

/// Группе — участникам: перечитать список диалогов.
async fn dms_changed(
    conn: &mut AsyncPgConnection,
    channel_id: i64,
    extra: &[i64],
) -> AppResult<()> {
    let mut user_ids: Vec<i64> = dm_participants::table
        .filter(dm_participants::channel_id.eq(channel_id))
        .select(dm_participants::user_id)
        .load(conn)
        .await?;
    user_ids.extend_from_slice(extra);
    notify(conn, &Notice::DmsChanged { user_ids }).await
}

/// Новая группа: вы и 2–9 человек.
async fn create_group(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<CreateGroupRequest>,
) -> AppResult<(StatusCode, Json<DmChannel>)> {
    let name = group_name(req.name)?;
    let mut conn = state.pool.get().await?;
    let mut ids = Vec::new();
    for username in &req.usernames {
        let id = user_by_name(&mut conn, username).await?;
        if id != user_id && !ids.contains(&id) {
            if crate::friends::blocked_between(&mut conn, user_id, id).await? {
                return Err(AppError::forbidden_with("blocked"));
            }
            ids.push(id);
        }
    }
    if ids.len() < 2 || ids.len() + 1 > MAX_GROUP {
        return Err(AppError::bad_request("a group needs 3-10 people"));
    }
    ids.push(user_id);
    let mut rnd = [0u8; 12];
    getrandom::fill(&mut rnd).expect("OS RNG available");
    let key = format!("g:{}", hex::encode(rnd));
    let channel_id = conn
        .transaction(async move |conn| -> AppResult<i64> {
            let id: i64 = diesel::insert_into(channels::table)
                .values((
                    channels::name.eq(&name),
                    channels::position.eq(0),
                    channels::dm_key.eq(&key),
                ))
                .returning(channels::id)
                .get_result(conn)
                .await?;
            let rows: Vec<_> = ids
                .iter()
                .map(|u| {
                    (
                        dm_participants::channel_id.eq(id),
                        dm_participants::user_id.eq(*u),
                    )
                })
                .collect();
            diesel::insert_into(dm_participants::table)
                .values(&rows)
                .execute(conn)
                .await?;
            Ok(id)
        })
        .await?;
    dms_changed(&mut conn, channel_id, &[]).await?;
    let dm = load(&mut conn, user_id, Some(channel_id))
        .await?
        .pop()
        .ok_or_else(|| AppError::not_found("channel"))?;
    Ok((StatusCode::CREATED, Json(dm)))
}

/// Только для групп, в которых вы состоите; иначе 404.
async fn require_group(
    conn: &mut AsyncPgConnection,
    channel_id: i64,
    user_id: i64,
) -> AppResult<()> {
    let is_group_member: bool = diesel::select(diesel::dsl::exists(
        channels::table
            .inner_join(dm_participants::table.on(dm_participants::channel_id.eq(channels::id)))
            .filter(channels::id.eq(channel_id))
            .filter(channels::dm_key.like("g:%"))
            .filter(dm_participants::user_id.eq(user_id)),
    ))
    .get_result(conn)
    .await?;
    if !is_group_member {
        return Err(AppError::not_found("channel"));
    }
    Ok(())
}

async fn rename_group(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
    Json(req): Json<NameRequest>,
) -> AppResult<StatusCode> {
    let name = group_name(Some(req.name))?;
    let mut conn = state.pool.get().await?;
    require_group(&mut conn, channel_id, user_id).await?;
    diesel::update(channels::table.find(channel_id))
        .set(channels::name.eq(name))
        .execute(&mut conn)
        .await?;
    dms_changed(&mut conn, channel_id, &[]).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn add_member(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
    Json(req): Json<OpenDmRequest>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    require_group(&mut conn, channel_id, user_id).await?;
    let other = user_by_name(&mut conn, &req.username).await?;
    if crate::friends::blocked_between(&mut conn, user_id, other).await? {
        return Err(AppError::forbidden_with("blocked"));
    }
    let count: i64 = dm_participants::table
        .filter(dm_participants::channel_id.eq(channel_id))
        .count()
        .get_result(&mut conn)
        .await?;
    if count as usize >= MAX_GROUP {
        return Err(AppError::bad_request("a group needs 3-10 people"));
    }
    diesel::insert_into(dm_participants::table)
        .values((
            dm_participants::channel_id.eq(channel_id),
            dm_participants::user_id.eq(other),
        ))
        .on_conflict_do_nothing()
        .execute(&mut conn)
        .await?;
    dms_changed(&mut conn, channel_id, &[]).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Выйти из группы. Последний вышедший — группа удаляется вместе с перепиской.
async fn leave_group(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    require_group(&mut conn, channel_id, user_id).await?;
    diesel::delete(dm_participants::table.find((channel_id, user_id)))
        .execute(&mut conn)
        .await?;
    let left: i64 = dm_participants::table
        .filter(dm_participants::channel_id.eq(channel_id))
        .count()
        .get_result(&mut conn)
        .await?;
    if left == 0 {
        diesel::delete(channels::table.find(channel_id))
            .execute(&mut conn)
            .await?;
    }
    dms_changed(&mut conn, channel_id, &[user_id]).await?;
    Ok(StatusCode::NO_CONTENT)
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
