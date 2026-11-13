//! Голосовые каналы и звонки в ЛС. Сервер хранит только, кто где, и пересылает WebRTC-сигналы;
//! звук и видео идут напрямую между участниками (каждый с каждым).

use std::collections::HashMap;

use axum::{Json, Router, extract::State, routing::get};
use diesel::{
    prelude::*,
    sql_types::{BigInt, Bool, Text},
};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    events::{Notice, notify},
    permissions::{self, CONNECT_VOICE, VIEW_CHANNEL},
    protocol::{ChannelKind, VoiceFlags, VoiceParticipant},
    schema::{channels, voice_signals, voice_states},
    servers::channel_access,
};

/// SDP с видео и ICE-кандидаты — единицы килобайт; больше не нужно и не принимаем.
pub const MAX_SIGNAL_BYTES: usize = 64 * 1024;

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/voice", get(list))
}

#[derive(diesel::QueryableByName)]
struct Row {
    #[diesel(sql_type = BigInt)]
    channel_id: i64,
    #[diesel(sql_type = BigInt)]
    user_id: i64,
    #[diesel(sql_type = Text)]
    username: String,
    #[diesel(sql_type = BigInt)]
    connection_id: i64,
    #[diesel(sql_type = Bool)]
    muted: bool,
    #[diesel(sql_type = Bool)]
    deafened: bool,
    #[diesel(sql_type = Bool)]
    camera: bool,
    #[diesel(sql_type = Bool)]
    screen: bool,
}

impl From<Row> for VoiceParticipant {
    fn from(r: Row) -> Self {
        VoiceParticipant {
            channel_id: r.channel_id,
            user_id: r.user_id,
            username: r.username,
            connection_id: r.connection_id,
            flags: VoiceFlags {
                muted: r.muted,
                deafened: r.deafened,
                camera: r.camera,
                screen: r.screen,
            },
        }
    }
}

/// Все голоса, которые пользователь может видеть: в каналах его серверов и в его ЛС.
async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> AppResult<Json<Vec<VoiceParticipant>>> {
    let mut conn = state.pool.get().await?;
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT v.channel_id, v.user_id, u.username, v.connection_id, v.muted, v.deafened, v.camera, v.screen
         FROM voice_states v
         JOIN users u ON u.id = v.user_id
         JOIN channels c ON c.id = v.channel_id
         WHERE EXISTS (SELECT 1 FROM members m WHERE m.server_id = c.server_id AND m.user_id = $1)
            OR EXISTS (SELECT 1 FROM dm_participants d WHERE d.channel_id = c.id AND d.user_id = $1)
         ORDER BY v.joined_at",
    )
    .bind::<BigInt, _>(user_id)
    .load(&mut conn)
    .await?;

    // Скрытые каналы не должны выдавать даже присутствие участников.
    let channel_ids: Vec<i64> = rows.iter().map(|r| r.channel_id).collect();
    let meta: Vec<(i64, Option<i64>)> = channels::table
        .filter(channels::id.eq_any(&channel_ids))
        .select((channels::id, channels::server_id))
        .load(&mut conn)
        .await?;
    let server_of: HashMap<i64, Option<i64>> = meta.into_iter().collect();
    let mut perms_by_server = HashMap::new();
    let mut visible_by_channel = HashMap::new();
    let mut visible = Vec::with_capacity(rows.len());
    for row in rows {
        if let Some(Some(server_id)) = server_of.get(&row.channel_id).copied() {
            if let std::collections::hash_map::Entry::Vacant(entry) =
                perms_by_server.entry(server_id)
            {
                entry.insert(permissions::member_perms(&mut conn, server_id, user_id).await?);
            }
            let perms = perms_by_server.get(&server_id).expect("inserted above");
            let allowed = match visible_by_channel.get(&row.channel_id) {
                Some(allowed) => *allowed,
                None => permissions::has(
                    permissions::channel_bits(&mut conn, perms, row.channel_id).await?,
                    VIEW_CHANNEL,
                ),
            };
            visible_by_channel.insert(row.channel_id, allowed);
        }
        if visible_by_channel.get(&row.channel_id).copied().unwrap_or(true) {
            visible.push(row.into());
        }
    }
    Ok(Json(visible))
}

/// Канал и подключение, из которого пользователь сейчас в голосе. `None` — не в голосе.
async fn current(conn: &mut AsyncPgConnection, user_id: i64) -> AppResult<Option<(i64, i64)>> {
    Ok(voice_states::table
        .find(user_id)
        .select((voice_states::channel_id, voice_states::connection_id))
        .first(conn)
        .await
        .optional()?)
}

/// Входит в голос. Голосовой канал сервера или любой ЛС; из прежнего голоса выходит.
pub async fn join(
    conn: &mut AsyncPgConnection,
    user_id: i64,
    connection_id: i64,
    channel_id: i64,
) -> AppResult<()> {
    let access = channel_access(conn, channel_id, user_id).await?;
    if access.server_id.is_some() && access.kind != ChannelKind::Voice {
        return Err(AppError::bad_request("not a voice channel"));
    }
    if let Some(server_id) = access.server_id {
        permissions::require_channel_permissions(
            conn,
            server_id,
            user_id,
            channel_id,
            VIEW_CHANNEL | CONNECT_VOICE,
        )
        .await?;
    }
    if let Some((old_channel, _)) = current(conn, user_id).await?
        && old_channel != channel_id
    {
        notify_left(conn, user_id, old_channel).await?;
    }
    // Новое подключение заменяет старое (вход из другой вкладки), флаги сбрасываются.
    diesel::insert_into(voice_states::table)
        .values((
            voice_states::user_id.eq(user_id),
            voice_states::channel_id.eq(channel_id),
            voice_states::connection_id.eq(connection_id),
        ))
        .on_conflict(voice_states::user_id)
        .do_update()
        .set((
            voice_states::channel_id.eq(channel_id),
            voice_states::connection_id.eq(connection_id),
            voice_states::muted.eq(false),
            voice_states::deafened.eq(false),
            voice_states::camera.eq(false),
            voice_states::screen.eq(false),
        ))
        .execute(conn)
        .await?;
    notify_state(conn, user_id, access.server_id).await
}

/// Флаги меняет только то подключение, из которого пользователь в голосе.
pub async fn set_flags(
    conn: &mut AsyncPgConnection,
    user_id: i64,
    connection_id: i64,
    f: VoiceFlags,
) -> AppResult<()> {
    let Some((channel_id, owner)) = current(conn, user_id).await? else {
        return Ok(());
    };
    if owner != connection_id {
        return Ok(());
    }
    diesel::update(voice_states::table.find(user_id))
        .set((
            voice_states::muted.eq(f.muted),
            voice_states::deafened.eq(f.deafened),
            voice_states::camera.eq(f.camera),
            voice_states::screen.eq(f.screen),
        ))
        .execute(conn)
        .await?;
    let server_id = channel_access(conn, channel_id, user_id).await?.server_id;
    notify_state(conn, user_id, server_id).await
}

/// Выход из голоса этим подключением (кнопка «Отключиться» или закрытая вкладка).
pub async fn leave(
    conn: &mut AsyncPgConnection,
    user_id: i64,
    connection_id: i64,
) -> AppResult<()> {
    let left: Option<i64> = diesel::delete(
        voice_states::table
            .find(user_id)
            .filter(voice_states::connection_id.eq(connection_id)),
    )
    .returning(voice_states::channel_id)
    .get_result(conn)
    .await
    .optional()?;
    match left {
        Some(channel_id) => notify_left(conn, user_id, channel_id).await,
        None => Ok(()),
    }
}

/// Пересылает сигнал, только если оба сейчас в одном голосовом канале.
pub async fn signal(
    conn: &mut AsyncPgConnection,
    from: i64,
    to: i64,
    data: &serde_json::Value,
) -> AppResult<()> {
    let payload = data.to_string();
    if payload.len() > MAX_SIGNAL_BYTES {
        return Err(AppError::bad_request("signal too large"));
    }
    let channels: Vec<i64> = voice_states::table
        .filter(voice_states::user_id.eq_any([from, to]))
        .select(voice_states::channel_id)
        .load(conn)
        .await?;
    if channels.len() != 2 || channels[0] != channels[1] {
        return Err(AppError::forbidden());
    }
    // Старые сигналы не нужны: соединение либо установилось за секунды, либо уже не установится.
    diesel::sql_query("DELETE FROM voice_signals WHERE created_at < now() - interval '1 minute'")
        .execute(conn)
        .await?;
    let id: i64 = diesel::insert_into(voice_signals::table)
        .values((
            voice_signals::from_user.eq(from),
            voice_signals::to_user.eq(to),
            voice_signals::payload.eq(payload),
        ))
        .returning(voice_signals::id)
        .get_result(conn)
        .await?;
    notify(conn, &Notice::VoiceSignal { signal_id: id }).await
}

/// Сигнал для слушателя событий: (кому, от кого, данные).
pub async fn load_signal(
    conn: &mut AsyncPgConnection,
    id: i64,
) -> AppResult<Option<(i64, i64, String)>> {
    Ok(voice_signals::table
        .find(id)
        .select((
            voice_signals::to_user,
            voice_signals::from_user,
            voice_signals::payload,
        ))
        .first(conn)
        .await
        .optional()?)
}

async fn notify_state(
    conn: &mut AsyncPgConnection,
    user_id: i64,
    server_id: Option<i64>,
) -> AppResult<()> {
    let row: Row = diesel::sql_query(
        "SELECT v.channel_id, v.user_id, u.username, v.connection_id, v.muted, v.deafened, v.camera, v.screen
         FROM voice_states v JOIN users u ON u.id = v.user_id WHERE v.user_id = $1",
    )
    .bind::<BigInt, _>(user_id)
    .get_result(conn)
    .await?;
    notify(
        conn,
        &Notice::VoiceState {
            server_id,
            participant: row.into(),
        },
    )
    .await
}

async fn notify_left(conn: &mut AsyncPgConnection, user_id: i64, channel_id: i64) -> AppResult<()> {
    let server_id: Option<i64> = crate::schema::channels::table
        .find(channel_id)
        .select(crate::schema::channels::server_id)
        .first(conn)
        .await
        .optional()?
        .flatten();
    notify(
        conn,
        &Notice::VoiceLeft {
            server_id,
            channel_id,
            user_id,
        },
    )
    .await
}

/// Для уборки упавших экземпляров: убрать голос подключений без пульса и оповестить.
pub async fn drop_stale(
    conn: &mut AsyncPgConnection,
    cutoff: chrono::DateTime<chrono::Utc>,
) -> AppResult<()> {
    use crate::schema::presence_connections as pc;
    let stale: Vec<(i64, i64)> = diesel::delete(
        voice_states::table.filter(
            voice_states::connection_id
                .eq_any(pc::table.filter(pc::heartbeat_at.lt(cutoff)).select(pc::id)),
        ),
    )
    .returning((voice_states::user_id, voice_states::channel_id))
    .get_results(conn)
    .await?;
    for (user_id, channel_id) in stale {
        notify_left(conn, user_id, channel_id).await?;
    }
    Ok(())
}
