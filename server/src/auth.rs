use std::sync::LazyLock;

use axum::{
    Json, Router,
    extract::{FromRequestParts, State},
    http::{StatusCode, request::Parts},
    routing::{get, post},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{Duration, Utc};
use diesel::{prelude::*, result::DatabaseErrorKind};
use diesel_async::RunQueryDsl;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    AppState,
    error::{AppError, AppResult},
    protocol::{AuthResponse, LoginRequest, RegisterRequest, User},
    schema::{refresh_tokens, users},
};

const ACCESS_TTL_SECS: i64 = 15 * 60;
const REFRESH_TTL_DAYS: i64 = 30;
const REFRESH_COOKIE: &str = "refresh_token";
const REFRESH_PATH: &str = "/api/auth";

pub struct JwtKeys {
    enc: EncodingKey,
    dec: DecodingKey,
}

impl JwtKeys {
    pub fn new(secret: &str) -> Self {
        Self {
            enc: EncodingKey::from_secret(secret.as_bytes()),
            dec: DecodingKey::from_secret(secret.as_bytes()),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    iat: i64,
    exp: i64,
}

pub fn issue_access(keys: &JwtKeys, user_id: i64) -> AppResult<String> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now,
        exp: now + ACCESS_TTL_SECS,
    };
    Ok(jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &keys.enc,
    )?)
}

/// Проверяет access-токен (подпись, алгоритм, срок) и возвращает id пользователя.
pub fn verify_access(keys: &JwtKeys, token: &str) -> Option<i64> {
    let data = jsonwebtoken::decode::<Claims>(token, &keys.dec, &Validation::new(Algorithm::HS256));
    data.ok()?.claims.sub.parse().ok()
}

/// Экстрактор: пользователь из заголовка `Authorization: Bearer <access>`.
pub struct AuthUser(pub i64);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> AppResult<Self> {
        parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .and_then(|t| verify_access(&state.jwt, t))
            .map(AuthUser)
            .ok_or_else(AppError::unauthorized)
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/logout", post(logout))
        .route("/api/me", get(me))
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = users, check_for_backend(diesel::pg::Pg))]
struct UserRow {
    id: i64,
    email: String,
    username: String,
    password_hash: String,
}

impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        User {
            id: r.id,
            email: r.email,
            username: r.username,
        }
    }
}

fn validate(req: &RegisterRequest) -> AppResult<()> {
    let email_ok = req.email.len() <= 254
        && req
            .email
            .split_once('@')
            .is_some_and(|(l, d)| !l.is_empty() && d.contains('.'));
    if !email_ok {
        return Err(AppError::bad_request("invalid email"));
    }
    let name_len = req.username.chars().count();
    if !(2..=32).contains(&name_len) {
        return Err(AppError::bad_request("username must be 2-32 characters"));
    }
    // bcrypt молча обрезает всё длиннее 72 байт, поэтому не принимаем такие пароли.
    if req.password.chars().count() < 8 || req.password.len() > 72 {
        return Err(AppError::bad_request("password must be 8-72 bytes"));
    }
    Ok(())
}

async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(mut req): Json<RegisterRequest>,
) -> AppResult<(StatusCode, CookieJar, Json<AuthResponse>)> {
    req.email = req.email.trim().to_lowercase();
    req.username = req.username.trim().to_string();
    validate(&req)?;

    let password = req.password;
    let hash =
        tokio::task::spawn_blocking(move || bcrypt::hash(password, bcrypt::DEFAULT_COST)).await??;

    let mut conn = state.pool.get().await?;
    let row: UserRow = diesel::insert_into(users::table)
        .values((
            users::email.eq(&req.email),
            users::username.eq(&req.username),
            users::password_hash.eq(&hash),
        ))
        .returning(UserRow::as_returning())
        .get_result(&mut conn)
        .await
        .map_err(|e| match e {
            diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, info) => {
                match info.constraint_name() {
                    Some("users_username_key") => AppError::conflict("username already taken"),
                    _ => AppError::conflict("email already registered"),
                }
            }
            e => e.into(),
        })?;

    let (jar, body) = start_session(&state, jar, row.into()).await?;
    Ok((StatusCode::CREATED, jar, body))
}

/// Хэш, с которым сравниваем пароль при неизвестном email, чтобы время ответа не выдавало,
/// существует ли аккаунт.
static DUMMY_HASH: LazyLock<String> =
    LazyLock::new(|| bcrypt::hash("dummy-password", bcrypt::DEFAULT_COST).expect("bcrypt works"));

async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> AppResult<(CookieJar, Json<AuthResponse>)> {
    let email = req.email.trim().to_lowercase();
    let mut conn = state.pool.get().await?;
    let row: Option<UserRow> = users::table
        .filter(users::email.eq(&email))
        .select(UserRow::as_select())
        .first(&mut conn)
        .await
        .optional()?;
    drop(conn);

    let hash = row
        .as_ref()
        .map_or_else(|| DUMMY_HASH.clone(), |r| r.password_hash.clone());
    let password = req.password;
    let ok = tokio::task::spawn_blocking(move || bcrypt::verify(password, &hash)).await??;

    match row {
        Some(row) if ok => start_session(&state, jar, row.into()).await,
        _ => Err(AppError(
            StatusCode::UNAUTHORIZED,
            "invalid email or password".into(),
        )),
    }
}

async fn refresh(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<(CookieJar, Json<AuthResponse>)> {
    let Some(token) = jar.get(REFRESH_COOKIE).map(|c| c.value().to_owned()) else {
        return Err(AppError::unauthorized());
    };

    let mut conn = state.pool.get().await?;
    // Ротация: токен отзывается атомарно, второй запрос с тем же токеном получит 401.
    let user_id: Option<i64> = diesel::update(refresh_tokens::table)
        .filter(refresh_tokens::token_hash.eq(sha256_hex(&token)))
        .filter(refresh_tokens::revoked_at.is_null())
        .filter(refresh_tokens::expires_at.gt(Utc::now()))
        .set(refresh_tokens::revoked_at.eq(Utc::now()))
        .returning(refresh_tokens::user_id)
        .get_result(&mut conn)
        .await
        .optional()?;
    let Some(user_id) = user_id else {
        return Err(AppError::unauthorized());
    };

    let row: UserRow = users::table
        .find(user_id)
        .select(UserRow::as_select())
        .first(&mut conn)
        .await?;
    drop(conn);
    start_session(&state, jar, row.into()).await
}

async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<(StatusCode, CookieJar)> {
    if let Some(token) = jar.get(REFRESH_COOKIE) {
        let mut conn = state.pool.get().await?;
        diesel::update(refresh_tokens::table)
            .filter(refresh_tokens::token_hash.eq(sha256_hex(token.value())))
            .filter(refresh_tokens::revoked_at.is_null())
            .set(refresh_tokens::revoked_at.eq(Utc::now()))
            .execute(&mut conn)
            .await?;
    }
    let jar = jar.remove(Cookie::build(REFRESH_COOKIE).path(REFRESH_PATH));
    Ok((StatusCode::NO_CONTENT, jar))
}

async fn me(State(state): State<AppState>, AuthUser(user_id): AuthUser) -> AppResult<Json<User>> {
    let mut conn = state.pool.get().await?;
    let row: Option<UserRow> = users::table
        .find(user_id)
        .select(UserRow::as_select())
        .first(&mut conn)
        .await
        .optional()?;
    row.map(|r| Json(r.into()))
        .ok_or_else(AppError::unauthorized)
}

/// Выдаёт новую пару: access в теле ответа, refresh в httpOnly cookie.
async fn start_session(
    state: &AppState,
    jar: CookieJar,
    user: User,
) -> AppResult<(CookieJar, Json<AuthResponse>)> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("OS RNG available");
    let refresh = hex::encode(bytes);

    let mut conn = state.pool.get().await?;
    diesel::insert_into(refresh_tokens::table)
        .values((
            refresh_tokens::user_id.eq(user.id),
            refresh_tokens::token_hash.eq(sha256_hex(&refresh)),
            refresh_tokens::expires_at.eq(Utc::now() + Duration::days(REFRESH_TTL_DAYS)),
        ))
        .execute(&mut conn)
        .await?;

    let cookie = Cookie::build((REFRESH_COOKIE, refresh))
        .http_only(true)
        .secure(true) // браузеры разрешают Secure-cookie и на http://localhost
        .same_site(SameSite::Strict)
        .path(REFRESH_PATH)
        .max_age(cookie::time::Duration::days(REFRESH_TTL_DAYS));

    let access_token = issue_access(&state.jwt, user.id)?;
    Ok((jar.add(cookie), Json(AuthResponse { access_token, user })))
}

fn sha256_hex(s: &str) -> String {
    hex::encode(Sha256::digest(s.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_token_roundtrip_and_rejects_foreign_or_garbage() {
        let keys = JwtKeys::new("test-secret-test-secret-test-secret");
        let token = issue_access(&keys, 42).unwrap();
        assert_eq!(verify_access(&keys, &token), Some(42));

        let other = JwtKeys::new("another-secret-another-secret-another");
        assert_eq!(verify_access(&other, &token), None);
        assert_eq!(verify_access(&keys, "garbage"), None);
    }

    #[test]
    fn validation_limits() {
        let req = |email: &str, username: &str, password: &str| RegisterRequest {
            email: email.into(),
            username: username.into(),
            password: password.into(),
        };
        assert!(validate(&req("a@b.co", "ok", "12345678")).is_ok());
        assert!(validate(&req("no-at-sign", "ok", "12345678")).is_err());
        assert!(validate(&req("a@b.co", "x", "12345678")).is_err());
        assert!(validate(&req("a@b.co", "ok", "short")).is_err());
        assert!(validate(&req("a@b.co", "ok", &"x".repeat(73))).is_err());
    }
}
