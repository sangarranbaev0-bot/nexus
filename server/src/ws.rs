use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tokio::sync::broadcast::error::RecvError;

use crate::{
    AppState,
    auth::verify_access,
    error::{AppError, AppResult},
    events::{Event, Notice, notify},
    presence,
    protocol::{ClientMessage, ServerMessage, now_ms},
    schema::{members, users},
    servers::channel_access,
    voice,
};

const AUTH_TIMEOUT: Duration = Duration::from_secs(10);
const TYPING_MIN_INTERVAL: Duration = Duration::from_secs(2);

pub async fn handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Состояние одного подключения.
struct Conn {
    user_id: i64,
    username: String,
    /// Серверы, события которых получает подключение.
    // shortcut: набор только растёт (вступление приходит событием `Joined`), потому что выхода
    // и исключения с сервера пока нет; когда появятся, нужно событие `Left` и удаление отсюда.
    servers: HashSet<i64>,
    /// Каналы, к которым доступ уже проверен: id сервера или `None` для ЛС.
    channel_server: HashMap<i64, Option<i64>>,
    last_typing: HashMap<i64, Instant>,
    /// Строка в `presence_connections`.
    presence_row: i64,
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    // Токен идёт первым сообщением, а не в URL: URL попадает в логи.
    let Some(user_id) = authenticate(&mut socket, &state).await else {
        let _ = send(
            &mut socket,
            &ServerMessage::Error {
                message: "unauthorized".into(),
            },
        )
        .await;
        return;
    };
    // Подписываемся до чтения членства, чтобы не пропустить вступление между этими шагами.
    let mut events = state.events.subscribe();
    let mut conn = match load_conn(&state, user_id).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("ws: load user {user_id}: {e:?}");
            return;
        }
    };
    let conn_id = conn.presence_row;
    tracing::debug!(conn_id, user_id, "ws connected");

    if send(&mut socket, &ServerMessage::Hello { conn_id, user_id })
        .await
        .is_err()
    {
        return;
    }

    let mut heartbeat = tokio::time::interval(presence::HEARTBEAT_EVERY);
    heartbeat.tick().await; // первый тик срабатывает сразу, а строка только что создана

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                if let Err(e) = beat(&state, conn.presence_row).await {
                    tracing::warn!(conn_id, "presence heartbeat failed: {e:?}");
                }
            }
            incoming = socket.recv() => {
                let Some(Ok(msg)) = incoming else { break };
                let Message::Text(text) = msg else { continue };
                if let Some(reply) = on_client_message(&state, &mut conn, &text).await
                    && send(&mut socket, &reply).await.is_err()
                {
                    break;
                }
            }
            event = events.recv() => match event {
                Ok(Event::ToServer { server_id, json }) if conn.servers.contains(&server_id) => {
                    if socket.send(Message::Text((*json).into())).await.is_err() {
                        break;
                    }
                }
                Ok(Event::ToServers { server_ids, json })
                    if server_ids.iter().any(|id| conn.servers.contains(id)) =>
                {
                    if socket.send(Message::Text((*json).into())).await.is_err() {
                        break;
                    }
                }
                Ok(Event::ToUsers { user_ids, json }) if user_ids.contains(&conn.user_id) => {
                    if socket.send(Message::Text((*json).into())).await.is_err() {
                        break;
                    }
                }
                Ok(Event::Joined { server_id, user_id }) if user_id == conn.user_id => {
                    conn.servers.insert(server_id);
                }
                Ok(_) => {}
                // Отстали от потока событий: закрываем, клиент переподключится и перечитает ленту.
                Err(RecvError::Lagged(n)) => {
                    tracing::warn!(conn_id, n, "ws lagged, closing");
                    break;
                }
                Err(RecvError::Closed) => break,
            }
        }
    }
    // Если запись не удалась, строки уберёт sweep по просроченному пульсу.
    if let Ok(mut db) = state.pool.get().await {
        if let Err(e) = voice::leave(&mut db, conn.user_id, conn.presence_row).await {
            tracing::warn!(conn_id, "voice leave failed: {e:?}");
        }
        if let Err(e) = presence::disconnect(&mut db, conn.presence_row, conn.user_id).await {
            tracing::warn!(conn_id, "presence disconnect failed: {e:?}");
        }
    }
    tracing::debug!(conn_id, "ws disconnected");
}

async fn beat(state: &AppState, row: i64) -> AppResult<()> {
    let mut db = state.pool.get().await?;
    presence::heartbeat(&mut db, row).await
}

async fn load_conn(state: &AppState, user_id: i64) -> AppResult<Conn> {
    let mut db = state.pool.get().await?;
    let username: String = users::table
        .find(user_id)
        .select(users::username)
        .first(&mut db)
        .await?;
    let servers: Vec<i64> = members::table
        .filter(members::user_id.eq(user_id))
        .select(members::server_id)
        .load(&mut db)
        .await?;
    let presence_row = presence::connect(&mut db, user_id).await?;
    Ok(Conn {
        user_id,
        username,
        servers: servers.into_iter().collect(),
        channel_server: HashMap::new(),
        last_typing: HashMap::new(),
        presence_row,
    })
}

async fn on_client_message(state: &AppState, conn: &mut Conn, text: &str) -> Option<ServerMessage> {
    match serde_json::from_str::<ClientMessage>(text) {
        Ok(ClientMessage::SetIdle { idle }) => {
            let result = async {
                let mut db = state.pool.get().await?;
                presence::set_idle(&mut db, conn.presence_row, conn.user_id, idle).await
            }
            .await;
            if let Err(e) = result {
                tracing::warn!("set idle failed: {e:?}");
            }
            None
        }
        Ok(ClientMessage::Ping { sent_at }) => Some(ServerMessage::Pong {
            sent_at,
            server_time: now_ms(),
        }),
        Ok(ClientMessage::Typing { channel_id }) => {
            if let Err(e) = on_typing(state, conn, channel_id).await {
                tracing::debug!("typing ignored: {e:?}");
            }
            None
        }
        Ok(ClientMessage::VoiceJoin { channel_id }) => {
            voice_result(state, async |db| {
                voice::join(db, conn.user_id, conn.presence_row, channel_id).await
            })
            .await
        }
        Ok(ClientMessage::VoiceLeave) => {
            voice_result(state, async |db| {
                voice::leave(db, conn.user_id, conn.presence_row).await
            })
            .await
        }
        Ok(ClientMessage::VoiceSetFlags { flags }) => {
            voice_result(state, async |db| {
                voice::set_flags(db, conn.user_id, conn.presence_row, flags).await
            })
            .await
        }
        Ok(ClientMessage::VoiceSignal { to_user_id, data }) => {
            voice_result(state, async |db| {
                voice::signal(db, conn.user_id, to_user_id, &data).await
            })
            .await
        }
        Ok(ClientMessage::Auth { .. }) => Some(ServerMessage::Error {
            message: "already authenticated".into(),
        }),
        Err(e) => Some(ServerMessage::Error {
            message: format!("invalid message: {e}"),
        }),
    }
}

/// Рассылает «печатает…», если у пользователя есть доступ к каналу. Чаще раза в 2 с не шлём.
async fn on_typing(state: &AppState, conn: &mut Conn, channel_id: i64) -> AppResult<()> {
    let now = Instant::now();
    if conn
        .last_typing
        .get(&channel_id)
        .is_some_and(|t| now.duration_since(*t) < TYPING_MIN_INTERVAL)
    {
        return Ok(());
    }
    let mut db = state.pool.get().await?;
    // Доступ проверяем один раз на канал: членство и участие в ЛС только появляются.
    let server_id = match conn.channel_server.get(&channel_id) {
        Some(id) => *id,
        None => {
            let id = channel_access(&mut db, channel_id, conn.user_id)
                .await?
                .server_id;
            conn.channel_server.insert(channel_id, id);
            id
        }
    };
    conn.last_typing.insert(channel_id, now);
    let notice = Notice::Typing {
        server_id,
        channel_id,
        user_id: conn.user_id,
        username: conn.username.clone(),
    };
    notify(&mut db, &notice).await
}

// shortcut: токен проверяется один раз при подключении, истечение access-токена не рвёт
// открытый сокет. События фильтруются по членству, так что доступ к чужим данным это не даёт.
async fn authenticate(socket: &mut WebSocket, state: &AppState) -> Option<i64> {
    let msg = tokio::time::timeout(AUTH_TIMEOUT, socket.recv())
        .await
        .ok()??
        .ok()?;
    let Message::Text(text) = msg else {
        return None;
    };
    match serde_json::from_str(&text).ok()? {
        ClientMessage::Auth { token } => verify_access(&state.jwt, &token),
        _ => None,
    }
}

/// Ошибку голосовой операции возвращаем клиенту текстом: он покажет её пользователю.
async fn voice_result(
    state: &AppState,
    op: impl AsyncFnOnce(&mut diesel_async::AsyncPgConnection) -> AppResult<()>,
) -> Option<ServerMessage> {
    let result = async {
        let mut db = state.pool.get().await?;
        op(&mut db).await
    }
    .await;
    result
        .err()
        .map(|AppError(_, message)| ServerMessage::Error { message })
}

async fn send(socket: &mut WebSocket, msg: &ServerMessage) -> Result<(), axum::Error> {
    let json = serde_json::to_string(msg).expect("ServerMessage always serializes");
    socket.send(Message::Text(json.into())).await
}
