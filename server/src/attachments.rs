//! Файлы в сообщениях: загрузка в S3, отдача по подписанной ссылке, уборка ненужных.
//!
//! Браузер грузит картинки через `<img src>`, без заголовка Authorization, поэтому доступ
//! проверяется, когда ссылку выдают (вместе с сообщением, только тем, у кого есть доступ к
//! каналу), а сама ссылка подписана HMAC и живёт 1–2 суток — как CDN-ссылки в Discord.

use std::{collections::HashMap, time::Duration};

use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::Response,
    routing::{get, post},
};
use chrono::Utc;
use diesel::{prelude::*, sql_types::Text};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use hmac::{Hmac, KeyInit, Mac};
use object_store::{ObjectStoreExt, PutPayload, aws::AmazonS3Builder, path::Path as ObjectPath};
use serde::Deserialize;
use sha2::Sha256;

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    permissions::{self, ATTACH_FILES},
    protocol::Attachment,
    schema::attachments,
    servers::channel_access,
};

pub const MAX_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_PER_MESSAGE: usize = 10;
const GC_EVERY: Duration = Duration::from_secs(60 * 60);

pub type Storage = object_store::aws::AmazonS3;

/// S3-совместимое хранилище из переменных окружения (локально — MinIO из docker-compose).
pub fn storage_from_env() -> anyhow::Result<Storage> {
    let var = |name: &str| std::env::var(name).map_err(|_| anyhow::anyhow!("{name} is not set"));
    let endpoint = var("S3_ENDPOINT")?;
    Ok(AmazonS3Builder::new()
        .with_allow_http(endpoint.starts_with("http://"))
        .with_endpoint(endpoint)
        .with_bucket_name(var("S3_BUCKET")?)
        .with_access_key_id(var("S3_ACCESS_KEY")?)
        .with_secret_access_key(var("S3_SECRET_KEY")?)
        .with_region(std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".into()))
        .with_virtual_hosted_style_request(false)
        .build()?)
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/channels/{id}/attachments",
            // Запас на заголовки multipart поверх самого файла.
            post(upload).layer(DefaultBodyLimit::max(MAX_BYTES + 64 * 1024)),
        )
        .route("/api/attachments/{id}/{filename}", get(download))
}

/// Тип определяем по первым байтам, а не по имени или заголовку клиента.
pub fn detect_type(bytes: &[u8]) -> Option<&'static str> {
    let starts = |sig: &[u8]| bytes.starts_with(sig);
    if starts(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if starts(b"\xFF\xD8\xFF") {
        Some("image/jpeg")
    } else if starts(b"GIF87a") || starts(b"GIF89a") {
        Some("image/gif")
    } else if starts(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("image/webp")
    } else if starts(b"%PDF-") {
        Some("application/pdf")
    } else if starts(b"PK\x03\x04") || starts(b"PK\x05\x06") {
        Some("application/zip")
    } else {
        None
    }
}

/// Только имя файла без пути и управляющих символов, не длиннее 100 символов.
pub fn sanitize_filename(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let clean: String = base
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '"' | '<' | '>' | ':' | '|' | '?' | '*'))
        .collect::<String>()
        .trim()
        .trim_start_matches('.')
        .chars()
        .take(100)
        .collect();
    if clean.is_empty() {
        "file".into()
    } else {
        clean
    }
}

fn percent_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn signature(key: &[u8], id: i64, exp: i64) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(format!("{id}:{exp}").as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Срок округляем до суток: ссылка на файл стабильна в течение дня и кэшируется браузером.
pub fn signed_url(key: &[u8], id: i64, filename: &str) -> String {
    const DAY: i64 = 86_400;
    let exp = (Utc::now().timestamp() / DAY + 2) * DAY;
    format!(
        "/api/attachments/{id}/{}?exp={exp}&sig={}",
        percent_encode(filename),
        signature(key, id, exp)
    )
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = attachments, check_for_backend(diesel::pg::Pg))]
struct Row {
    id: i64,
    message_id: Option<i64>,
    filename: String,
    content_type: String,
    size_bytes: i64,
}

fn to_dto(key: &[u8], r: Row) -> Attachment {
    Attachment {
        url: signed_url(key, r.id, &r.filename),
        id: r.id,
        filename: r.filename,
        content_type: r.content_type,
        size_bytes: r.size_bytes,
    }
}

/// Вложения сообщений одним запросом: message_id → список по порядку загрузки.
pub async fn for_messages(
    conn: &mut AsyncPgConnection,
    key: &[u8],
    message_ids: &[i64],
) -> AppResult<HashMap<i64, Vec<Attachment>>> {
    if message_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<Row> = attachments::table
        .filter(attachments::message_id.eq_any(message_ids))
        .order(attachments::id.asc())
        .select(Row::as_select())
        .load(conn)
        .await?;
    let mut map: HashMap<i64, Vec<Attachment>> = HashMap::new();
    for r in rows {
        if let Some(mid) = r.message_id {
            map.entry(mid).or_default().push(to_dto(key, r));
        }
    }
    Ok(map)
}

async fn upload(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<Attachment>)> {
    let mut conn = state.pool.get().await?;
    let access = channel_access(&mut conn, channel_id, user_id).await?;
    if let Some(server_id) = access.server_id {
        permissions::require_channel_permission(
            &mut conn,
            server_id,
            user_id,
            channel_id,
            ATTACH_FILES,
        )
        .await?;
    }
    drop(conn); // не держим соединение, пока идёт загрузка

    let too_large = || {
        AppError(
            StatusCode::PAYLOAD_TOO_LARGE,
            "file is larger than 10 MB".into(),
        )
    };
    let (filename, bytes) = loop {
        let field = multipart
            .next_field()
            .await
            .map_err(|e| match e.status() {
                StatusCode::PAYLOAD_TOO_LARGE => too_large(),
                _ => AppError::bad_request("invalid multipart body"),
            })?
            .ok_or_else(|| AppError::bad_request("missing file field"))?;
        if field.name() == Some("file") {
            let name = sanitize_filename(field.file_name().unwrap_or(""));
            let bytes = field.bytes().await.map_err(|e| match e.status() {
                StatusCode::PAYLOAD_TOO_LARGE => too_large(),
                _ => AppError::bad_request("invalid multipart body"),
            })?;
            break (name, bytes);
        }
    };
    if bytes.len() > MAX_BYTES {
        return Err(too_large());
    }
    let content_type =
        detect_type(&bytes).ok_or_else(|| AppError::bad_request("unsupported file type"))?;

    let mut rnd = [0u8; 16];
    getrandom::fill(&mut rnd).expect("OS RNG available");
    let storage_key = format!("attachments/{}", hex::encode(rnd));
    let size = bytes.len() as i64;
    state
        .storage
        .put(
            &ObjectPath::from(storage_key.as_str()),
            PutPayload::from(bytes),
        )
        .await?;

    let mut conn = state.pool.get().await?;
    let id: i64 = diesel::insert_into(attachments::table)
        .values((
            attachments::channel_id.eq(channel_id),
            attachments::uploader_id.eq(user_id),
            attachments::filename.eq(&filename),
            attachments::content_type.eq(content_type),
            attachments::size_bytes.eq(size),
            attachments::storage_key.eq(&storage_key),
        ))
        .returning(attachments::id)
        .get_result(&mut conn)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(Attachment {
            url: signed_url(&state.file_key, id, &filename),
            id,
            filename,
            content_type: content_type.into(),
            size_bytes: size,
        }),
    ))
}

#[derive(Deserialize)]
struct Signed {
    exp: i64,
    sig: String,
}

async fn download(
    State(state): State<AppState>,
    Path((id, _filename)): Path<(i64, String)>,
    Query(signed): Query<Signed>,
) -> AppResult<Response> {
    // verify_slice сравнивает HMAC без утечки по времени.
    let mut mac =
        Hmac::<Sha256>::new_from_slice(&state.file_key).expect("HMAC accepts any key length");
    mac.update(format!("{id}:{}", signed.exp).as_bytes());
    let valid = hex::decode(&signed.sig).is_ok_and(|sig| mac.verify_slice(&sig).is_ok());
    if !valid || signed.exp < Utc::now().timestamp() {
        return Err(AppError(
            StatusCode::FORBIDDEN,
            "invalid or expired link".into(),
        ));
    }

    let mut conn = state.pool.get().await?;
    // Канал удалён — файл недоступен сразу, сам объект уберёт уборка.
    let row: Option<(String, String, i64, String)> = attachments::table
        .filter(attachments::id.eq(id))
        .filter(attachments::channel_id.is_not_null())
        .select((
            attachments::storage_key,
            attachments::content_type,
            attachments::size_bytes,
            attachments::filename,
        ))
        .first(&mut conn)
        .await
        .optional()?;
    drop(conn);
    let (storage_key, content_type, size, filename) =
        row.ok_or_else(|| AppError::not_found("attachment"))?;

    let object = state
        .storage
        .get(&ObjectPath::from(storage_key.as_str()))
        .await?;
    let disposition = if content_type.starts_with("image/") {
        "inline"
    } else {
        "attachment"
    };
    let response = Response::builder()
        .header(header::CONTENT_TYPE, &content_type)
        .header(header::CONTENT_LENGTH, size)
        .header(
            header::CONTENT_DISPOSITION,
            format!(
                "{disposition}; filename*=UTF-8''{}",
                percent_encode(&filename)
            ),
        )
        .header(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        )
        .header(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'none'; sandbox"),
        )
        .header(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, max-age=86400"),
        )
        .body(Body::from_stream(object.into_stream()))
        .map_err(|e| AppError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(response)
}

/// Удаляет объекты из S3; ошибки только логируем: строк в БД уже нет, файл просто станет мусором.
pub async fn delete_objects(state: &AppState, keys: &[String]) {
    for key in keys {
        if let Err(e) = state.storage.delete(&ObjectPath::from(key.as_str())).await {
            tracing::warn!("delete {key} from storage failed: {e}");
        }
    }
}

/// Прикрепляет загруженные пользователем файлы к сообщению. Только свои, из того же канала
/// и ещё не прикреплённые; иначе ошибка, и вызывающий откатывает транзакцию.
pub async fn attach(
    conn: &mut AsyncPgConnection,
    ids: &[i64],
    message_id: i64,
    channel_id: i64,
    user_id: i64,
) -> AppResult<()> {
    let updated = diesel::update(
        attachments::table
            .filter(attachments::id.eq_any(ids))
            .filter(attachments::uploader_id.eq(user_id))
            .filter(attachments::channel_id.eq(channel_id))
            .filter(attachments::message_id.is_null()),
    )
    .set(attachments::message_id.eq(message_id))
    .execute(conn)
    .await?;
    if updated != ids.len() {
        return Err(AppError::bad_request("invalid attachments"));
    }
    Ok(())
}

/// Вложения сообщения удаляются вместе с ним: строки здесь, объекты — после коммита.
pub async fn detach_for_delete(
    conn: &mut AsyncPgConnection,
    message_id: i64,
) -> AppResult<Vec<String>> {
    Ok(
        diesel::delete(attachments::table.filter(attachments::message_id.eq(message_id)))
            .returning(attachments::storage_key)
            .get_results(conn)
            .await?,
    )
}

#[derive(diesel::QueryableByName)]
struct Key {
    #[diesel(sql_type = Text)]
    storage_key: String,
}

/// Убирает файлы удалённых каналов и так и не отправленные загрузки старше суток.
/// DELETE … RETURNING: каждый файл удаляет ровно один экземпляр.
pub async fn gc(state: &AppState) -> AppResult<usize> {
    let mut conn = state.pool.get().await?;
    let keys: Vec<Key> = diesel::sql_query(
        "DELETE FROM attachments
         WHERE channel_id IS NULL OR (message_id IS NULL AND created_at < now() - interval '1 day')
         RETURNING storage_key",
    )
    .load(&mut conn)
    .await?;
    drop(conn);
    let keys: Vec<String> = keys.into_iter().map(|k| k.storage_key).collect();
    delete_objects(state, &keys).await;
    Ok(keys.len())
}

pub fn spawn_gc(state: AppState) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(GC_EVERY);
        loop {
            tick.tick().await;
            match gc(&state).await {
                Ok(0) => {}
                Ok(n) => tracing::info!("attachment gc removed {n} files"),
                Err(e) => tracing::error!("attachment gc failed: {e:?}"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_types_by_content() {
        assert_eq!(detect_type(b"\x89PNG\r\n\x1a\nrest"), Some("image/png"));
        assert_eq!(detect_type(b"\xFF\xD8\xFF\xE0"), Some("image/jpeg"));
        assert_eq!(detect_type(b"GIF89a..."), Some("image/gif"));
        assert_eq!(detect_type(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
        assert_eq!(detect_type(b"%PDF-1.7"), Some("application/pdf"));
        assert_eq!(detect_type(b"PK\x03\x04"), Some("application/zip"));
        assert_eq!(detect_type(b"MZ\x90\0"), None, "exe");
        assert_eq!(detect_type(b"<svg onload=alert(1)>"), None, "svg/html");
        assert_eq!(detect_type(b"RIFF\0\0\0\0WAVE"), None, "wav is not webp");
    }

    #[test]
    fn filenames_are_sanitized() {
        assert_eq!(sanitize_filename("../../etc/passwd"), "passwd");
        assert_eq!(sanitize_filename("C:\\Users\\me\\отчёт.pdf"), "отчёт.pdf");
        assert_eq!(sanitize_filename("a\"b<c>\n.png"), "abc.png");
        assert_eq!(sanitize_filename("...hidden"), "hidden");
        assert_eq!(sanitize_filename(""), "file");
        assert_eq!(sanitize_filename(&"я".repeat(300)).chars().count(), 100);
    }

    #[test]
    fn signatures_bind_id_and_expiry() {
        let key = b"k";
        assert_eq!(signature(key, 1, 100), signature(key, 1, 100));
        assert_ne!(signature(key, 1, 100), signature(key, 2, 100));
        assert_ne!(signature(key, 1, 100), signature(key, 1, 101));
        assert_ne!(signature(key, 1, 100), signature(b"other", 1, 100));
        assert!(signed_url(key, 5, "мой файл.png").starts_with("/api/attachments/5/%D0%BC"));
    }
}
