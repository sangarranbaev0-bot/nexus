//! Apps/Bots/Integrations: вебхуки каналов + регистрация OAuth-приложений.
//! Вебхуки позволяют отправлять сообщения в канал по токену (без JWT пользователя).
//! OAuth-приложения регистрируются пользователями (упрощённая схема: client_id + client_secret).

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use chrono::Utc;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text, Array};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    events::{Notice, notify},
    protocol::{ChannelKind, CreateOAuthAppRequest, CreateOAuthAppResponse, CreateWebhookRequest, OAuthApp, Webhook},
    schema::{channels, messages, oauth_apps, users, webhooks},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/channels/{id}/webhooks", get(list_webhooks).post(create_webhook))
        .route("/api/channels/{id}/webhooks/{wid}", delete(delete_webhook))
        .route("/api/webhooks/{token}", post(webhook_send))
        .route("/api/oauth/apps", get(list_apps).post(create_app))
        .route("/api/oauth/apps/{id}", delete(delete_app))
}

#[derive(diesel::QueryableByName)]
struct WebhookRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    avatar_url: String,
    #[diesel(sql_type = BigInt)]
    channel_id: i64,
    #[diesel(sql_type = BigInt)]
    created_at: i64,
}

#[derive(diesel::QueryableByName)]
struct OauthAppRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
    #[diesel(sql_type = BigInt)]
    owner_id: i64,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    client_id: String,
    #[diesel(sql_type = Array<Text>)]
    scopes: Vec<String>,
    #[diesel(sql_type = Array<Text>)]
    redirect_uris: Vec<String>,
    #[diesel(sql_type = BigInt)]
    created_at: i64,
}

/// Проверить, что канал принадлежит серверу, и вернуть server_id.
async fn channel_server(
    conn: &mut AsyncPgConnection,
    channel_id: i64,
) -> AppResult<Option<i64>> {
    let (server_id, kind): (Option<i64>, String) = channels::table
        .find(channel_id)
        .select((channels::server_id, channels::kind))
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| AppError::not_found("channel"))?;
    if ChannelKind::parse(&kind) == ChannelKind::Voice {
        return Err(AppError::bad_request("voice channels have no webhooks"));
    }
    Ok(server_id)
}

async fn require_server_perm(
    conn: &mut AsyncPgConnection,
    server_id: i64,
    user_id: i64,
    perm: i64,
) -> AppResult<()> {
    crate::permissions::require_permission(conn, server_id, user_id, perm).await?;
    Ok(())
}

/// Список вебхуков канала. Токены скрываем.
async fn list_webhooks(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
) -> AppResult<Json<Vec<Webhook>>> {
    let mut conn = state.pool.get().await?;
    let server_id = channel_server(&mut conn, channel_id).await?;
    if let Some(sid) = server_id {
        require_server_perm(&mut conn, sid, user_id, crate::permissions::MANAGE_WEBHOOKS).await?;
    }
    let rows: Vec<WebhookRow> = diesel::sql_query(
        r#"SELECT id, name, avatar_url, channel_id, EXTRACT(EPOCH FROM created_at)::BIGINT AS created_at
           FROM webhooks WHERE channel_id = $1 ORDER BY id ASC"#,
    )
    .bind::<BigInt, _>(channel_id)
    .load(&mut conn)
    .await?;
    Ok(Json(rows.into_iter().map(|r| Webhook {
        id: r.id,
        name: r.name,
        avatar_url: r.avatar_url,
        channel_id: r.channel_id,
        token: None,
        created_at: r.created_at,
    }).collect()))
}

/// Создать вебхук. Возвращает токен только один раз.
async fn create_webhook(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
    Json(req): Json<CreateWebhookRequest>,
) -> AppResult<(StatusCode, Json<Webhook>)> {
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(AppError::bad_request("webhook name must be 1-100 characters"));
    }
    let mut conn = state.pool.get().await?;
    let server_id = channel_server(&mut conn, channel_id).await?;
    if let Some(sid) = server_id {
        require_server_perm(&mut conn, sid, user_id, crate::permissions::MANAGE_WEBHOOKS).await?;
    }
    let token = format!("wh_{}", hex(&rand_token()));
    let avatar = req.avatar_url.clone().unwrap_or_default();
    let id: i64 = diesel::insert_into(webhooks::table)
        .values((
            webhooks::channel_id.eq(channel_id),
            webhooks::name.eq(name),
            webhooks::avatar_url.eq(avatar.clone()),
            webhooks::token.eq(&token),
            webhooks::created_by.eq(user_id),
        ))
        .returning(webhooks::id)
        .get_result(&mut conn)
        .await?;
    if let Some(sid) = server_id {
        notify(&mut conn, &Notice::ChannelsChanged { server_id: sid }).await?;
    }
    let created_at = Utc::now().timestamp();
    Ok((StatusCode::CREATED, Json(Webhook {
        id,
        name: name.to_string(),
        avatar_url: avatar,
        channel_id,
        token: Some(token),
        created_at,
    })))
}

async fn delete_webhook(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((channel_id, wid)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let server_id = channel_server(&mut conn, channel_id).await?;
    if let Some(sid) = server_id {
        require_server_perm(&mut conn, sid, user_id, crate::permissions::MANAGE_WEBHOOKS).await?;
    }
    let deleted = diesel::delete(
        webhooks::table.find(wid).filter(webhooks::channel_id.eq(channel_id)),
    )
    .execute(&mut conn)
    .await?;
    if deleted == 0 {
        return Err(AppError::not_found("webhook"));
    }
    if let Some(sid) = server_id {
        notify(&mut conn, &Notice::ChannelsChanged { server_id: sid }).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Отправка сообщения вебхуком (по токену, без JWT).
#[derive(Deserialize)]
#[allow(dead_code)]
struct WebhookPayload {
    #[serde(default)]
    content: String,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
}

async fn webhook_send(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(payload): Json<WebhookPayload>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let mut conn = state.pool.get().await?;
    let (channel_id, webhook_name, avatar): (i64, String, String) = webhooks::table
        .filter(webhooks::token.eq(&token))
        .select((webhooks::channel_id, webhooks::name, webhooks::avatar_url))
        .first(&mut conn)
        .await
        .optional()?
        .ok_or_else(|| AppError::not_found("webhook"))?;
    let content = payload.content.trim();
    if content.is_empty() || content.chars().count() > 2000 {
        return Err(AppError::bad_request("content must be 1-2000 characters"));
    }
    let author_id = ensure_webhook_user(&mut conn).await?;
    let id: i64 = diesel::insert_into(messages::table)
        .values((
            messages::channel_id.eq(channel_id),
            messages::author_id.eq(author_id),
            messages::content.eq(content),
            messages::has_embed.eq(false),
        ))
        .returning(messages::id)
        .get_result(&mut conn)
        .await?;
    let server_id: Option<i64> = channels::table
        .find(channel_id)
        .select(channels::server_id)
        .first(&mut conn)
        .await?;
    if let Some(sid) = server_id {
        notify(&mut conn, &Notice::MessageCreated { server_id: Some(sid), message_id: id }).await?;
    }
    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "id": id,
        "webhook": webhook_name,
        "avatar_url": avatar,
        "content": content,
    }))))
}

/// Вебхуки пишут от имени специального системного пользователя (создаётся один раз).
const WEBHOOK_USERNAME: &str = "__nexus_webhook__";
const WEBHOOK_EMAIL: &str = "webhook@internal.nexus";

async fn ensure_webhook_user(conn: &mut AsyncPgConnection) -> AppResult<i64> {
    let existing: Option<i64> = users::table
        .filter(users::username.eq(WEBHOOK_USERNAME))
        .select(users::id)
        .first(conn)
        .await
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    diesel::insert_into(users::table)
        .values((
            users::email.eq(WEBHOOK_EMAIL),
            users::username.eq(WEBHOOK_USERNAME),
            users::password_hash.eq("!internal-webhook"),
        ))
        .on_conflict_do_nothing()
        .returning(users::id)
        .get_result(conn)
        .await
        .optional()?
        .ok_or_else(|| AppError::conflict("webhook user unavailable"))
}

/// Список OAuth-приложений пользователя.
async fn list_apps(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> AppResult<Json<Vec<OAuthApp>>> {
    let mut conn = state.pool.get().await?;
    let rows: Vec<OauthAppRow> = diesel::sql_query(
        r#"SELECT id, owner_id, name, client_id, scopes, redirect_uris,
                  EXTRACT(EPOCH FROM created_at)::BIGINT AS created_at
           FROM oauth_apps WHERE owner_id = $1 ORDER BY id ASC"#,
    )
    .bind::<BigInt, _>(user_id)
    .load(&mut conn)
    .await?;
    Ok(Json(rows.into_iter().map(|r| OAuthApp {
        id: r.id,
        owner_id: r.owner_id,
        name: r.name,
        client_id: r.client_id,
        scopes: r.scopes,
        redirect_uris: r.redirect_uris,
        created_at: r.created_at,
    }).collect()))
}

/// Зарегистрировать OAuth-приложение. Возвращает client_secret один раз.
async fn create_app(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<CreateOAuthAppRequest>,
) -> AppResult<(StatusCode, Json<CreateOAuthAppResponse>)> {
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(AppError::bad_request("app name must be 1-100 characters"));
    }
    let client_id = format!("app_{}", hex(&rand_token()));
    let client_secret = hex(&rand_token());
    let secret_hash = hex(&Sha256::digest(client_secret.as_bytes()));
    let mut conn = state.pool.get().await?;
    let id: i64 = diesel::insert_into(oauth_apps::table)
        .values((
            oauth_apps::owner_id.eq(user_id),
            oauth_apps::name.eq(name),
            oauth_apps::client_id.eq(&client_id),
            oauth_apps::client_secret_hash.eq(&secret_hash),
            oauth_apps::scopes.eq(req.scopes.clone()),
            oauth_apps::redirect_uris.eq(req.redirect_uris.clone()),
        ))
        .returning(oauth_apps::id)
        .get_result(&mut conn)
        .await?;
    Ok((StatusCode::CREATED, Json(CreateOAuthAppResponse {
        id,
        owner_id: user_id,
        name: name.to_string(),
        client_id,
        client_secret,
        scopes: req.scopes,
        redirect_uris: req.redirect_uris,
        created_at: Utc::now().timestamp(),
    })))
}

async fn delete_app(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let deleted = diesel::delete(
        oauth_apps::table.find(id).filter(oauth_apps::owner_id.eq(user_id)),
    )
    .execute(&mut conn)
    .await?;
    if deleted == 0 {
        return Err(AppError::not_found("app"));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Случайные 16 байт в hex.
fn rand_token() -> [u8; 16] {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("OS RNG available");
    bytes
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
