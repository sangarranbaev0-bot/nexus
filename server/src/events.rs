//! Рассылка событий между экземплярами сервера через PostgreSQL LISTEN/NOTIFY.
//!
//! Обработчик пишет в БД и вызывает `notify`. Каждый экземпляр слушает канал `nexus_events`,
//! превращает уведомление в `ServerMessage` и раздаёт своим WebSocket-подключениям.
//! В уведомлении только id: лимит payload в PostgreSQL — 8000 байт, а сообщение может быть больше.

use std::{sync::Arc, time::Duration};

use diesel::sql_types::Text;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use futures_util::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use tokio_postgres::{AsyncMessage, NoTls};

use diesel::prelude::*;

use crate::{
    AppState,
    error::AppResult,
    messages::load_message,
    presence,
    protocol::{ServerMessage, VoiceParticipant},
    schema::{dm_participants, members},
    voice,
};

const PG_CHANNEL: &str = "nexus_events";

/// Что уходит в pg_notify.
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Notice {
    MessageCreated {
        /// `None` — личные сообщения: доставка двум собеседникам.
        server_id: Option<i64>,
        message_id: i64,
    },
    MessageUpdated {
        /// `None` — личные сообщения: доставка двум собеседникам.
        server_id: Option<i64>,
        message_id: i64,
    },
    MessageDeleted {
        /// `None` — личные сообщения: доставка двум собеседникам.
        server_id: Option<i64>,
        channel_id: i64,
        message_id: i64,
    },
    Typing {
        /// `None` — личные сообщения: доставка двум собеседникам.
        server_id: Option<i64>,
        channel_id: i64,
        user_id: i64,
        username: String,
    },
    ChannelsChanged {
        server_id: i64,
    },
    RolesChanged {
        server_id: i64,
    },
    MemberJoined {
        server_id: i64,
        user_id: i64,
    },
    /// Подключения пользователя изменились: слушатель сам перечитает итоговый статус,
    /// поэтому порядок уведомлений от разных экземпляров не важен.
    PresenceChanged {
        user_id: i64,
    },
    VoiceState {
        server_id: Option<i64>,
        participant: VoiceParticipant,
    },
    VoiceLeft {
        server_id: Option<i64>,
        channel_id: i64,
        user_id: i64,
    },
    VoiceSignal {
        signal_id: i64,
    },
    ChannelRead {
        user_id: i64,
        server_id: Option<i64>,
        channel_id: i64,
    },
}

/// Событие внутри экземпляра, которое получают WebSocket-подключения.
#[derive(Clone, Debug)]
pub enum Event {
    /// Готовый JSON для всех участников сервера: сериализуем один раз, а не на каждое подключение.
    ToServer { server_id: i64, json: Arc<str> },
    /// Для участников любого из этих серверов (статус виден во всех общих серверах).
    ToServers {
        server_ids: Arc<[i64]>,
        json: Arc<str>,
    },
    /// Только подключениям этих пользователей: собеседникам в ЛС, своим вкладкам.
    ToUsers {
        user_ids: Arc<[i64]>,
        json: Arc<str>,
    },
    /// Пользователь стал участником сервера: его подключения начинают получать события.
    Joined { server_id: i64, user_id: i64 },
}

pub fn channel() -> broadcast::Sender<Event> {
    broadcast::channel(1024).0
}

pub async fn notify(conn: &mut AsyncPgConnection, notice: &Notice) -> AppResult<()> {
    let payload = serde_json::to_string(notice).expect("Notice always serializes");
    diesel::sql_query("SELECT pg_notify($1, $2)")
        .bind::<Text, _>(PG_CHANNEL)
        .bind::<Text, _>(payload)
        .execute(conn)
        .await?;
    Ok(())
}

/// Подписывается на `nexus_events` и возвращается, когда LISTEN уже действует.
/// Дальше слушатель работает в фоне и сам переподключается при обрыве.
pub async fn start_listener(state: AppState, database_url: String) -> anyhow::Result<()> {
    let mut current = listen(&database_url).await?;
    tokio::spawn(async move {
        loop {
            // `current.0` — клиент: держим его, иначе соединение закроется.
            while let Some(payload) = current.1.recv().await {
                handle(&state, &payload).await;
            }
            // shortcut: события за время переподключения теряются; клиент перечитывает
            // ленту при переподключении сокета, replay понадобится, если это станет заметно.
            tracing::warn!("event listener disconnected, reconnecting");
            current = loop {
                match listen(&database_url).await {
                    Ok(c) => break c,
                    Err(e) => {
                        tracing::error!("event listener reconnect failed: {e}");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            };
        }
    });
    Ok(())
}

/// Отдельное соединение (не из пула): LISTEN привязан к сессии.
async fn listen(
    database_url: &str,
) -> anyhow::Result<(tokio_postgres::Client, mpsc::UnboundedReceiver<String>)> {
    let (client, mut connection) = tokio_postgres::connect(database_url, NoTls).await?;
    let (tx, rx) = mpsc::unbounded_channel();
    // Соединение надо опрашивать, иначе ни запросы клиента, ни уведомления не пройдут.
    tokio::spawn(async move {
        let mut messages = stream::poll_fn(move |cx| connection.poll_message(cx));
        while let Some(msg) = messages.next().await {
            match msg {
                Ok(AsyncMessage::Notification(n)) => {
                    if tx.send(n.payload().to_owned()).is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("event listener connection error: {e}");
                    break;
                }
            }
        }
    });
    client
        .batch_execute(&format!("LISTEN {PG_CHANNEL}"))
        .await?;
    Ok((client, rx))
}

async fn handle(state: &AppState, payload: &str) {
    let notice: Notice = match serde_json::from_str(payload) {
        Ok(n) => n,
        Err(e) => return tracing::error!("bad notice {payload}: {e}"),
    };
    let result = match notice {
        Notice::MemberJoined { server_id, user_id } => {
            let _ = state.events.send(Event::Joined { server_id, user_id });
            Ok(())
        }
        Notice::PresenceChanged { user_id } => broadcast_presence(state, user_id).await,
        Notice::VoiceState {
            server_id,
            participant,
        } => {
            let channel_id = participant.channel_id;
            deliver_lazy(
                state,
                server_id,
                channel_id,
                &ServerMessage::VoiceState { participant },
            )
            .await
        }
        Notice::VoiceLeft {
            server_id,
            channel_id,
            user_id,
        } => {
            deliver_lazy(
                state,
                server_id,
                channel_id,
                &ServerMessage::VoiceLeft {
                    channel_id,
                    user_id,
                },
            )
            .await
        }
        Notice::VoiceSignal { signal_id } => {
            async {
                let mut conn = state.pool.get().await?;
                if let Some((to, from, payload)) = voice::load_signal(&mut conn, signal_id).await? {
                    let data = serde_json::from_str(&payload).unwrap_or_default();
                    let msg = ServerMessage::VoiceSignal {
                        from_user_id: from,
                        data,
                    };
                    send(
                        state,
                        Event::ToUsers {
                            user_ids: [to].into(),
                            json: to_json(&msg),
                        },
                    );
                }
                AppResult::Ok(())
            }
            .await
        }
        Notice::ChannelRead {
            user_id,
            server_id,
            channel_id,
        } => {
            let msg = ServerMessage::ChannelRead {
                server_id,
                channel_id,
            };
            send(
                state,
                Event::ToUsers {
                    user_ids: [user_id].into(),
                    json: to_json(&msg),
                },
            );
            Ok(())
        }
        Notice::ChannelsChanged { server_id } => {
            send(
                state,
                Event::ToServer {
                    server_id,
                    json: to_json(&ServerMessage::ChannelsChanged { server_id }),
                },
            );
            Ok(())
        }
        Notice::RolesChanged { server_id } => {
            send(
                state,
                Event::ToServer {
                    server_id,
                    json: to_json(&ServerMessage::RolesChanged { server_id }),
                },
            );
            Ok(())
        }
        Notice::MessageCreated {
            server_id,
            message_id,
        }
        | Notice::MessageUpdated {
            server_id,
            message_id,
        } => {
            let created = matches!(notice, Notice::MessageCreated { .. });
            async {
                let mut conn = state.pool.get().await?;
                // Успели удалить: событие удаления придёт следом.
                let Some(message) = load_message(&mut conn, &state.file_key, message_id).await?
                else {
                    return Ok(());
                };
                let channel_id = message.channel_id;
                let msg = if created {
                    ServerMessage::MessageCreated { message }
                } else {
                    ServerMessage::MessageUpdated { message }
                };
                deliver(state, &mut conn, server_id, channel_id, &msg).await
            }
            .await
        }
        Notice::MessageDeleted {
            server_id,
            channel_id,
            message_id,
        } => {
            let msg = ServerMessage::MessageDeleted {
                channel_id,
                message_id,
            };
            deliver_lazy(state, server_id, channel_id, &msg).await
        }
        Notice::Typing {
            server_id,
            channel_id,
            user_id,
            username,
        } => {
            let msg = ServerMessage::Typing {
                channel_id,
                user_id,
                username,
            };
            deliver_lazy(state, server_id, channel_id, &msg).await
        }
    };
    if let Err(e) = result {
        tracing::error!("event listener: {payload}: {e:?}");
    }
}

fn send(state: &AppState, event: Event) {
    // Ошибка значит лишь, что на этом экземпляре сейчас нет подключений.
    let _ = state.events.send(event);
}

/// Серверный канал — всем участникам сервера, ЛС — двум собеседникам.
async fn deliver(
    state: &AppState,
    conn: &mut AsyncPgConnection,
    server_id: Option<i64>,
    channel_id: i64,
    msg: &ServerMessage,
) -> AppResult<()> {
    let json = to_json(msg);
    match server_id {
        Some(server_id) => send(state, Event::ToServer { server_id, json }),
        None => {
            let user_ids: Vec<i64> = dm_participants::table
                .filter(dm_participants::channel_id.eq(channel_id))
                .select(dm_participants::user_id)
                .load(conn)
                .await?;
            send(
                state,
                Event::ToUsers {
                    user_ids: user_ids.into(),
                    json,
                },
            );
        }
    }
    Ok(())
}

/// Как `deliver`, но соединение с БД берём, только если это ЛС.
async fn deliver_lazy(
    state: &AppState,
    server_id: Option<i64>,
    channel_id: i64,
    msg: &ServerMessage,
) -> AppResult<()> {
    if let Some(server_id) = server_id {
        send(
            state,
            Event::ToServer {
                server_id,
                json: to_json(msg),
            },
        );
        return Ok(());
    }
    let mut conn = state.pool.get().await?;
    deliver(state, &mut conn, None, channel_id, msg).await
}

fn to_json(msg: &ServerMessage) -> Arc<str> {
    serde_json::to_string(msg)
        .expect("ServerMessage always serializes")
        .into()
}

#[derive(diesel::QueryableByName)]
struct UserId {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    user_id: i64,
}

/// Статус читаем из БД в момент обработки: так итог верен при любом порядке уведомлений.
/// Получают его участники общих серверов и собеседники по ЛС.
async fn broadcast_presence(state: &AppState, user_id: i64) -> AppResult<()> {
    let mut conn = state.pool.get().await?;
    let status = presence::status_of(&mut conn, user_id).await?;
    let server_ids: Vec<i64> = members::table
        .filter(members::user_id.eq(user_id))
        .select(members::server_id)
        .load(&mut conn)
        .await?;
    let partners: Vec<UserId> = diesel::sql_query(
        "SELECT DISTINCT other.user_id FROM dm_participants me
         JOIN dm_participants other ON other.channel_id = me.channel_id AND other.user_id <> me.user_id
         WHERE me.user_id = $1",
    )
    .bind::<diesel::sql_types::BigInt, _>(user_id)
    .load(&mut conn)
    .await?;

    let json = to_json(&ServerMessage::PresenceChanged { user_id, status });
    if !server_ids.is_empty() {
        send(
            state,
            Event::ToServers {
                server_ids: server_ids.into(),
                json: json.clone(),
            },
        );
    }
    if !partners.is_empty() {
        let user_ids: Vec<i64> = partners.into_iter().map(|p| p.user_id).collect();
        send(
            state,
            Event::ToUsers {
                user_ids: user_ids.into(),
                json,
            },
        );
    }
    Ok(())
}
