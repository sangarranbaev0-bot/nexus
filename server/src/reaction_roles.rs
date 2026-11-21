use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    events::{Notice, notify},
    permissions::MANAGE_ROLES,
    protocol::{CreateReactionRoleRequest, ReactionRole},
    schema::{member_roles, reaction_roles, roles},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/servers/{id}/reaction-roles", get(list_server))
        .route(
            "/api/servers/{id}/messages/{message_id}/reaction-roles",
            post(create),
        )
        .route(
            "/api/servers/{id}/reaction-roles/{message_id}/{emoji}/{role_id}",
            delete(remove),
        )
        .route("/api/messages/{id}/reaction-roles", get(list_message))
}

#[derive(QueryableByName)]
struct Row {
    #[diesel(sql_type = diesel::sql_types::Int8)]
    message_id: i64,
    #[diesel(sql_type = diesel::sql_types::Text)]
    emoji: String,
    #[diesel(sql_type = diesel::sql_types::Int8)]
    role_id: i64,
    #[diesel(sql_type = diesel::sql_types::Int8)]
    server_id: i64,
}

const LIST_SQL: &str = "SELECT rr.message_id, rr.emoji, rr.role_id, c.server_id \
    FROM reaction_roles rr JOIN messages m ON m.id = rr.message_id \
    JOIN channels c ON c.id = m.channel_id WHERE c.server_id = $1 \
    ORDER BY rr.message_id, rr.emoji, rr.role_id";

async fn require_manage(
    conn: &mut diesel_async::AsyncPgConnection,
    server_id: i64,
    user_id: i64,
) -> AppResult<()> {
    crate::permissions::require_permission(conn, server_id, user_id, MANAGE_ROLES).await?;
    Ok(())
}

fn dto(r: Row) -> ReactionRole {
    ReactionRole {
        message_id: r.message_id,
        emoji: r.emoji,
        role_id: r.role_id,
        server_id: r.server_id,
    }
}

async fn list_server(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
) -> AppResult<Json<Vec<ReactionRole>>> {
    let mut conn = state.pool.get().await?;
    require_manage(&mut conn, server_id, user_id).await?;
    let rows: Vec<Row> = diesel::sql_query(LIST_SQL)
        .bind::<diesel::sql_types::BigInt, _>(server_id)
        .load(&mut conn)
        .await?;
    Ok(Json(rows.into_iter().map(dto).collect()))
}

async fn list_message(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(message_id): Path<i64>,
) -> AppResult<Json<Vec<ReactionRole>>> {
    let mut conn = state.pool.get().await?;
    let (_channel, _author, access) =
        crate::messages::locate(&mut conn, message_id, user_id).await?;
    let Some(server_id) = access.server_id else {
        return Ok(Json(Vec::new()));
    };
    let rows: Vec<Row> = diesel::sql_query(LIST_SQL)
        .bind::<diesel::sql_types::BigInt, _>(server_id)
        .load(&mut conn)
        .await?;
    Ok(Json(
        rows.into_iter()
            .filter(|r| r.message_id == message_id)
            .map(dto)
            .collect(),
    ))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, message_id)): Path<(i64, i64)>,
    Json(req): Json<CreateReactionRoleRequest>,
) -> AppResult<StatusCode> {
    let emoji = crate::reactions::validate(&req.emoji)?.to_string();
    let mut conn = state.pool.get().await?;
    require_manage(&mut conn, server_id, user_id).await?;
    let (role_server, is_default, position): (i64, bool, i32) = roles::table
        .find(req.role_id)
        .select((roles::server_id, roles::is_default, roles::position))
        .first(&mut conn)
        .await
        .optional()?
        .ok_or_else(|| AppError::bad_request("unknown role"))?;
    if role_server != server_id || is_default || position <= 0 || position >= 100 {
        return Err(AppError::bad_request("role cannot be granted by reaction"));
    }
    #[derive(QueryableByName)]
    struct ServerRow {
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::BigInt>)]
        server_id: Option<i64>,
    }
    let message_server: Option<i64> = diesel::sql_query(
        "SELECT c.server_id FROM messages m JOIN channels c ON c.id = m.channel_id WHERE m.id = $1",
    )
    .bind::<diesel::sql_types::BigInt, _>(message_id)
    .load(&mut conn)
    .await?
    .into_iter()
    .next()
    .and_then(|r: ServerRow| r.server_id);
    if message_server != Some(server_id) {
        return Err(AppError::bad_request("message is not on this server"));
    }
    diesel::insert_into(reaction_roles::table)
        .values((
            reaction_roles::message_id.eq(message_id),
            reaction_roles::emoji.eq(emoji),
            reaction_roles::role_id.eq(req.role_id),
        ))
        .on_conflict_do_nothing()
        .execute(&mut conn)
        .await?;
    Ok(StatusCode::CREATED)
}

async fn remove(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, message_id, emoji, role_id)): Path<(i64, i64, String, i64)>,
) -> AppResult<StatusCode> {
    let emoji = crate::reactions::validate(&emoji)?.to_string();
    let mut conn = state.pool.get().await?;
    require_manage(&mut conn, server_id, user_id).await?;
    diesel::delete(
        reaction_roles::table
            .filter(reaction_roles::message_id.eq(message_id))
            .filter(reaction_roles::emoji.eq(emoji))
            .filter(reaction_roles::role_id.eq(role_id)),
    )
    .execute(&mut conn)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Выдаёт/отзывает роли, привязанные к реакции.
pub async fn apply(
    conn: &mut diesel_async::AsyncPgConnection,
    server_id: i64,
    message_id: i64,
    emoji: &str,
    user_id: i64,
    add: bool,
) -> AppResult<bool> {
    let role_ids: Vec<i64> = reaction_roles::table
        .filter(reaction_roles::message_id.eq(message_id))
        .filter(reaction_roles::emoji.eq(emoji))
        .select(reaction_roles::role_id)
        .load(conn)
        .await?;
    if role_ids.is_empty() {
        return Ok(false);
    }
    for role_id in role_ids {
        if add {
            diesel::insert_into(member_roles::table)
                .values((
                    member_roles::server_id.eq(server_id),
                    member_roles::user_id.eq(user_id),
                    member_roles::role_id.eq(role_id),
                ))
                .on_conflict_do_nothing()
                .execute(conn)
                .await?;
        } else {
            diesel::delete(
                member_roles::table
                    .filter(member_roles::server_id.eq(server_id))
                    .filter(member_roles::user_id.eq(user_id))
                    .filter(member_roles::role_id.eq(role_id)),
            )
            .execute(conn)
            .await?;
        }
    }
    notify(conn, &Notice::RolesChanged { server_id }).await?;
    Ok(true)
}
