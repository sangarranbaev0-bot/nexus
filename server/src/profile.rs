use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
};
use axum_extra::extract::CookieJar;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{
    AppState,
    auth::{self, AuthUser},
    error::{AppError, AppResult},
    permissions::CHANGE_NICKNAME,
    protocol::{
        ExportResponse, LoginWithCode, Note, Session, TwoFactorCode, TwoFactorSetup,
        UpdateMemberProfileRequest, UpdateNoteRequest, UpdateProfileRequest, User,
    },
    rate_limit,
    schema::{members, recovery_codes, refresh_tokens, user_notes, users},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/me/profile", get(profile_get).patch(profile_update))
        .route("/api/me/notes", get(notes_list))
        .route("/api/me/notes/{id}", put(note_put).delete(note_delete))
        .route("/api/me/sessions", get(sessions_list))
        .route("/api/me/sessions/{id}", delete(session_delete))
        .route("/api/me/sessions/logout-others", post(logout_others))
        .route("/api/me/2fa/setup", post(twofa_setup))
        .route("/api/me/2fa/enable", post(twofa_enable))
        .route("/api/me/2fa/disable", post(twofa_disable))
        .route("/api/auth/login-2fa", post(login_2fa))
        .route("/api/me/export", get(export))
        .route("/api/servers/{id}/members/me", put(member_profile))
}

const B32: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

fn b32_encode(data: &[u8]) -> String {
    let mut out = String::new();
    let mut bits = 0u32;
    let mut acc = 0u32;
    for &b in data {
        acc = (acc << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(B32[((acc >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(B32[((acc << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn b32_decode(s: &str) -> Option<Vec<u8>> {
    let mut acc = 0u32;
    let mut bits = 0u32;
    let mut out = Vec::new();
    for c in s.trim().to_ascii_uppercase().chars().filter(|c| *c != '=') {
        let idx = B32.iter().position(|&x| x == c as u8)? as u32;
        acc = (acc << 5) | idx;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

fn hmac_sha1(key: &[u8], msg: &[u8]) -> [u8; 20] {
    use sha1::{Digest, Sha1};
    let mut key = key.to_vec();
    if key.len() > 64 {
        key = Sha1::digest(&key).to_vec();
    }
    key.resize(64, 0);
    let ipad: Vec<u8> = key.iter().map(|b| b ^ 0x36).collect();
    let opad: Vec<u8> = key.iter().map(|b| b ^ 0x5c).collect();
    let mut inner = Sha1::new();
    inner.update(&ipad);
    inner.update(msg);
    let inner = inner.finalize();
    let mut outer = Sha1::new();
    outer.update(&opad);
    outer.update(inner);
    let out = outer.finalize();
    let mut res = [0u8; 20];
    res.copy_from_slice(&out);
    res
}

fn totp(secret: &str, step: i64) -> String {
    let key = b32_decode(secret).unwrap_or_default();
    let counter = step.to_be_bytes();
    let mac = hmac_sha1(&key, &counter);
    let offset = (mac[19] & 0x0f) as usize;
    let code = u32::from_be_bytes([
        mac[offset] & 0x7f,
        mac[offset + 1],
        mac[offset + 2],
        mac[offset + 3],
    ]) % 1_000_000;
    format!("{code:06}")
}

fn verify_code(secret: &str, code: &str) -> bool {
    let now = chrono::Utc::now().timestamp() / 30;
    (now - 1..=now + 1).any(|step| totp(secret, step) == code.trim())
}

fn random_string(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    getrandom::fill(&mut bytes).expect("OS RNG available");
    hex::encode(bytes)
}

async fn load_user(conn: &mut diesel_async::AsyncPgConnection, user_id: i64) -> AppResult<User> {
    let row: auth::UserRow = users::table
        .find(user_id)
        .select(auth::UserRow::as_select())
        .first(conn)
        .await?;
    Ok(row.into())
}

async fn profile_get(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> AppResult<Json<User>> {
    let mut conn = state.pool.get().await?;
    Ok(Json(load_user(&mut conn, user_id).await?))
}

fn limit_text(s: &str, max: usize) -> AppResult<&str> {
    let s = s.trim();
    if s.chars().count() > max {
        Err(AppError::bad_request("value too long"))
    } else {
        Ok(s)
    }
}

async fn profile_update(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> AppResult<Json<User>> {
    let mut conn = state.pool.get().await?;
    if let Some(v) = req.about {
        diesel::update(users::table.find(user_id))
            .set(users::about.eq(limit_text(&v, 2000)?))
            .execute(&mut conn)
            .await?;
    }
    if let Some(v) = req.banner_url {
        diesel::update(users::table.find(user_id))
            .set(users::banner_url.eq(limit_text(&v, 512)?))
            .execute(&mut conn)
            .await?;
    }
    if let Some(v) = req.pronouns {
        diesel::update(users::table.find(user_id))
            .set(users::pronouns.eq(limit_text(&v, 64)?))
            .execute(&mut conn)
            .await?;
    }
    if let Some(v) = req.accent_color {
        diesel::update(users::table.find(user_id))
            .set(users::accent_color.eq(v))
            .execute(&mut conn)
            .await?;
    }
    if let Some(v) = req.status_text {
        diesel::update(users::table.find(user_id))
            .set(users::status_text.eq(limit_text(&v, 128)?))
            .execute(&mut conn)
            .await?;
    }
    if let Some(v) = req.status_emoji {
        diesel::update(users::table.find(user_id))
            .set(users::status_emoji.eq(limit_text(&v, 32)?))
            .execute(&mut conn)
            .await?;
    }
    if let Some(v) = req.status_expires_at {
        let until = v
            .map(|ms| {
                chrono::DateTime::from_timestamp_millis(ms)
                    .ok_or_else(|| AppError::bad_request("invalid time"))
            })
            .transpose()?;
        diesel::update(users::table.find(user_id))
            .set(users::status_expires_at.eq(until))
            .execute(&mut conn)
            .await?;
    }
    if let Some(v) = req.dnd {
        diesel::update(users::table.find(user_id))
            .set(users::dnd.eq(v))
            .execute(&mut conn)
            .await?;
    }
    if let Some(v) = req.activity_name {
        diesel::update(users::table.find(user_id))
            .set(users::activity_name.eq(limit_text(&v, 128)?))
            .execute(&mut conn)
            .await?;
    }
    if let Some(v) = req.activity_type {
        let allowed = [
            "playing",
            "streaming",
            "listening",
            "watching",
            "custom",
            "competing",
        ];
        if allowed.contains(&v.as_str()) {
            diesel::update(users::table.find(user_id))
                .set(users::activity_type.eq(limit_text(&v, 16)?))
                .execute(&mut conn)
                .await?;
        }
    }
    if let Some(v) = req.notify_mode {
        let allowed = ["all", "mentions", "none"];
        if allowed.contains(&v.as_str()) {
            diesel::update(users::table.find(user_id))
                .set(users::notify_mode.eq(limit_text(&v, 8)?))
                .execute(&mut conn)
                .await?;
        }
    }
    if let Some(ref v) = req.badge {
        let trimmed = v.trim();
        if trimmed.chars().count() > 16 {
            return Err(AppError::bad_request("badge must be 16 chars or less"));
        }
        diesel::update(users::table.find(user_id))
            .set(users::badge.eq(trimmed))
            .execute(&mut conn)
            .await?;
    }
    Ok(Json(load_user(&mut conn, user_id).await?))
}

#[derive(QueryableByName)]
struct NoteRow {
    #[diesel(sql_type = diesel::sql_types::Int8)]
    target_id: i64,
    #[diesel(sql_type = diesel::sql_types::Text)]
    note: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    nickname: String,
}

async fn notes_list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> AppResult<Json<Vec<Note>>> {
    let mut conn = state.pool.get().await?;
    let rows: Vec<NoteRow> =
        diesel::sql_query("SELECT target_id, note, nickname FROM user_notes WHERE owner_id=$1")
            .bind::<diesel::sql_types::BigInt, _>(user_id)
            .load(&mut conn)
            .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| Note {
                target_id: r.target_id,
                note: r.note,
                nickname: r.nickname,
            })
            .collect(),
    ))
}

async fn note_put(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(target_id): Path<i64>,
    Json(req): Json<UpdateNoteRequest>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    diesel::sql_query(
        "INSERT INTO user_notes (owner_id,target_id,note,nickname) VALUES ($1,$2,$3,$4) \
         ON CONFLICT (owner_id,target_id) DO UPDATE SET note=EXCLUDED.note, nickname=EXCLUDED.nickname, updated_at=now()",
    )
    .bind::<diesel::sql_types::BigInt, _>(user_id)
    .bind::<diesel::sql_types::BigInt, _>(target_id)
    .bind::<diesel::sql_types::Text, _>(limit_text(&req.note, 2000)?)
    .bind::<diesel::sql_types::Text, _>(limit_text(&req.nickname, 64)?)
    .execute(&mut conn).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn note_delete(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(target_id): Path<i64>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    diesel::delete(user_notes::table)
        .filter(user_notes::owner_id.eq(user_id))
        .filter(user_notes::target_id.eq(target_id))
        .execute(&mut conn)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(QueryableByName)]
struct SessionRow {
    #[diesel(sql_type = diesel::sql_types::Int8)]
    id: i64,
    #[diesel(sql_type = diesel::sql_types::Text)]
    device_label: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    user_agent: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    ip: String,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    created_at: chrono::DateTime<chrono::Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    expires_at: chrono::DateTime<chrono::Utc>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    revoked_at: Option<chrono::DateTime<chrono::Utc>>,
}

async fn sessions_list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    jar: CookieJar,
) -> AppResult<Json<Vec<Session>>> {
    let mut conn = state.pool.get().await?;
    let rows: Vec<SessionRow> = diesel::sql_query(
        "SELECT id,device_label,user_agent,ip,created_at,expires_at,last_used_at,revoked_at \
         FROM refresh_tokens WHERE user_id=$1 ORDER BY created_at DESC LIMIT 100",
    )
    .bind::<diesel::sql_types::BigInt, _>(user_id)
    .load(&mut conn)
    .await?;
    let current_hash = jar
        .get(auth::REFRESH_COOKIE)
        .map(|c| auth::sha256_hex(c.value()))
        .unwrap_or_default();
    let current_id: Option<i64> =
        diesel::sql_query("SELECT id FROM refresh_tokens WHERE token_hash=$1")
            .bind::<diesel::sql_types::Text, _>(&current_hash)
            .load(&mut conn)
            .await?
            .into_iter()
            .next()
            .map(|r: IdRow| r.id);
    Ok(Json(
        rows.into_iter()
            .map(|r| Session {
                id: r.id,
                device_label: r.device_label,
                user_agent: r.user_agent,
                ip: r.ip,
                created_at: r.created_at.timestamp_millis(),
                expires_at: r.expires_at.timestamp_millis(),
                last_used_at: r.last_used_at.map(|t| t.timestamp_millis()),
                revoked: r.revoked_at.is_some(),
                current: Some(r.id) == current_id,
            })
            .collect(),
    ))
}

#[derive(QueryableByName)]
struct IdRow {
    #[diesel(sql_type = diesel::sql_types::Int8)]
    id: i64,
}

async fn session_delete(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(session_id): Path<i64>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    diesel::update(refresh_tokens::table)
        .filter(refresh_tokens::id.eq(session_id))
        .filter(refresh_tokens::user_id.eq(user_id))
        .set(refresh_tokens::revoked_at.eq(chrono::Utc::now()))
        .execute(&mut conn)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn logout_others(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    jar: CookieJar,
) -> AppResult<StatusCode> {
    let current = jar
        .get(auth::REFRESH_COOKIE)
        .map(|c| auth::sha256_hex(c.value()));
    let mut conn = state.pool.get().await?;
    let mut q = diesel::update(refresh_tokens::table)
        .filter(refresh_tokens::user_id.eq(user_id))
        .filter(refresh_tokens::revoked_at.is_null())
        .into_boxed();
    if let Some(hash) = current {
        q = q.filter(refresh_tokens::token_hash.ne(hash));
    }
    q.set(refresh_tokens::revoked_at.eq(chrono::Utc::now()))
        .execute(&mut conn)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn twofa_setup(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> AppResult<Json<TwoFactorSetup>> {
    let mut bytes = [0u8; 20];
    getrandom::fill(&mut bytes).expect("OS RNG available");
    let secret = b32_encode(&bytes);
    let mut conn = state.pool.get().await?;
    diesel::update(users::table.find(user_id))
        .set(users::totp_secret.eq(Some(&secret)))
        .execute(&mut conn)
        .await?;
    let user: User = load_user(&mut conn, user_id).await?;
    let mut codes = Vec::new();
    for _ in 0..8 {
        let raw = format!(
            "{}-{}",
            random_string(4).to_uppercase(),
            random_string(4).to_uppercase()
        );
        codes.push(raw.clone());
        diesel::insert_into(recovery_codes::table)
            .values((
                recovery_codes::user_id.eq(user_id),
                recovery_codes::code_hash.eq(auth::sha256_hex(&raw)),
            ))
            .on_conflict_do_nothing()
            .execute(&mut conn)
            .await?;
    }
    let uri = format!(
        "otpauth://totp/NexusChat:{}?secret={}&issuer=NexusChat&algorithm=SHA1&digits=6&period=30",
        user.username, secret
    );
    Ok(Json(TwoFactorSetup {
        secret,
        uri,
        recovery_codes: codes,
    }))
}

async fn twofa_enable(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<TwoFactorCode>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let secret: Option<String> = users::table
        .find(user_id)
        .select(users::totp_secret)
        .first(&mut conn)
        .await?;
    let Some(secret) = secret else {
        return Err(AppError::bad_request("setup 2FA first"));
    };
    if !verify_code(&secret, &req.code) {
        return Err(AppError::bad_request("invalid code"));
    }
    diesel::update(users::table.find(user_id))
        .set(users::two_factor_enabled.eq(true))
        .execute(&mut conn)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn twofa_disable(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<TwoFactorCode>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let secret: Option<String> = users::table
        .find(user_id)
        .select(users::totp_secret)
        .first(&mut conn)
        .await?;
    if secret.as_deref().is_none_or(|s| !verify_code(s, &req.code)) {
        return Err(AppError::bad_request("invalid code"));
    }
    diesel::update(users::table.find(user_id))
        .set((
            users::two_factor_enabled.eq(false),
            users::totp_secret.eq(None::<String>),
        ))
        .execute(&mut conn)
        .await?;
    diesel::delete(recovery_codes::table.filter(recovery_codes::user_id.eq(user_id)))
        .execute(&mut conn)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn login_2fa(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: axum::http::HeaderMap,
    Json(req): Json<LoginWithCode>,
) -> AppResult<(CookieJar, Json<crate::protocol::AuthResponse>)> {
    rate_limit::auth(&headers).map_err(|(code, msg)| AppError(code, msg))?;
    let email = req.email.trim().to_lowercase();
    let mut conn = state.pool.get().await?;
    let row: Option<auth::UserRow> = users::table
        .filter(users::email.eq(&email))
        .select(auth::UserRow::as_select())
        .first(&mut conn)
        .await
        .optional()?;
    let Some(row) = row else {
        return Err(AppError::unauthorized());
    };
    let hash = row.password_hash.clone();
    let password = req.password;
    let ok = tokio::task::spawn_blocking(move || bcrypt::verify(password, &hash)).await??;
    if !ok {
        return Err(AppError::unauthorized());
    }
    let user: User = row.into();
    let secret: Option<String> = users::table
        .find(user.id)
        .select(users::totp_secret)
        .first(&mut conn)
        .await?;
    let secret = secret.ok_or_else(|| AppError::bad_request("2FA is not configured"))?;

    let mut verified = verify_code(&secret, &req.code);
    if !verified && !req.recovery_code.is_empty() {
        let hash = auth::sha256_hex(req.recovery_code.trim());
        let found: Option<String> = recovery_codes::table
            .filter(recovery_codes::user_id.eq(user.id))
            .filter(recovery_codes::code_hash.eq(&hash))
            .filter(recovery_codes::used_at.is_null())
            .select(recovery_codes::code_hash)
            .first(&mut conn)
            .await
            .optional()?;
        if found.is_some() {
            diesel::update(recovery_codes::table)
                .filter(recovery_codes::user_id.eq(user.id))
                .filter(recovery_codes::code_hash.eq(&hash))
                .set(recovery_codes::used_at.eq(chrono::Utc::now()))
                .execute(&mut conn)
                .await?;
            verified = true;
        }
    }
    if !verified {
        return Err(AppError::unauthorized());
    }
    drop(conn);
    auth::start_session(&state, jar, user).await
}

async fn member_profile(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
    Json(req): Json<UpdateMemberProfileRequest>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    crate::servers::role_in(&mut conn, server_id, user_id).await?;
    let me: Option<String> = members::table
        .filter(members::server_id.eq(server_id))
        .filter(members::user_id.eq(user_id))
        .select(members::role)
        .first(&mut conn)
        .await
        .optional()?;
    let role = crate::protocol::Role::parse(&me.unwrap_or_default());
    let allowed =
        crate::permissions::require_permission(&mut conn, server_id, user_id, CHANGE_NICKNAME)
            .await
            .is_ok()
            || role == Some(crate::protocol::Role::Owner)
            || role == Some(crate::protocol::Role::Admin);
    if !allowed {
        return Err(AppError::forbidden());
    }
    diesel::update(members::table)
        .filter(members::server_id.eq(server_id))
        .filter(members::user_id.eq(user_id))
        .set((
            members::nickname.eq(limit_text(&req.nickname, 64)?),
            members::avatar_url.eq(limit_text(&req.avatar_url, 512)?),
        ))
        .execute(&mut conn)
        .await?;
    crate::events::notify(
        &mut conn,
        &crate::events::Notice::RolesChanged { server_id },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn export(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> AppResult<Json<ExportResponse>> {
    let mut conn = state.pool.get().await?;
    let user = load_user(&mut conn, user_id).await?;
    #[derive(diesel::QueryableByName)]
    struct ServerRow {
        #[diesel(sql_type = diesel::sql_types::Int8)]
        id: i64,
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        description: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        rules: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        role: String,
    }
    #[derive(diesel::QueryableByName)]
    struct MessageRow {
        #[diesel(sql_type = diesel::sql_types::Int8)]
        id: i64,
        #[diesel(sql_type = diesel::sql_types::Int8)]
        channel_id: i64,
        #[diesel(sql_type = diesel::sql_types::Text)]
        content: String,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)]
        created_at: chrono::DateTime<chrono::Utc>,
    }
    let server_rows: Vec<ServerRow> = diesel::sql_query(
        "SELECT s.id, s.name, s.description, s.rules, mem.role FROM servers s JOIN members mem ON mem.server_id=s.id WHERE mem.user_id=$1",
    ).bind::<diesel::sql_types::BigInt, _>(user_id).load(&mut conn).await?;
    let servers: Vec<serde_json::Value> = server_rows.into_iter().map(|r| serde_json::json!({
        "id": r.id, "name": r.name, "description": r.description, "rules": r.rules, "role": r.role,
    })).collect();
    let message_rows: Vec<MessageRow> = diesel::sql_query(
        "SELECT id, channel_id, content, created_at FROM messages WHERE author_id=$1 ORDER BY id LIMIT 10000",
    ).bind::<diesel::sql_types::BigInt, _>(user_id).load(&mut conn).await?;
    let messages: Vec<serde_json::Value> = message_rows.into_iter().map(|r| serde_json::json!({
        "id": r.id, "channelId": r.channel_id, "content": r.content, "createdAt": r.created_at.timestamp_millis(),
    })).collect();
    let notes: Vec<Note> = notes_list(State(state.clone()), AuthUser(user_id)).await?.0;
    let data = serde_json::json!({ "user": user, "servers": servers, "messages": messages, "notes": notes });
    Ok(Json(ExportResponse {
        data: data.to_string(),
    }))
}
