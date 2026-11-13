//! WebSocket: аутентификация и ping. Нужен PostgreSQL из ../.env (DATABASE_URL).

mod common;

use common::{Ws, register_user, spawn_instance};
use futures_util::SinkExt;
use nexus_server::protocol::{ClientMessage, ServerMessage};
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn auth_then_ping_pong_and_bad_input() {
    let (app, url) = spawn_instance().await;
    let (token, user_id) = register_user(&app).await;
    let mut ws = Ws::connect(&url, &token).await;
    assert_eq!(ws.hello_user_id, user_id);

    let ping = serde_json::to_string(&ClientMessage::Ping { sent_at: 42 }).unwrap();
    assert_eq!(ping, r#"{"type":"ping","sentAt":42}"#);
    ws.stream.send(Message::Text(ping.into())).await.unwrap();
    match ws.next().await {
        ServerMessage::Pong {
            sent_at,
            server_time,
        } => {
            assert_eq!(sent_at, 42);
            assert!(server_time > 0);
        }
        other => panic!("expected pong, got {other:?}"),
    }

    ws.stream
        .send(Message::Text("not json".into()))
        .await
        .unwrap();
    assert!(matches!(ws.next().await, ServerMessage::Error { .. }));
}

#[tokio::test]
async fn rejects_bad_token_and_unauthenticated_ping() {
    let (_, url) = spawn_instance().await;
    for first in [
        ClientMessage::Auth {
            token: "garbage".into(),
        },
        ClientMessage::Ping { sent_at: 1 },
    ] {
        let mut ws = Ws::raw(&url).await;
        ws.send(&first).await;
        assert_eq!(
            ws.next().await,
            ServerMessage::Error {
                message: "unauthorized".into()
            }
        );
    }
}
