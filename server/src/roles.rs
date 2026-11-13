//! Пользовательские роли, категории каналов, переопределения прав и журнал аудита.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch, put},
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    events::{Notice, notify},
    permissions::{
        self, MemberPerms, MANAGE_CHANNELS, MANAGE_ROLES, VIEW_AUDIT_LOG,
    },
    protocol::{
        AssignRolesRequest, AuditEntry, Category, CreateCategoryRequest, CreateRoleRequest,
        Overwrite, RoleInfo, UpdateCategoryRequest, UpdateChannelRequest, UpdateRoleRequest,
        UpsertOverwriteRequest,
    },
    schema::{categories, channel_overwrites, channels, member_roles, roles},
    servers::{channel_server, normalize_channel_name, role_in},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/servers/{id}/roles", get(list_roles).post(create_role))
        .route(
            "/api/servers/{id}/roles/{role_id}",
            patch(update_role).delete(delete_role),
        )
        .route(
            "/api/servers/{id}/members/{user_id}/roles",
            put(assign_roles),
        )
        .route(
            "/api/servers/{id}/categories",
            get(list_categories).post(create_category),
        )
        .route(
            "/api/servers/{id}/categories/{category_id}",
            patch(update_category).delete(delete_category),
        )
        .route("/api/channels/{id}", patch(update_channel))
        .route(
            "/api/channels/{id}/overwrites",
            get(list_overwrites).put(upsert_overwrite),
        )
        .route(
            "/api/channels/{id}/overwrites/{overwrite_id}",
            delete(delete_overwrite),
        )
        .route("/api/servers/{id}/audit", get(list_audit))
}

#[derive(Queryable)]
struct RoleRow {
    id: i64,
    server_id: i64,
    name: String,
    color: Option<i32>,
    permissions: i64,
    position: i32,
    hoist: bool,
    mentionable: bool,
    is_default: bool,
}

fn role_columns() -> (
    roles::id,
    roles::server_id,
    roles::name,
    roles::color,
    roles::permissions,
    roles::position,
    roles::hoist,
    roles::mentionable,
    roles::is_default,
) {
    (
        roles::id,
        roles::server_id,
        roles::name,
        roles::color,
        roles::permissions,
        roles::position,
        roles::hoist,
        roles::mentionable,
        roles::is_default,
    )
}

fn to_info(r: RoleRow) -> RoleInfo {
    RoleInfo {
        id: r.id,
        server_id: r.server_id,
        name: r.name,
        color: r.color,
        permissions: r.permissions,
        position: r.position,
        hoist: r.hoist,
        mentionable: r.mentionable,
        is_default: r.is_default,
    }
}

fn validate_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if !(1..=100).contains(&name.chars().count()) {
        return Err(AppError::bad_request("name must be 1-100 characters"));
    }
    Ok(name.to_string())
}

fn validate_color(color: i32) -> AppResult<Option<i32>> {
    if color == -1 {
        return Ok(None);
    }
    if (0..=0xFF_FF_FF).contains(&color) {
        Ok(Some(color))
    } else {
        Err(AppError::bad_request("color must be 0xRRGGBB or -1"))
    }
}

fn validate_permissions(bits: i64) -> AppResult<i64> {
    if bits < 0 || bits & !permissions::ALL != 0 {
        return Err(AppError::bad_request("unknown permission bits"));
    }
    Ok(bits)
}

fn validate_position(position: i32) -> AppResult<i32> {
    if (0..=1000).contains(&position) {
        Ok(position)
    } else {
        Err(AppError::bad_request("position must be 0-1000"))
    }
}

async fn load_role(
    conn: &mut AsyncPgConnection,
    server_id: i64,
    role_id: i64,
) -> AppResult<RoleRow> {
    roles::table
        .filter(roles::id.eq(role_id))
        .filter(roles::server_id.eq(server_id))
        .select(role_columns())
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| AppError::not_found("role"))
}

/// Старшинство участника: владелец выше всех, администратор — над пользовательскими
/// ролями, остальные — по самой высокой назначенной роли.
async fn actor_rank(
    conn: &mut AsyncPgConnection,
    server_id: i64,
    perms: &MemberPerms,
) -> AppResult<i32> {
    match perms.role {
        crate::protocol::Role::Owner => Ok(i32::MAX),
        crate::protocol::Role::Admin => Ok(i32::MAX - 1),
        crate::protocol::Role::Member => {
            let top: Option<i32> = roles::table
                .filter(roles::server_id.eq(server_id))
                .filter(roles::id.eq_any(&perms.role_ids))
                .select(diesel::dsl::max(roles::position))
                .first(conn)
                .await?;
            Ok(top.unwrap_or(0))
        }
    }
}

async fn audit(
    conn: &mut AsyncPgConnection,
    server_id: i64,
    actor_id: i64,
    action: &str,
    target: Option<&str>,
    metadata: Option<serde_json::Value>,
) -> AppResult<()> {
    use diesel::sql_types::{BigInt, Nullable, Text};
    let meta = metadata.map(|v| v.to_string());
    diesel::sql_query(
        "INSERT INTO audit_log (server_id, actor_id, action, target, metadata)
         VALUES ($1, $2, $3, $4, $5::jsonb)",
    )
    .bind::<BigInt, _>(server_id)
    .bind::<BigInt, _>(actor_id)
    .bind::<Text, _>(action)
    .bind::<Nullable<Text>, _>(target)
    .bind::<Nullable<Text>, _>(meta)
    .execute(conn)
    .await?;
    Ok(())
}

async fn list_roles(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
) -> AppResult<Json<Vec<RoleInfo>>> {
    let mut conn = state.pool.get().await?;
    permissions::member_perms(&mut conn, server_id, user_id).await?;
    let rows: Vec<RoleRow> = roles::table
        .filter(roles::server_id.eq(server_id))
        .order((roles::position.desc(), roles::id.desc()))
        .select(role_columns())
        .load(&mut conn)
        .await?;
    Ok(Json(rows.into_iter().map(to_info).collect()))
}

async fn create_role(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
    Json(req): Json<CreateRoleRequest>,
) -> AppResult<(StatusCode, Json<RoleInfo>)> {
    let name = validate_name(&req.name)?;
    let color = req.color.map(validate_color).transpose()?.flatten();
    let permissions_bits = validate_permissions(req.permissions.unwrap_or(0))?;
    let mut conn = state.pool.get().await?;
    let actor = permissions::require_permission(&mut conn, server_id, user_id, MANAGE_ROLES).await?;
    let rank = actor_rank(&mut conn, server_id, &actor).await?;

    let max_position: Option<i32> = roles::table
        .filter(roles::server_id.eq(server_id))
        .select(diesel::dsl::max(roles::position))
        .first(&mut conn)
        .await?;
    let next = max_position.map_or(1, |p| p + 1);
    // Участник без admin/owner не может создать роль выше собственной.
    let position = if rank >= i32::MAX - 1 {
        next
    } else {
        let max_below: Option<i32> = roles::table
            .filter(roles::server_id.eq(server_id))
            .filter(roles::position.lt(rank))
            .select(diesel::dsl::max(roles::position))
            .first(&mut conn)
            .await?;
        let p = max_below.map_or(1, |v| v + 1);
        if p >= rank {
            return Err(AppError::forbidden());
        }
        p
    };
    let position = validate_position(position)?;

    let id: i64 = diesel::insert_into(roles::table)
        .values((
            roles::server_id.eq(server_id),
            roles::name.eq(&name),
            roles::color.eq(color),
            roles::permissions.eq(permissions_bits),
            roles::position.eq(position),
            roles::hoist.eq(req.hoist.unwrap_or(false)),
            roles::mentionable.eq(req.mentionable.unwrap_or(false)),
        ))
        .returning(roles::id)
        .get_result(&mut conn)
        .await?;
    audit(
        &mut conn,
        server_id,
        user_id,
        "role.create",
        Some(&name),
        Some(serde_json::json!({ "roleId": id, "permissions": permissions_bits })),
    )
    .await?;
    notify(&mut conn, &Notice::RolesChanged { server_id }).await?;
    Ok((
        StatusCode::CREATED,
        Json(RoleInfo {
            id,
            server_id,
            name,
            color,
            permissions: permissions_bits,
            position,
            hoist: req.hoist.unwrap_or(false),
            mentionable: req.mentionable.unwrap_or(false),
            is_default: false,
        }),
    ))
}

async fn update_role(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, role_id)): Path<(i64, i64)>,
    Json(req): Json<UpdateRoleRequest>,
) -> AppResult<Json<RoleInfo>> {
    let mut conn = state.pool.get().await?;
    let actor = permissions::require_permission(&mut conn, server_id, user_id, MANAGE_ROLES).await?;
    let existing = load_role(&mut conn, server_id, role_id).await?;
    let rank = actor_rank(&mut conn, server_id, &actor).await?;

    if existing.is_default {
        if req.name.is_some() || req.position.is_some() {
            return Err(AppError::bad_request("@everyone cannot be renamed or moved"));
        }
    } else if existing.position >= rank {
        return Err(AppError::forbidden());
    }

    let name = match req.name {
        Some(name) => validate_name(&name)?,
        None => existing.name,
    };
    let color = match req.color {
        Some(color) => validate_color(color)?,
        None => existing.color,
    };
    let permissions_bits = match req.permissions {
        Some(bits) => validate_permissions(bits)?,
        None => existing.permissions,
    };
    let position = match req.position {
        Some(position) => validate_position(position)?,
        None => existing.position,
    };
    if !existing.is_default && position >= rank {
        return Err(AppError::forbidden());
    }
    let hoist = req.hoist.unwrap_or(existing.hoist);
    let mentionable = req.mentionable.unwrap_or(existing.mentionable);

    diesel::update(roles::table.find(role_id))
        .set((
            roles::name.eq(&name),
            roles::color.eq(color),
            roles::permissions.eq(permissions_bits),
            roles::position.eq(position),
            roles::hoist.eq(hoist),
            roles::mentionable.eq(mentionable),
        ))
        .execute(&mut conn)
        .await?;
    audit(
        &mut conn,
        server_id,
        user_id,
        "role.update",
        Some(&name),
        Some(serde_json::json!({ "roleId": role_id, "permissions": permissions_bits })),
    )
    .await?;
    notify(&mut conn, &Notice::RolesChanged { server_id }).await?;
    Ok(Json(RoleInfo {
        id: role_id,
        server_id,
        name,
        color,
        permissions: permissions_bits,
        position,
        hoist,
        mentionable,
        is_default: existing.is_default,
    }))
}

async fn delete_role(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, role_id)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let actor = permissions::require_permission(&mut conn, server_id, user_id, MANAGE_ROLES).await?;
    let existing = load_role(&mut conn, server_id, role_id).await?;
    if existing.is_default {
        return Err(AppError::bad_request("@everyone cannot be deleted"));
    }
    if existing.position >= actor_rank(&mut conn, server_id, &actor).await? {
        return Err(AppError::forbidden());
    }
    diesel::delete(roles::table.find(role_id))
        .execute(&mut conn)
        .await?;
    audit(
        &mut conn,
        server_id,
        user_id,
        "role.delete",
        Some(&existing.name),
        Some(serde_json::json!({ "roleId": role_id })),
    )
    .await?;
    notify(&mut conn, &Notice::RolesChanged { server_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn assign_roles(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, target_id)): Path<(i64, i64)>,
    Json(req): Json<AssignRolesRequest>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let actor = permissions::require_permission(&mut conn, server_id, user_id, MANAGE_ROLES).await?;
    let target_role = role_in(&mut conn, server_id, target_id).await?;
    if target_role == crate::protocol::Role::Owner && actor.role != crate::protocol::Role::Owner {
        return Err(AppError::forbidden());
    }
    let rank = actor_rank(&mut conn, server_id, &actor).await?;

    let mut role_ids = req.role_ids;
    role_ids.sort_unstable();
    role_ids.dedup();
    if !role_ids.is_empty() {
        let found: Vec<(i64, i32)> = roles::table
            .filter(roles::server_id.eq(server_id))
            .filter(roles::id.eq_any(&role_ids))
            .select((roles::id, roles::position))
            .load(&mut conn)
            .await?;
        if found.len() != role_ids.len() {
            return Err(AppError::bad_request("unknown role"));
        }
        if found.iter().any(|(_, position)| *position >= rank) {
            return Err(AppError::forbidden());
        }
    }

    conn.transaction(async move |conn| -> AppResult<()> {
        diesel::delete(member_roles::table)
            .filter(member_roles::server_id.eq(server_id))
            .filter(member_roles::user_id.eq(target_id))
            .execute(conn)
            .await?;
        for role_id in role_ids {
            diesel::insert_into(member_roles::table)
                .values((
                    member_roles::server_id.eq(server_id),
                    member_roles::user_id.eq(target_id),
                    member_roles::role_id.eq(role_id),
                ))
                .execute(conn)
                .await?;
        }
        Ok(())
    })
    .await?;
    audit(
        &mut conn,
        server_id,
        user_id,
        "member.roles",
        Some(&target_id.to_string()),
        None,
    )
    .await?;
    notify(&mut conn, &Notice::RolesChanged { server_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Queryable)]
struct CategoryRow {
    id: i64,
    server_id: i64,
    name: String,
    position: i32,
}

fn category_columns() -> (categories::id, categories::server_id, categories::name, categories::position) {
    (
        categories::id,
        categories::server_id,
        categories::name,
        categories::position,
    )
}

fn to_category(r: CategoryRow) -> Category {
    Category {
        id: r.id,
        server_id: r.server_id,
        name: r.name,
        position: r.position,
    }
}

async fn list_categories(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
) -> AppResult<Json<Vec<Category>>> {
    let mut conn = state.pool.get().await?;
    permissions::member_perms(&mut conn, server_id, user_id).await?;
    let rows: Vec<CategoryRow> = categories::table
        .filter(categories::server_id.eq(server_id))
        .order((categories::position, categories::id))
        .select(category_columns())
        .load(&mut conn)
        .await?;
    Ok(Json(rows.into_iter().map(to_category).collect()))
}

async fn create_category(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
    Json(req): Json<CreateCategoryRequest>,
) -> AppResult<(StatusCode, Json<Category>)> {
    let name = validate_name(&req.name)?;
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, MANAGE_CHANNELS).await?;
    let max_position: Option<i32> = categories::table
        .filter(categories::server_id.eq(server_id))
        .select(diesel::dsl::max(categories::position))
        .first(&mut conn)
        .await?;
    let position = validate_position(max_position.map_or(0, |p| p + 1))?;
    let id: i64 = diesel::insert_into(categories::table)
        .values((
            categories::server_id.eq(server_id),
            categories::name.eq(&name),
            categories::position.eq(position),
        ))
        .returning(categories::id)
        .get_result(&mut conn)
        .await?;
    audit(
        &mut conn,
        server_id,
        user_id,
        "category.create",
        Some(&name),
        Some(serde_json::json!({ "categoryId": id })),
    )
    .await?;
    notify(&mut conn, &Notice::ChannelsChanged { server_id }).await?;
    Ok((
        StatusCode::CREATED,
        Json(Category {
            id,
            server_id,
            name,
            position,
        }),
    ))
}

async fn update_category(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, category_id)): Path<(i64, i64)>,
    Json(req): Json<UpdateCategoryRequest>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, MANAGE_CHANNELS).await?;
    let existing: Option<CategoryRow> = categories::table
        .filter(categories::id.eq(category_id))
        .filter(categories::server_id.eq(server_id))
        .select(category_columns())
        .first(&mut conn)
        .await
        .optional()?;
    let existing = existing.ok_or_else(|| AppError::not_found("category"))?;
    let name = match req.name {
        Some(name) => validate_name(&name)?,
        None => existing.name,
    };
    let position = match req.position {
        Some(position) => validate_position(position)?,
        None => existing.position,
    };
    diesel::update(categories::table.find(category_id))
        .set((
            categories::name.eq(&name),
            categories::position.eq(position),
        ))
        .execute(&mut conn)
        .await?;
    audit(
        &mut conn,
        server_id,
        user_id,
        "category.update",
        Some(&name),
        Some(serde_json::json!({ "categoryId": category_id })),
    )
    .await?;
    notify(&mut conn, &Notice::ChannelsChanged { server_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_category(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, category_id)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, MANAGE_CHANNELS).await?;
    let deleted = diesel::delete(categories::table)
        .filter(categories::id.eq(category_id))
        .filter(categories::server_id.eq(server_id))
        .execute(&mut conn)
        .await?;
    if deleted == 0 {
        return Err(AppError::not_found("category"));
    }
    audit(
        &mut conn,
        server_id,
        user_id,
        "category.delete",
        None,
        Some(serde_json::json!({ "categoryId": category_id })),
    )
    .await?;
    notify(&mut conn, &Notice::ChannelsChanged { server_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_channel(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
    Json(req): Json<UpdateChannelRequest>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let server_id = channel_server(&mut conn, channel_id).await?;
    permissions::require_permission(&mut conn, server_id, user_id, MANAGE_CHANNELS).await?;

    let (name, category_id, position): (String, Option<i64>, i32) = channels::table
        .find(channel_id)
        .select((channels::name, channels::category_id, channels::position))
        .first(&mut conn)
        .await?;

    let name = match req.name {
        Some(name) => normalize_channel_name(&name)?,
        None => name,
    };
    let category_id = match req.category_id {
        None => category_id,
        Some(0) => None,
        Some(category_id) => {
            let found: Option<i64> = categories::table
                .filter(categories::id.eq(category_id))
                .filter(categories::server_id.eq(server_id))
                .select(categories::id)
                .first(&mut conn)
                .await
                .optional()?;
            if found.is_none() {
                return Err(AppError::bad_request("unknown category"));
            }
            Some(category_id)
        }
    };
    let position = match req.position {
        Some(position) => validate_position(position)?,
        None => position,
    };

    diesel::update(channels::table.find(channel_id))
        .set((
            channels::name.eq(&name),
            channels::category_id.eq(category_id),
            channels::position.eq(position),
        ))
        .execute(&mut conn)
        .await?;
    audit(
        &mut conn,
        server_id,
        user_id,
        "channel.update",
        Some(&name),
        Some(serde_json::json!({ "channelId": channel_id })),
    )
    .await?;
    notify(&mut conn, &Notice::ChannelsChanged { server_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Queryable)]
struct OverwriteRow {
    id: i64,
    channel_id: i64,
    role_id: Option<i64>,
    user_id: Option<i64>,
    allow: i64,
    deny: i64,
}

fn overwrite_columns() -> (
    channel_overwrites::id,
    channel_overwrites::channel_id,
    channel_overwrites::role_id,
    channel_overwrites::user_id,
    channel_overwrites::allow,
    channel_overwrites::deny,
) {
    (
        channel_overwrites::id,
        channel_overwrites::channel_id,
        channel_overwrites::role_id,
        channel_overwrites::user_id,
        channel_overwrites::allow,
        channel_overwrites::deny,
    )
}

fn to_overwrite(r: OverwriteRow) -> Overwrite {
    Overwrite {
        id: r.id,
        channel_id: r.channel_id,
        role_id: r.role_id,
        user_id: r.user_id,
        allow: r.allow,
        deny: r.deny,
    }
}

async fn list_overwrites(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
) -> AppResult<Json<Vec<Overwrite>>> {
    let mut conn = state.pool.get().await?;
    let server_id = channel_server(&mut conn, channel_id).await?;
    permissions::member_perms(&mut conn, server_id, user_id).await?;
    let rows: Vec<OverwriteRow> = channel_overwrites::table
        .filter(channel_overwrites::channel_id.eq(channel_id))
        .order(channel_overwrites::id)
        .select(overwrite_columns())
        .load(&mut conn)
        .await?;
    Ok(Json(rows.into_iter().map(to_overwrite).collect()))
}

async fn upsert_overwrite(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
    Json(req): Json<UpsertOverwriteRequest>,
) -> AppResult<StatusCode> {
    let (role_id, target_user) = match (req.role_id, req.user_id) {
        (Some(role_id), None) => (role_id, None),
        (None, Some(target)) => (0, Some(target)),
        _ => return Err(AppError::bad_request("exactly one of roleId/userId")),
    };
    if req.allow < 0
        || req.deny < 0
        || req.allow & !permissions::ALL != 0
        || req.deny & !permissions::ALL != 0
        || req.allow & req.deny != 0
    {
        return Err(AppError::bad_request("invalid overwrite bits"));
    }

    let mut conn = state.pool.get().await?;
    let server_id = channel_server(&mut conn, channel_id).await?;
    permissions::require_permission(&mut conn, server_id, user_id, MANAGE_CHANNELS).await?;

    if let Some(target) = target_user {
        role_in(&mut conn, server_id, target).await?;
    } else {
        let found: Option<i64> = roles::table
            .filter(roles::id.eq(role_id))
            .filter(roles::server_id.eq(server_id))
            .select(roles::id)
            .first(&mut conn)
            .await
            .optional()?;
        if found.is_none() {
            return Err(AppError::bad_request("unknown role"));
        }
    }

    let allow = req.allow;
    let deny = req.deny;
    conn.transaction(async move |conn| -> AppResult<()> {
        match target_user {
            Some(target) => {
                diesel::delete(channel_overwrites::table)
                    .filter(channel_overwrites::channel_id.eq(channel_id))
                    .filter(channel_overwrites::user_id.eq(target))
                    .execute(conn)
                    .await?;
            }
            None => {
                diesel::delete(channel_overwrites::table)
                    .filter(channel_overwrites::channel_id.eq(channel_id))
                    .filter(channel_overwrites::role_id.eq(role_id))
                    .execute(conn)
                    .await?;
            }
        }
        diesel::insert_into(channel_overwrites::table)
            .values((
                channel_overwrites::channel_id.eq(channel_id),
                channel_overwrites::role_id.eq(if target_user.is_some() {
                    None
                } else {
                    Some(role_id)
                }),
                channel_overwrites::user_id.eq(target_user),
                channel_overwrites::allow.eq(allow),
                channel_overwrites::deny.eq(deny),
            ))
            .execute(conn)
            .await?;
        Ok(())
    })
    .await?;
    audit(
        &mut conn,
        server_id,
        user_id,
        "channel.overwrite",
        None,
        Some(serde_json::json!({ "channelId": channel_id })),
    )
    .await?;
    notify(&mut conn, &Notice::ChannelsChanged { server_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_overwrite(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((channel_id, overwrite_id)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let server_id = channel_server(&mut conn, channel_id).await?;
    permissions::require_permission(&mut conn, server_id, user_id, MANAGE_CHANNELS).await?;
    let deleted = diesel::delete(channel_overwrites::table)
        .filter(channel_overwrites::id.eq(overwrite_id))
        .filter(channel_overwrites::channel_id.eq(channel_id))
        .execute(&mut conn)
        .await?;
    if deleted == 0 {
        return Err(AppError::not_found("overwrite"));
    }
    audit(
        &mut conn,
        server_id,
        user_id,
        "channel.overwrite.delete",
        None,
        Some(serde_json::json!({ "channelId": channel_id })),
    )
    .await?;
    notify(&mut conn, &Notice::ChannelsChanged { server_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(diesel::QueryableByName)]
struct AuditRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    id: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    server_id: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    actor_id: i64,
    #[diesel(sql_type = diesel::sql_types::Text)]
    actor_username: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    action: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    target: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    created_at: DateTime<Utc>,
}

async fn list_audit(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
) -> AppResult<Json<Vec<AuditEntry>>> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, VIEW_AUDIT_LOG).await?;
    let rows: Vec<AuditRow> = diesel::sql_query(
        "SELECT a.id, a.server_id, a.actor_id, u.username AS actor_username,
                a.action, a.target, a.created_at
         FROM audit_log a JOIN users u ON u.id = a.actor_id
         WHERE a.server_id = $1 ORDER BY a.id DESC LIMIT 50",
    )
    .bind::<diesel::sql_types::BigInt, _>(server_id)
    .load(&mut conn)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| AuditEntry {
                id: r.id,
                server_id: r.server_id,
                actor_id: r.actor_id,
                actor_username: r.actor_username,
                action: r.action,
                target: r.target,
                created_at: r.created_at.timestamp_millis(),
            })
            .collect(),
    ))
}