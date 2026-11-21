use axum::{
    Json, Router,
    extract::{Multipart, Path, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use diesel::{sql_types::{Array, BigInt, Int4, Text, Timestamptz}, prelude::*};
use diesel_async::RunQueryDsl;
use object_store::{ObjectStoreExt, PutPayload, path::Path as ObjectPath};
use crate::{
    AppState, auth::AuthUser, error::{AppError, AppResult}, permissions,
    protocol::{StickerPack, Sticker, CreateStickerPackRequest, CreateStickerRequest},
    schema::stickers,
    attachments::{detect_type, sanitize_filename},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/servers/{id}/stickers/packages",
            get(list_packs).post(create_pack),
        )
        .route(
            "/api/servers/{id}/stickers/packages/{pack_id}",
            get(get_pack).delete(delete_pack),
        )
        .route(
            "/api/servers/{id}/stickers/packages/{pack_id}/stickers",
            get(list_pack_stickers).post(upload_sticker_multipart),
        )
        .route(
            "/api/servers/{id}/stickers/by_metadata",
            post(create_sticker),
        )
        .route(
            "/api/servers/{id}/stickers/{sticker_id}",
            delete(delete_sticker),
        )
        .route(
            "/api/channels/{id}/stickers",
            get(list_channel_stickers),
        )
}

#[derive(diesel::QueryableByName)]
struct StickerPackRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
    #[diesel(sql_type = BigInt)]
    server_id: i64,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    description: String,
    #[diesel(sql_type = BigInt)]
    created_by: i64,
    #[diesel(sql_type = Timestamptz)]
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(diesel::QueryableByName)]
struct StickerRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
    #[diesel(sql_type = BigInt)]
    pack_id: i64,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    emoji: String,
    #[diesel(sql_type = Text)]
    content_type: String,
    #[diesel(sql_type = BigInt)]
    size_bytes: i64,
    #[diesel(sql_type = Text)]
    storage_key: String,
    #[diesel(sql_type = Array<Text>)]
    tags: Vec<String>,
    #[diesel(sql_type = Int4)]
    position: i32,
    #[diesel(sql_type = Timestamptz)]
    created_at: chrono::DateTime<chrono::Utc>,
}

async fn list_packs(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
) -> AppResult<Json<Vec<StickerPack>>> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, permissions::VIEW_CHANNEL).await?;

    let rows: Vec<StickerPackRow> = diesel::sql_query(
        "SELECT id, server_id, name, description, created_by, created_at FROM sticker_packs WHERE server_id=$1 ORDER BY id",
    )
    .bind::<BigInt, _>(server_id)
    .load(&mut conn)
    .await?;

    Ok(Json(rows.into_iter().map(|r| StickerPack {
        id: r.id,
        server_id: r.server_id,
        name: r.name,
        description: r.description,
        created_by: r.created_by,
        created_at: r.created_at.timestamp_millis(),
    }).collect()))
}

async fn create_pack(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
    Json(req): Json<CreateStickerPackRequest>,
) -> AppResult<(StatusCode, Json<StickerPack>)> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, permissions::MANAGE_ROLES).await?;

    #[derive(diesel::QueryableByName)]
    struct IdR {
        #[diesel(sql_type = BigInt)]
        id: i64,
    }

    let id_row: IdR = diesel::sql_query(
        "INSERT INTO sticker_packs (server_id, name, description, created_by) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind::<BigInt, _>(server_id)
    .bind::<Text, _>(req.name)
    .bind::<Text, _>(req.description.unwrap_or_default())
    .bind::<BigInt, _>(user_id)
    .get_result(&mut conn)
    .await?;

    let row: StickerPackRow = diesel::sql_query(
        "SELECT id, server_id, name, description, created_by, created_at FROM sticker_packs WHERE id=$1",
    )
    .bind::<BigInt, _>(id_row.id)
    .get_result(&mut conn)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(StickerPack {
            id: row.id,
            server_id: row.server_id,
            name: row.name,
            description: row.description,
            created_by: row.created_by,
            created_at: row.created_at.timestamp_millis(),
        }),
    ))
}

async fn get_pack(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, pack_id)): Path<(i64, i64)>,
) -> AppResult<Json<StickerPack>> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, permissions::VIEW_CHANNEL).await?;

    let row: StickerPackRow = diesel::sql_query(
        "SELECT id, server_id, name, description, created_by, created_at FROM sticker_packs WHERE id=$1 AND server_id=$2",
    )
    .bind::<BigInt, _>(pack_id)
    .bind::<BigInt, _>(server_id)
    .get_result(&mut conn)
    .await?;

    Ok(Json(StickerPack {
        id: row.id,
        server_id: row.server_id,
        name: row.name,
        description: row.description,
        created_by: row.created_by,
        created_at: row.created_at.timestamp_millis(),
    }))
}

async fn delete_pack(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, pack_id)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, permissions::MANAGE_ROLES).await?;

    diesel::sql_query("DELETE FROM sticker_packs WHERE id=$1 AND server_id=$2")
        .bind::<BigInt, _>(pack_id)
        .bind::<BigInt, _>(server_id)
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn create_sticker(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, pack_id)): Path<(i64, i64)>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateStickerRequest>,
) -> AppResult<(StatusCode, Json<Sticker>)> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, permissions::MANAGE_ROLES).await?;

    crate::rate_limit::sticker_create(&headers).map_err(|(code, msg)| AppError(code, msg))?;

    #[derive(diesel::QueryableByName)]
    struct IdR {
        #[diesel(sql_type = BigInt)]
        id: i64,
    }

    let id_row: IdR = diesel::sql_query(
        "INSERT INTO stickers (pack_id, name, emoji, content_type, size_bytes, storage_key, tags, position) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
    )
    .bind::<BigInt, _>(pack_id)
    .bind::<Text, _>(req.name)
    .bind::<Text, _>(req.emoji)
    .bind::<Text, _>(req.content_type)
    .bind::<BigInt, _>(req.size_bytes)
    .bind::<Text, _>(req.storage_key)
    .bind::<Array<Text>, _>(req.tags)
    .bind::<Int4, _>(req.position)
    .get_result(&mut conn)
    .await?;

    let row: StickerRow = diesel::sql_query(
        "SELECT id, pack_id, name, emoji, content_type, size_bytes, storage_key, tags, position, created_at FROM stickers WHERE id=$1",
    )
    .bind::<BigInt, _>(id_row.id)
    .get_result(&mut conn)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(Sticker {
            id: row.id,
            pack_id: row.pack_id,
            name: row.name,
            emoji: row.emoji,
            content_type: row.content_type,
            size_bytes: row.size_bytes,
            storage_key: row.storage_key,
            tags: row.tags,
            position: row.position as i64,
            created_at: row.created_at.timestamp_millis(),
        }),
    ))
}

async fn delete_sticker(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((_server_id, sticker_id)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;

    #[derive(diesel::QueryableByName)]
    struct PackServerRow {
        #[diesel(sql_type = BigInt)]
        server_id: i64,
    }

    let pack_row: PackServerRow = diesel::sql_query(
        "SELECT sp.server_id FROM stickers s JOIN sticker_packs sp ON s.pack_id = sp.id WHERE s.id=$1",
    )
    .bind::<BigInt, _>(sticker_id)
    .get_result(&mut conn)
    .await?;

    permissions::require_permission(&mut conn, pack_row.server_id, user_id, permissions::MANAGE_ROLES).await?;

    diesel::sql_query("DELETE FROM stickers WHERE id=$1")
        .bind::<BigInt, _>(sticker_id)
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn list_channel_stickers(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
) -> AppResult<Json<Vec<Sticker>>> {
    let mut conn = state.pool.get().await?;

    #[derive(diesel::QueryableByName)]
    struct ChannelServerRow {
        #[diesel(sql_type = BigInt)]
        server_id: i64,
    }

    let server_row: ChannelServerRow = diesel::sql_query(
        "SELECT server_id FROM channels WHERE id=$1",
    )
    .bind::<BigInt, _>(channel_id)
    .get_result(&mut conn)
    .await?;

    permissions::require_permission(&mut conn, server_row.server_id, user_id, permissions::VIEW_CHANNEL).await?;

    let rows: Vec<StickerRow> = diesel::sql_query(
        "SELECT s.id, s.pack_id, s.name, s.emoji, s.content_type, s.size_bytes, s.storage_key, s.tags, s.position, s.created_at FROM stickers s JOIN sticker_packs sp ON s.pack_id = sp.id WHERE sp.server_id=$1 ORDER BY s.position, s.id",
    )
    .bind::<BigInt, _>(server_row.server_id)
    .load(&mut conn)
    .await?;

    Ok(Json(rows.into_iter().map(|r| Sticker {
        id: r.id,
        pack_id: r.pack_id,
        name: r.name,
        emoji: r.emoji,
        content_type: r.content_type,
        size_bytes: r.size_bytes,
        storage_key: r.storage_key,
        tags: r.tags,
        position: r.position as i64,
        created_at: r.created_at.timestamp_millis(),
    }).collect()))
}

async fn upload_sticker_multipart(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, pack_id)): Path<(i64, i64)>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<Sticker>)> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, permissions::MANAGE_ROLES).await?;

    // Rate limit sticker creation: 5 per minute per IP/User-Agent
    crate::rate_limit::sticker_create(&headers).map_err(|(code, msg)| AppError(code, msg))?;

    let too_large = || {
        AppError(
            StatusCode::PAYLOAD_TOO_LARGE,
            "file is larger than 10 MB".into(),
        )
    };
    let mut filename = String::new();
    let mut bytes: Option<Vec<u8>> = None;
    let mut sticker_name = String::new();
    let mut emoji_name = String::new();
    let mut tags_str = String::new();
    while let Some(field) = multipart.next_field().await.map_err(|_| AppError::bad_request("invalid multipart body"))? {
        match field.name() {
            Some("file") => {
                filename = sanitize_filename(field.file_name().unwrap_or(""));
                bytes = Some(field.bytes().await.map_err(|_| AppError::bad_request("invalid file"))?.to_vec());
            }
            Some("name") => {
                sticker_name = field.text().await.unwrap_or_default();
            }
            Some("emoji") => {
                emoji_name = field.text().await.unwrap_or_default();
            }
            Some("tags") => {
                tags_str = field.text().await.unwrap_or_default();
            }
            _ => {}
        }
    }
    let bytes = bytes.ok_or_else(|| AppError::bad_request("missing file field"))?;
    if sticker_name.is_empty() {
        sticker_name = filename.clone();
    }
    if emoji_name.is_empty() {
        emoji_name = sticker_name.clone();
    }
    let tags: Vec<String> = if tags_str.is_empty() {
        vec![]
    } else {
        tags_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    };
    
    if bytes.len() > 10 * 1024 * 1024 {
        return Err(too_large());
    }
    let content_type =
        detect_type(&bytes).ok_or_else(|| AppError::bad_request("unsupported file type"))?;

    let mut rnd = [0u8; 16];
    getrandom::fill(&mut rnd).expect("OS RNG available");
    let storage_key = format!("stickers/{}", hex::encode(rnd));
    let size = bytes.len() as i64;
    state
        .storage
        .put(
            &ObjectPath::from(storage_key.as_str()),
            PutPayload::from(bytes),
        )
        .await?;

    let mut conn = state.pool.get().await?;
    let id: i64 = diesel::insert_into(stickers::table)
        .values((
            stickers::pack_id.eq(pack_id),
            stickers::name.eq(&sticker_name),
            stickers::emoji.eq(&emoji_name),
            stickers::content_type.eq(content_type),
            stickers::size_bytes.eq(size),
            stickers::storage_key.eq(&storage_key),
            stickers::tags.eq(tags),
            stickers::position.eq(0),
        ))
        .returning(stickers::id)
        .get_result(&mut conn)
        .await?;

    let row: StickerRow = diesel::sql_query(
        "SELECT id, pack_id, name, emoji, content_type, size_bytes, storage_key, tags, position, created_at FROM stickers WHERE id=$1",
    )
    .bind::<BigInt, _>(id)
    .get_result(&mut conn)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(Sticker {
            id: row.id,
            pack_id: row.pack_id,
            name: row.name,
            emoji: row.emoji,
            content_type: row.content_type,
            size_bytes: row.size_bytes,
            storage_key: row.storage_key,
            tags: row.tags,
            position: row.position as i64,
            created_at: row.created_at.timestamp_millis(),
        }),
    ))
}

async fn list_pack_stickers(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, pack_id)): Path<(i64, i64)>,
) -> AppResult<Json<Vec<Sticker>>> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, permissions::VIEW_CHANNEL).await?;

    let rows: Vec<StickerRow> = diesel::sql_query(
        "SELECT s.id, s.pack_id, s.name, s.emoji, s.content_type, s.size_bytes, s.storage_key, s.tags, s.position, s.created_at FROM stickers s WHERE s.pack_id=$1 ORDER BY s.position, s.id",
    )
    .bind::<BigInt, _>(pack_id)
    .load(&mut conn)
    .await?;

    Ok(Json(rows.into_iter().map(|r| Sticker {
        id: r.id,
        pack_id: r.pack_id,
        name: r.name,
        emoji: r.emoji,
        content_type: r.content_type,
        size_bytes: r.size_bytes,
        storage_key: r.storage_key,
        tags: r.tags,
        position: r.position as i64,
        created_at: r.created_at.timestamp_millis(),
    }).collect()))
}
