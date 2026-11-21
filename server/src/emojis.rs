//! Свои эмодзи сервера: PNG до 256 КБ, имя `[a-z0-9_]{2,32}`.
//! Картинки отдаются по подписанной ссылке — как вложения, но из байт в БД.

use axum::{
    Json, Router,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{StatusCode, header},
    response::Response,
    routing::get,
};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    events::{Notice, notify},
    protocol::{Emoji, Role},
    schema::server_emojis,
    servers::require_role,
};

/// Максимальный размер картинки: небольшая иконка, а не вложение.
pub const MAX_BYTES: usize = 256 * 1024;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/servers/{id}/emojis", get(list).post(upload))
        .route(
            "/api/servers/{id}/emojis/{emoji_id}",
            axum::routing::delete(remove),
        )
        .route("/api/emojis/{id}/{name}", get(image))
}

type EmojiPath = (i64, String);

#[derive(diesel::Queryable)]
struct Row {
    id: i64,
    server_id: i64,
    name: String,
}

#[derive(Deserialize)]
struct Signed {
    exp: i64,
    sig: String,
}

fn signature(key: &[u8], id: i64, exp: i64) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(format!("emoji:{id}:{exp}").as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn signed_url(key: &[u8], id: i64, name: &str) -> String {
    const DAY: i64 = 86_400;
    let exp = (Utc::now().timestamp() / DAY + 2) * DAY;
    format!(
        "/api/emojis/{id}/{name}.png?exp={exp}&sig={}",
        signature(key, id, exp)
    )
}

fn to_dto(key: &[u8], r: Row) -> Emoji {
    Emoji {
        url: signed_url(key, r.id, &r.name),
        id: r.id,
        server_id: r.server_id,
        name: r.name,
    }
}

/// Имя эмодзи: только нижний регистр, цифры и `_`, длина 2–32.
fn validate_name(raw: &str) -> AppResult<String> {
    let name = raw.trim().to_ascii_lowercase();
    let valid = (2..=32).contains(&name.chars().count())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !valid {
        return Err(AppError::bad_request(
            "emoji name must be 2-32 chars of a-z, 0-9, _",
        ));
    }
    Ok(name)
}

async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
) -> AppResult<Json<Vec<Emoji>>> {
    let mut conn = state.pool.get().await?;
    // Смотреть эмодзи может любой участник сервера.
    require_role(&mut conn, server_id, user_id, Role::Member).await?;
    let rows: Vec<Row> = server_emojis::table
        .filter(server_emojis::server_id.eq(server_id))
        .order((server_emojis::name.asc(), server_emojis::id.asc()))
        .select((
            server_emojis::id,
            server_emojis::server_id,
            server_emojis::name,
        ))
        .load(&mut conn)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| to_dto(&state.file_key, r))
            .collect(),
    ))
}

async fn upload(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<Emoji>)> {
    let mut conn = state.pool.get().await?;
    // Управлять набором может только админ сервера.
    require_role(&mut conn, server_id, user_id, Role::Admin).await?;

    let mut name = None;
    let mut data = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::bad_request("invalid multipart body"))?
    {
        match field.name() {
            Some("name") => {
                let text = field
                    .text()
                    .await
                    .map_err(|_| AppError::bad_request("invalid name field"))?;
                name = Some(validate_name(&text)?);
            }
            Some("file") => {
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|_| AppError::bad_request("invalid file field"))?;
                data = Some(bytes);
            }
            _ => {}
        }
    }
    let name = name.ok_or_else(|| AppError::bad_request("missing name"))?;
    let bytes = data.ok_or_else(|| AppError::bad_request("missing file"))?;
    if bytes.len() > MAX_BYTES {
        return Err(AppError(
            StatusCode::PAYLOAD_TOO_LARGE,
            "emoji is larger than 256 KB".into(),
        ));
    }
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(AppError::bad_request("emoji must be a PNG"));
    }
    let size = bytes.len() as i64;

    let exists: Option<i64> = server_emojis::table
        .filter(server_emojis::server_id.eq(server_id))
        .filter(server_emojis::name.eq(&name))
        .select(server_emojis::id)
        .first(&mut conn)
        .await
        .optional()?;
    if exists.is_some() {
        return Err(AppError::conflict("emoji with this name already exists"));
    }

    let id: i64 = diesel::insert_into(server_emojis::table)
        .values((
            server_emojis::server_id.eq(server_id),
            server_emojis::name.eq(&name),
            server_emojis::content_type.eq("image/png"),
            server_emojis::size_bytes.eq(size),
            server_emojis::data.eq(bytes.as_ref()),
            server_emojis::created_by.eq(user_id),
        ))
        .returning(server_emojis::id)
        .get_result(&mut conn)
        .await?;
    notify(&mut conn, &Notice::EmojisChanged { server_id }).await?;
    Ok((
        StatusCode::CREATED,
        Json(Emoji {
            url: signed_url(&state.file_key, id, &name),
            id,
            server_id,
            name,
        }),
    ))
}

async fn remove(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, emoji_id)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    require_role(&mut conn, server_id, user_id, Role::Admin).await?;
    let deleted = diesel::delete(server_emojis::table)
        .filter(server_emojis::id.eq(emoji_id))
        .filter(server_emojis::server_id.eq(server_id))
        .execute(&mut conn)
        .await?;
    if deleted == 0 {
        return Err(AppError::not_found("emoji"));
    }
    notify(&mut conn, &Notice::EmojisChanged { server_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn image(
    State(state): State<AppState>,
    Path((_id, _name)): Path<EmojiPath>,
    Query(signed): Query<Signed>,
) -> AppResult<Response> {
    let id = _id;
    // verify_slice сравнивает HMAC без утечки по времени.
    let mut mac =
        Hmac::<Sha256>::new_from_slice(&state.file_key).expect("HMAC accepts any key length");
    mac.update(format!("emoji:{id}:{}", signed.exp).as_bytes());
    let valid = hex::decode(&signed.sig).is_ok_and(|sig| mac.verify_slice(&sig).is_ok());
    if !valid || signed.exp < Utc::now().timestamp() {
        return Err(AppError(
            StatusCode::FORBIDDEN,
            "invalid or expired link".into(),
        ));
    }
    let mut conn = state.pool.get().await?;
    let row: Option<(Vec<u8>, i64)> = server_emojis::table
        .filter(server_emojis::id.eq(id))
        .select((server_emojis::data, server_emojis::size_bytes))
        .first(&mut conn)
        .await
        .optional()?;
    let (data, size) = row.ok_or_else(|| AppError::not_found("emoji"))?;
    let response = Response::builder()
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CONTENT_LENGTH, size)
        .header(header::CACHE_CONTROL, "public, max-age=86400")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(data))
        .map_err(|_| {
            AppError(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to build image".into(),
            )
        })?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_validated() {
        assert!(validate_name("party_blob").is_ok());
        assert!(validate_name("  CAT  ").unwrap() == "cat");
        assert!(validate_name("a").is_err());
        assert!(validate_name("bad-name").is_err());
        assert!(validate_name(&"x".repeat(33)).is_err());
    }

    #[test]
    fn signature_is_bound_to_id_and_expiry() {
        let a = signature(b"key-key-key-key", 1, 100);
        assert_eq!(a, signature(b"key-key-key-key", 1, 100));
        assert_ne!(a, signature(b"key-key-key-key", 2, 100));
        assert_ne!(a, signature(b"key-key-key-key", 1, 101));
    }
}
