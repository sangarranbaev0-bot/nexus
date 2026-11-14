use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
};
use chrono::{DateTime, Duration, Utc};
use diesel::{dsl::max, prelude::*, result::DatabaseErrorKind};
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    events::{Notice, notify},
    permissions,
    presence::STATUS_SQL,
    protocol::{
        Channel, ChannelKind, CreateChannelRequest, CreateInviteRequest, CreateServerRequest,
        Invite, Member, Presence, Role, Server, UpdateMemberRequest,
    },
    reads,
    schema::{channels, dm_participants, invites, members, roles, servers, users},
};

const DEFAULT_CHANNEL: &str = "общий";
const DEFAULT_VOICE_CHANNEL: &str = "голосовой";
const INVITE_CODE_LEN: usize = 8;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/servers", get(list_servers).post(create_server))
        .route(
            "/api/servers/{id}/channels",
            get(list_channels).post(create_channel),
        )
        .route("/api/servers/{id}/members", get(list_members))
        .route("/api/servers/{id}/members/{user_id}", patch(update_member))
        .route("/api/servers/{id}/wave", post(wave))
        .route("/api/servers/{id}/invites", post(create_invite))
        .route("/api/channels/{id}", delete(delete_channel))
        .route("/api/invites/{code}/join", post(join_by_invite))
}

/// Роль пользователя на сервере. Не-участнику 404: не раскрываем, что сервер существует.
pub async fn role_in(
    conn: &mut AsyncPgConnection,
    server_id: i64,
    user_id: i64,
) -> AppResult<Role> {
    let role: Option<String> = members::table
        .find((server_id, user_id))
        .select(members::role)
        .first(conn)
        .await
        .optional()?;
    role.as_deref()
        .and_then(Role::parse)
        .ok_or_else(|| AppError::not_found("server"))
}

/// Как `role_in`, но участнику ниже `min` отвечает 403.
pub async fn require_role(
    conn: &mut AsyncPgConnection,
    server_id: i64,
    user_id: i64,
    min: Role,
) -> AppResult<Role> {
    let role = role_in(conn, server_id, user_id).await?;
    if role < min {
        return Err(AppError::forbidden());
    }
    Ok(role)
}

/// Сервер, которому принадлежит канал; 404, если канала нет или это ЛС.
pub async fn channel_server(conn: &mut AsyncPgConnection, channel_id: i64) -> AppResult<i64> {
    let server_id: Option<Option<i64>> = channels::table
        .find(channel_id)
        .select(channels::server_id)
        .first(conn)
        .await
        .optional()?;
    server_id
        .flatten()
        .ok_or_else(|| AppError::not_found("channel"))
}

/// Кто и как может работать с каналом: участник сервера (с ролью) или собеседник в ЛС.
pub struct ChannelAccess {
    /// `None` — личные сообщения.
    pub server_id: Option<i64>,
    pub kind: ChannelKind,
    /// Роль на сервере; в ЛС ролей нет.
    pub role: Option<Role>,
}

/// Проверяет доступ к любому каналу. Постороннему 404, как и для серверов.
pub async fn channel_access(
    conn: &mut AsyncPgConnection,
    channel_id: i64,
    user_id: i64,
) -> AppResult<ChannelAccess> {
    let (server_id, kind): (Option<i64>, String) = channels::table
        .find(channel_id)
        .select((channels::server_id, channels::kind))
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| AppError::not_found("channel"))?;
    let kind = ChannelKind::parse(&kind);
    match server_id {
        Some(server_id) => {
            let role = role_in(conn, server_id, user_id)
                .await
                .map_err(|_| AppError::not_found("channel"))?;
            Ok(ChannelAccess {
                server_id: Some(server_id),
                kind,
                role: Some(role),
            })
        }
        None => {
            let participant: Option<i64> = dm_participants::table
                .find((channel_id, user_id))
                .select(dm_participants::user_id)
                .first(conn)
                .await
                .optional()?;
            participant.ok_or_else(|| AppError::not_found("channel"))?;
            Ok(ChannelAccess {
                server_id: None,
                kind,
                role: None,
            })
        }
    }
}

fn validate_server_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if !(2..=100).contains(&name.chars().count()) {
        return Err(AppError::bad_request(
            "server name must be 2-100 characters",
        ));
    }
    Ok(name.to_string())
}

/// Как в Discord: нижний регистр, пробелы → дефисы, только буквы, цифры, `-` и `_`.
pub fn normalize_channel_name(name: &str) -> AppResult<String> {
    let name: String = name
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");
    let valid = (1..=32).contains(&name.chars().count())
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_');
    if !valid {
        return Err(AppError::bad_request(
            "channel name must be 1-32 letters, digits, '-' or '_'",
        ));
    }
    Ok(name)
}

async fn list_servers(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> AppResult<Json<Vec<Server>>> {
    let mut conn = state.pool.get().await?;
    Ok(Json(reads::servers_with_unread(&mut conn, user_id).await?))
}

async fn create_server(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<CreateServerRequest>,
) -> AppResult<(StatusCode, Json<Server>)> {
    let name = validate_server_name(&req.name)?;
    let mut conn = state.pool.get().await?;
    let server = conn
        .transaction(async move |conn| -> AppResult<Server> {
            let id: i64 = diesel::insert_into(servers::table)
                .values((servers::name.eq(&name), servers::owner_id.eq(user_id)))
                .returning(servers::id)
                .get_result(conn)
                .await?;
            diesel::insert_into(members::table)
                .values((
                    members::server_id.eq(id),
                    members::user_id.eq(user_id),
                    members::role.eq(Role::Owner.as_str()),
                ))
                .execute(conn)
                .await?;
            diesel::insert_into(roles::table)
                .values((
                    roles::server_id.eq(id),
                    roles::name.eq("@everyone"),
                    roles::permissions.eq(0i64),
                    roles::position.eq(0),
                    roles::is_default.eq(true),
                ))
                .execute(conn)
                .await?;
            // Системный канал — текстовый «общий», создаём первым и запоминаем его id.
            let system_channel_id: i64 = diesel::insert_into(channels::table)
                .values((
                    channels::server_id.eq(id),
                    channels::name.eq(DEFAULT_CHANNEL),
                    channels::position.eq(0),
                    channels::kind.eq(ChannelKind::Text.as_str()),
                ))
                .returning(channels::id)
                .get_result(conn)
                .await?;
            diesel::insert_into(channels::table)
                .values((
                    channels::server_id.eq(id),
                    channels::name.eq(DEFAULT_VOICE_CHANNEL),
                    channels::position.eq(1),
                    channels::kind.eq(ChannelKind::Voice.as_str()),
                ))
                .execute(conn)
                .await?;
            diesel::update(servers::table.find(id))
                .set(servers::system_channel_id.eq(system_channel_id))
                .execute(conn)
                .await?;
            let system_channel_id = Some(system_channel_id);
            Ok(Server {
                id,
                name,
                owner_id: user_id,
                my_role: Role::Owner,
                my_permissions: permissions::ALL,
                unread_count: 0,
                system_channel_id,
            })
        })
        .await?;
    notify(
        &mut conn,
        &Notice::MemberJoined {
            server_id: server.id,
            user_id,
        },
    )
    .await?;
    // Приветствие владельца в системном канале.
    if let Some(cid) = server.system_channel_id {
        let _ = crate::messages::post_system(
            &mut conn,
            server.id,
            cid,
            user_id,
            "welcome",
            Some(&server.name),
            "",
        )
        .await;
    }
    Ok((StatusCode::CREATED, Json(server)))
}

async fn list_channels(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
) -> AppResult<Json<Vec<Channel>>> {
    let mut conn = state.pool.get().await?;
    let perms = permissions::member_perms(&mut conn, server_id, user_id).await?;
    let mut list = reads::channels_with_unread(&mut conn, user_id, server_id).await?;
    if let Some(visible) = permissions::visible_channel_ids(&mut conn, &perms, server_id).await? {
        list.retain(|c| visible.contains(&c.id));
    }
    Ok(Json(list))
}

async fn create_channel(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
    Json(req): Json<CreateChannelRequest>,
) -> AppResult<(StatusCode, Json<Channel>)> {
    let name = normalize_channel_name(&req.name)?;
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, permissions::MANAGE_CHANNELS)
        .await?;

    // shortcut: позиция = max + 1 без блокировки, при одновременном создании возможен дубль
    // позиции (порядок тогда решает id); заменить, когда появится перетаскивание каналов.
    let last: Option<i32> = channels::table
        .filter(channels::server_id.eq(server_id))
        .select(max(channels::position))
        .first(&mut conn)
        .await?;
    let position = last.map_or(0, |p| p + 1);
    let kind = req.kind.unwrap_or_default();
    let id: i64 = diesel::insert_into(channels::table)
        .values((
            channels::server_id.eq(server_id),
            channels::name.eq(&name),
            channels::position.eq(position),
            channels::kind.eq(kind.as_str()),
        ))
        .returning(channels::id)
        .get_result(&mut conn)
        .await?;
    notify(&mut conn, &Notice::ChannelsChanged { server_id }).await?;
    // Событие в системный канал (не мешает созданию).
    if let Ok(Some(system_id)) = system_channel(server_id, &mut conn).await {
        let detail = format!("#{name}");
        let _ = crate::messages::post_system(
            &mut conn,
            server_id,
            system_id,
            user_id,
            "channelCreate",
            Some(&detail),
            "",
        )
        .await;
    }
    let channel = Channel {
        id,
        server_id,
        name,
        position,
        kind,
        category_id: None,
        unread_count: 0,
    };
    Ok((StatusCode::CREATED, Json(channel)))
}

/// Канал событий сервера, если он задан.
async fn system_channel(server_id: i64, conn: &mut AsyncPgConnection) -> AppResult<Option<i64>> {
    Ok(servers::table
        .find(server_id)
        .select(servers::system_channel_id)
        .first(conn)
        .await?)
}

/// «Помахать»: короткое событие в системный канал сервера.
async fn wave(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    role_in(&mut conn, server_id, user_id).await?;
    let system_id = system_channel(server_id, &mut conn)
        .await?
        .ok_or_else(|| AppError::not_found("system channel"))?;
    crate::messages::post_system(&mut conn, server_id, system_id, user_id, "wave", None, "")
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_channel(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let server_id = channel_server(&mut conn, channel_id).await?;
    // Посторонний получает 404 и для канала: role_in отвечает так же, как на несуществующий.
    permissions::require_permission(&mut conn, server_id, user_id, permissions::MANAGE_CHANNELS)
        .await?;
    diesel::delete(channels::table.find(channel_id))
        .execute(&mut conn)
        .await?;
    notify(&mut conn, &Notice::ChannelsChanged { server_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(diesel::QueryableByName)]
struct MemberRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    user_id: i64,
    #[diesel(sql_type = diesel::sql_types::Text)]
    username: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    role: String,
    #[diesel(sql_type = diesel::sql_types::Array<diesel::sql_types::BigInt>)]
    role_ids: Vec<i64>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    status: String,
}

async fn list_members(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
) -> AppResult<Json<Vec<Member>>> {
    let mut conn = state.pool.get().await?;
    role_in(&mut conn, server_id, user_id).await?;
    let rows: Vec<MemberRow> = diesel::sql_query(format!(
        "SELECT u.id AS user_id, u.username, mem.role,
                COALESCE((
                    SELECT array_agg(mr.role_id ORDER BY r.position DESC, r.id)
                    FROM member_roles mr JOIN roles r ON r.id = mr.role_id
                    WHERE mr.server_id = mem.server_id AND mr.user_id = mem.user_id
                ), '{{}}') AS role_ids,
                {STATUS_SQL} AS status
         FROM members mem JOIN users u ON u.id = mem.user_id
         WHERE mem.server_id = $1
         ORDER BY mem.joined_at"
    ))
    .bind::<diesel::sql_types::BigInt, _>(server_id)
    .load(&mut conn)
    .await?;
    Ok(Json(
        rows.into_iter()
            .filter_map(|r| {
                Some(Member {
                    user_id: r.user_id,
                    username: r.username,
                    role: Role::parse(&r.role)?,
                    role_ids: r.role_ids,
                    status: Presence::parse(&r.status),
                })
            })
            .collect(),
    ))
}

async fn update_member(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, target_id)): Path<(i64, i64)>,
    Json(req): Json<UpdateMemberRequest>,
) -> AppResult<StatusCode> {
    if req.role == Role::Owner {
        return Err(AppError::bad_request("ownership transfer is not supported"));
    }
    let mut conn = state.pool.get().await?;
    require_role(&mut conn, server_id, user_id, Role::Owner).await?;
    if target_id == user_id {
        return Err(AppError::bad_request("owner cannot change own role"));
    }
    let updated = diesel::update(members::table.find((server_id, target_id)))
        .set(members::role.eq(req.role.as_str()))
        .execute(&mut conn)
        .await?;
    if updated == 0 {
        return Err(AppError::not_found("member"));
    }
    // Событие в системный канал: имя участника и новая роль.
    if let Ok(Some(system_id)) = system_channel(server_id, &mut conn).await {
        let username: Option<String> = users::table
            .find(target_id)
            .select(users::username)
            .first(&mut conn)
            .await
            .optional()?;
        if let Some(username) = username {
            let role_label = match req.role {
                Role::Admin => "администратор",
                Role::Member => "участник",
                Role::Owner => "владелец",
            };
            let detail = format!("{username}|{role_label}");
            let _ = crate::messages::post_system(
                &mut conn,
                server_id,
                system_id,
                user_id,
                "roleUpdate",
                Some(&detail),
                "",
            )
            .await;
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

fn invite_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
    let mut bytes = [0u8; INVITE_CODE_LEN];
    getrandom::fill(&mut bytes).expect("OS RNG available");
    // Небольшой перекос от `%` для инвайтов неважен: код не секрет, а приглашение.
    bytes
        .iter()
        .map(|b| ALPHABET[*b as usize % ALPHABET.len()] as char)
        .collect()
}

fn to_ms(t: Option<DateTime<Utc>>) -> Option<i64> {
    t.map(|t| t.timestamp_millis())
}

async fn create_invite(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
    body: Option<Json<CreateInviteRequest>>,
) -> AppResult<(StatusCode, Json<Invite>)> {
    let req = body.map(|Json(r)| r).unwrap_or_default();
    if req.max_uses.is_some_and(|n| n < 1) {
        return Err(AppError::bad_request("maxUses must be at least 1"));
    }
    if req.expires_in_hours.is_some_and(|h| h == 0 || h > 24 * 30) {
        return Err(AppError::bad_request("expiresInHours must be 1-720"));
    }
    let mut conn = state.pool.get().await?;
    // По умолчанию приглашать может любой участник (право есть в MEMBER_BASE).
    permissions::require_permission(&mut conn, server_id, user_id, permissions::CREATE_INVITE)
        .await?;

    let expires_at = req
        .expires_in_hours
        .map(|h| Utc::now() + Duration::hours(h.into()));
    let code = invite_code();
    diesel::insert_into(invites::table)
        .values((
            invites::code.eq(&code),
            invites::server_id.eq(server_id),
            invites::created_by.eq(user_id),
            invites::expires_at.eq(expires_at),
            invites::max_uses.eq(req.max_uses),
        ))
        .execute(&mut conn)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(Invite {
            code,
            server_id,
            expires_at: to_ms(expires_at),
            max_uses: req.max_uses,
            uses: 0,
        }),
    ))
}

async fn join_by_invite(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(code): Path<String>,
) -> AppResult<Json<Server>> {
    let mut conn = state.pool.get().await?;
    let server_id = conn
        .transaction(async move |conn| -> AppResult<i64> {
            let server_id: i64 = invites::table
                .find(&code)
                .select(invites::server_id)
                .first(conn)
                .await
                .optional()?
                .ok_or_else(|| AppError::not_found("invite"))?;

            // Уже участник: просто возвращаем сервер, использование не тратим.
            if role_in(conn, server_id, user_id).await.is_ok() {
                return Ok(server_id);
            }

            // Атомарно: срок и лимит проверяются в том же UPDATE, что увеличивает счётчик.
            let used = diesel::update(invites::table.find(&code))
                .filter(
                    invites::expires_at
                        .is_null()
                        .or(invites::expires_at.gt(Utc::now())),
                )
                .filter(
                    invites::max_uses
                        .is_null()
                        .or(invites::uses.lt(invites::max_uses.assume_not_null())),
                )
                .set(invites::uses.eq(invites::uses + 1))
                .execute(conn)
                .await?;
            if used == 0 {
                return Err(AppError(StatusCode::GONE, "invite expired".into()));
            }

            diesel::insert_into(members::table)
                .values((
                    members::server_id.eq(server_id),
                    members::user_id.eq(user_id),
                    members::role.eq(Role::Member.as_str()),
                ))
                .execute(conn)
                .await
                .map_err(|e| match e {
                    // Параллельный join того же пользователя: он уже участник.
                    diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                        AppError::conflict("already a member")
                    }
                    e => e.into(),
                })?;
            Ok(server_id)
        })
        .await?;

    notify(&mut conn, &Notice::MemberJoined { server_id, user_id }).await?;

    let (name, owner_id, system_channel_id): (String, i64, Option<i64>) = servers::table
        .find(server_id)
        .select((servers::name, servers::owner_id, servers::system_channel_id))
        .first(&mut conn)
        .await?;
    // Событие входа в системный канал.
    if let Some(cid) = system_channel_id {
        let _ = crate::messages::post_system(
            &mut conn,
            server_id,
            cid,
            user_id,
            "memberJoin",
            None,
            "",
        )
        .await;
    }
    let my_role = role_in(&mut conn, server_id, user_id).await?;
    let my_permissions = permissions::member_perms(&mut conn, server_id, user_id)
        .await?
        .bits;
    Ok(Json(Server {
        id: server_id,
        name,
        owner_id,
        my_role,
        my_permissions,
        // Вступивший не получает историю как непрочитанную.
        unread_count: 0,
        system_channel_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_names_are_normalized_like_discord() {
        assert_eq!(
            normalize_channel_name("  Мой Канал  ").unwrap(),
            "мой-канал"
        );
        assert_eq!(normalize_channel_name("dev_ops-2").unwrap(), "dev_ops-2");
        assert!(normalize_channel_name("").is_err());
        assert!(normalize_channel_name("bad!name").is_err());
        assert!(normalize_channel_name(&"x".repeat(33)).is_err());
    }

    #[test]
    fn role_order_and_roundtrip() {
        assert!(Role::Member < Role::Admin && Role::Admin < Role::Owner);
        for r in [Role::Member, Role::Admin, Role::Owner] {
            assert_eq!(Role::parse(r.as_str()), Some(r));
        }
        assert_eq!(Role::parse("root"), None);
    }

    #[test]
    fn invite_codes_have_expected_shape() {
        let a = invite_code();
        assert_eq!(a.len(), INVITE_CODE_LEN);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_ne!(a, invite_code());
    }
}
