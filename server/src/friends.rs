//! Друзья, заявки и блокировки. Дружба хранится двумя строками — по одной на каждую сторону.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
};
use diesel::{
    prelude::*,
    sql_types::{BigInt, Bool, Text},
};
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    events::{Notice, notify},
    presence::STATUS_SQL,
    protocol::{BlockedUser, Friend, FriendRequest, FriendRequests, Presence, SendFriendRequest},
    schema::{blocks, friend_requests, friends, users},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/friends", get(list))
        .route("/api/friends/{id}", delete(remove))
        .route("/api/friends/requests", get(requests).post(send_request))
        .route("/api/friends/requests/{id}", delete(cancel))
        .route("/api/friends/requests/{id}/accept", post(accept))
        .route("/api/blocks", get(blocked))
        .route("/api/blocks/{id}", put(block).delete(unblock))
}

#[derive(diesel::QueryableByName)]
struct Flag {
    #[diesel(sql_type = Bool)]
    flag: bool,
}

/// Заблокирован ли кто-то из пары в любую сторону.
pub async fn blocked_between(conn: &mut AsyncPgConnection, a: i64, b: i64) -> AppResult<bool> {
    let row: Flag = diesel::sql_query(
        "SELECT EXISTS (
            SELECT 1 FROM blocks
            WHERE (user_id = $1 AND blocked_id = $2)
               OR (user_id = $2 AND blocked_id = $1)
         ) AS flag",
    )
    .bind::<BigInt, _>(a)
    .bind::<BigInt, _>(b)
    .get_result(conn)
    .await?;
    Ok(row.flag)
}

/// Есть ли блокировка между участниками ЛС (в любую сторону).
pub async fn dm_has_block(
    conn: &mut AsyncPgConnection,
    channel_id: i64,
    user_id: i64,
) -> AppResult<bool> {
    let row: Flag = diesel::sql_query(
        "SELECT EXISTS (
            SELECT 1
            FROM dm_participants other
            JOIN blocks b ON ((b.user_id = $2 AND b.blocked_id = other.user_id)
                           OR (b.user_id = other.user_id AND b.blocked_id = $2))
            WHERE other.channel_id = $1 AND other.user_id <> $2
         ) AS flag",
    )
    .bind::<BigInt, _>(channel_id)
    .bind::<BigInt, _>(user_id)
    .get_result(conn)
    .await?;
    Ok(row.flag)
}

#[derive(diesel::QueryableByName)]
struct FriendRow {
    #[diesel(sql_type = BigInt)]
    user_id: i64,
    #[diesel(sql_type = Text)]
    username: String,
    #[diesel(sql_type = Text)]
    status: String,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> AppResult<Json<Vec<Friend>>> {
    let mut conn = state.pool.get().await?;
    let rows: Vec<FriendRow> = diesel::sql_query(format!(
        "SELECT f.friend_id AS user_id, u.username, {STATUS_SQL} AS status
         FROM friends f
         JOIN users u ON u.id = f.friend_id
         WHERE f.user_id = $1
         ORDER BY lower(u.username), u.id"
    ))
    .bind::<BigInt, _>(user_id)
    .load(&mut conn)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| Friend {
                user_id: r.user_id,
                username: r.username,
                status: Presence::parse(&r.status),
            })
            .collect(),
    ))
}

#[derive(diesel::QueryableByName)]
struct RequestRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
    #[diesel(sql_type = BigInt)]
    from_user_id: i64,
    #[diesel(sql_type = Text)]
    from_username: String,
    #[diesel(sql_type = BigInt)]
    to_user_id: i64,
    #[diesel(sql_type = Text)]
    to_username: String,
}

async fn requests(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> AppResult<Json<FriendRequests>> {
    let mut conn = state.pool.get().await?;
    let rows: Vec<RequestRow> = diesel::sql_query(
        "SELECT fr.id, fr.from_user_id, fu.username AS from_username,
                fr.to_user_id, tu.username AS to_username
         FROM friend_requests fr
         JOIN users fu ON fu.id = fr.from_user_id
         JOIN users tu ON tu.id = fr.to_user_id
         WHERE fr.from_user_id = $1 OR fr.to_user_id = $1
         ORDER BY fr.id DESC",
    )
    .bind::<BigInt, _>(user_id)
    .load(&mut conn)
    .await?;

    let mut incoming = Vec::new();
    let mut outgoing = Vec::new();
    for r in rows {
        let req = FriendRequest {
            id: r.id,
            from_user_id: r.from_user_id,
            from_username: r.from_username,
            to_user_id: r.to_user_id,
            to_username: r.to_username,
        };
        if req.to_user_id == user_id {
            incoming.push(req);
        } else {
            outgoing.push(req);
        }
    }
    Ok(Json(FriendRequests { incoming, outgoing }))
}

async fn send_request(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<SendFriendRequest>,
) -> AppResult<(StatusCode, Json<FriendRequest>)> {
    let name = req.username.trim();
    if name.is_empty() {
        return Err(AppError::bad_request("username is required"));
    }
    let mut conn = state.pool.get().await?;
    let target: Option<(i64, String)> = users::table
        .filter(users::username.eq(name))
        .select((users::id, users::username))
        .first(&mut conn)
        .await
        .optional()?;
    let (target_id, target_name) = target.ok_or_else(|| AppError::not_found("user"))?;
    if target_id == user_id {
        return Err(AppError::bad_request("cannot friend yourself"));
    }
    if blocked_between(&mut conn, user_id, target_id).await? {
        return Err(AppError::forbidden_with("blocked"));
    }

    let already: i64 = friends::table
        .filter(friends::user_id.eq(user_id))
        .filter(friends::friend_id.eq(target_id))
        .count()
        .get_result(&mut conn)
        .await?;
    if already > 0 {
        return Err(AppError::conflict("already friends"));
    }
    let existing: Option<i64> = friend_requests::table
        .filter(friend_requests::from_user_id.eq(user_id))
        .filter(friend_requests::to_user_id.eq(target_id))
        .select(friend_requests::id)
        .first(&mut conn)
        .await
        .optional()?;
    if existing.is_some() {
        return Err(AppError::conflict("request already sent"));
    }
    let reverse: Option<i64> = friend_requests::table
        .filter(friend_requests::from_user_id.eq(target_id))
        .filter(friend_requests::to_user_id.eq(user_id))
        .select(friend_requests::id)
        .first(&mut conn)
        .await
        .optional()?;
    if reverse.is_some() {
        return Err(AppError::conflict("incoming request exists"));
    }

    let my_name: String = users::table
        .find(user_id)
        .select(users::username)
        .first(&mut conn)
        .await?;
    let id: i64 = diesel::insert_into(friend_requests::table)
        .values((
            friend_requests::from_user_id.eq(user_id),
            friend_requests::to_user_id.eq(target_id),
        ))
        .returning(friend_requests::id)
        .get_result(&mut conn)
        .await?;

    notify(
        &mut conn,
        &Notice::FriendsChanged {
            user_ids: vec![user_id, target_id],
        },
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(FriendRequest {
            id,
            from_user_id: user_id,
            from_username: my_name,
            to_user_id: target_id,
            to_username: target_name,
        }),
    ))
}

async fn accept(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(request_id): Path<i64>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let row: Option<(i64, i64)> = friend_requests::table
        .find(request_id)
        .select((friend_requests::from_user_id, friend_requests::to_user_id))
        .first(&mut conn)
        .await
        .optional()?;
    let (from, to) = row.ok_or_else(|| AppError::not_found("request"))?;
    if to != user_id {
        return Err(AppError::not_found("request"));
    }
    if blocked_between(&mut conn, from, to).await? {
        return Err(AppError::forbidden_with("blocked"));
    }

    conn.transaction(async move |conn| -> AppResult<()> {
        diesel::insert_into(friends::table)
            .values(&[
                (friends::user_id.eq(user_id), friends::friend_id.eq(from)),
                (friends::user_id.eq(from), friends::friend_id.eq(user_id)),
            ])
            .on_conflict_do_nothing()
            .execute(conn)
            .await?;
        diesel::delete(
            friend_requests::table.filter(
                friend_requests::from_user_id
                    .eq(from)
                    .and(friend_requests::to_user_id.eq(user_id))
                    .or(friend_requests::from_user_id
                        .eq(user_id)
                        .and(friend_requests::to_user_id.eq(from))),
            ),
        )
        .execute(conn)
        .await?;
        Ok(())
    })
    .await?;

    notify(
        &mut conn,
        &Notice::FriendsChanged {
            user_ids: vec![user_id, from],
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn cancel(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(request_id): Path<i64>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let row: Option<(i64, i64)> = friend_requests::table
        .find(request_id)
        .select((friend_requests::from_user_id, friend_requests::to_user_id))
        .first(&mut conn)
        .await
        .optional()?;
    let (from, to) = row.ok_or_else(|| AppError::not_found("request"))?;
    if from != user_id && to != user_id {
        return Err(AppError::not_found("request"));
    }
    diesel::delete(friend_requests::table.find(request_id))
        .execute(&mut conn)
        .await?;
    notify(
        &mut conn,
        &Notice::FriendsChanged {
            user_ids: vec![from, to],
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(friend_id): Path<i64>,
) -> AppResult<StatusCode> {
    if friend_id == user_id {
        return Err(AppError::bad_request("cannot remove yourself"));
    }
    let mut conn = state.pool.get().await?;
    diesel::delete(
        friends::table.filter(
            friends::user_id
                .eq(user_id)
                .and(friends::friend_id.eq(friend_id))
                .or(friends::user_id
                    .eq(friend_id)
                    .and(friends::friend_id.eq(user_id))),
        ),
    )
    .execute(&mut conn)
    .await?;
    notify(
        &mut conn,
        &Notice::FriendsChanged {
            user_ids: vec![user_id, friend_id],
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn blocked(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> AppResult<Json<Vec<BlockedUser>>> {
    let mut conn = state.pool.get().await?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = BigInt)]
        user_id: i64,
        #[diesel(sql_type = Text)]
        username: String,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT b.blocked_id AS user_id, u.username
         FROM blocks b
         JOIN users u ON u.id = b.blocked_id
         WHERE b.user_id = $1
         ORDER BY lower(u.username), u.id",
    )
    .bind::<BigInt, _>(user_id)
    .load(&mut conn)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| BlockedUser {
                user_id: r.user_id,
                username: r.username,
            })
            .collect(),
    ))
}

async fn block(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(target_id): Path<i64>,
) -> AppResult<StatusCode> {
    if target_id == user_id {
        return Err(AppError::bad_request("cannot block yourself"));
    }
    let mut conn = state.pool.get().await?;
    let exists: Option<i64> = users::table
        .find(target_id)
        .select(users::id)
        .first(&mut conn)
        .await
        .optional()?;
    if exists.is_none() {
        return Err(AppError::not_found("user"));
    }

    conn.transaction(async move |conn| -> AppResult<()> {
        diesel::delete(
            friends::table.filter(
                friends::user_id
                    .eq(user_id)
                    .and(friends::friend_id.eq(target_id))
                    .or(friends::user_id
                        .eq(target_id)
                        .and(friends::friend_id.eq(user_id))),
            ),
        )
        .execute(conn)
        .await?;
        diesel::delete(
            friend_requests::table.filter(
                friend_requests::from_user_id
                    .eq(user_id)
                    .and(friend_requests::to_user_id.eq(target_id))
                    .or(friend_requests::from_user_id
                        .eq(target_id)
                        .and(friend_requests::to_user_id.eq(user_id))),
            ),
        )
        .execute(conn)
        .await?;
        diesel::insert_into(blocks::table)
            .values((
                blocks::user_id.eq(user_id),
                blocks::blocked_id.eq(target_id),
            ))
            .on_conflict_do_nothing()
            .execute(conn)
            .await?;
        Ok(())
    })
    .await?;

    notify(
        &mut conn,
        &Notice::FriendsChanged {
            user_ids: vec![user_id, target_id],
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn unblock(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(target_id): Path<i64>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    diesel::delete(
        blocks::table.filter(
            blocks::user_id
                .eq(user_id)
                .and(blocks::blocked_id.eq(target_id)),
        ),
    )
    .execute(&mut conn)
    .await?;
    notify(
        &mut conn,
        &Notice::FriendsChanged {
            user_ids: vec![user_id],
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
