use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
};
use diesel_async::RunQueryDsl;
use diesel::sql_query;

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    permissions,
    protocol::{AutoModRule, CreateAutoModRuleRequest},
    servers::role_in,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/servers/{id}/automod",
            get(list_rules).post(create_rule),
        )
        .route("/api/servers/{id}/automod/{rule_id}", delete(delete_rule))
}

async fn list_rules(
    State(_state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
) -> AppResult<Json<Vec<AutoModRule>>> {
    let mut conn = _state.pool.get().await?;
    role_in(&mut conn, server_id, user_id).await?;
    #[derive(diesel::QueryableByName)]
    struct R {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        id: i64,
        #[diesel(sql_type = diesel::sql_types::Text)]
        pattern: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        action: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        note: String,
    }
    let rows: Vec<R> = sql_query(
        "SELECT id, pattern, action, COALESCE(note,'') AS note FROM automod_rules WHERE server_id=$1 ORDER BY id",
    )
    .bind::<diesel::sql_types::BigInt, _>(server_id)
    .load(&mut conn)
    .await?;
    Ok(Json(rows.into_iter().map(|r| AutoModRule {
        id: r.id,
        pattern: r.pattern,
        action: r.action,
        note: r.note,
    }).collect()))
}

async fn create_rule(
    State(_state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
    Json(req): Json<CreateAutoModRuleRequest>,
) -> AppResult<(StatusCode, Json<AutoModRule>)> {
    let mut conn = _state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, permissions::MANAGE_ROLES).await?;

    if !["delete", "warn"].contains(&req.action.as_str()) {
        return Err(AppError::bad_request("action must be delete or warn"));
    }
    if req.pattern.len() < 2 || req.pattern.len() > 100 {
        return Err(AppError::bad_request("pattern must be 2-100 chars"));
    }

    #[derive(diesel::QueryableByName)]
    struct IdR {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        id: i64,
    }
    let id: i64 = sql_query(
        "INSERT INTO automod_rules (server_id, pattern, action, note) VALUES ($1,$2,$3,$4) RETURNING id",
    )
    .bind::<diesel::sql_types::BigInt, _>(server_id)
    .bind::<diesel::sql_types::Text, _>(&req.pattern)
    .bind::<diesel::sql_types::Text, _>(&req.action)
    .bind::<diesel::sql_types::Text, _>(req.note.as_deref().unwrap_or(""))
    .load(&mut conn)
    .await?
    .into_iter()
    .next()
    .map(|r: IdR| r.id)
    .ok_or_else(|| AppError::bad_request("create failed"))?;

    Ok((StatusCode::CREATED, Json(AutoModRule {
        id,
        pattern: req.pattern,
        action: req.action,
        note: req.note.unwrap_or_default(),
    })))
}

async fn delete_rule(
    State(_state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, rule_id)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    let mut conn = _state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, permissions::MANAGE_ROLES).await?;

    let deleted = sql_query("DELETE FROM automod_rules WHERE id=$1 AND server_id=$2")
        .bind::<diesel::sql_types::BigInt, _>(rule_id)
        .bind::<diesel::sql_types::BigInt, _>(server_id)
        .execute(&mut conn)
        .await?;

    if deleted == 0 {
        return Err(AppError::not_found("rule"));
    }
    Ok(StatusCode::NO_CONTENT)
}
