//! Личные сообщения. Нужен PostgreSQL из ../.env (DATABASE_URL).

mod common;

use std::time::Duration;

use axum::{Router, http::StatusCode};
use common::{Ws, call, register_user, spawn_instance, test_app};
use nexus_server::protocol::{ClientMessage, Presence, ServerMessage};
use serde_json::{Value, json};

/// Пользователь без общих серверов с остальными: (token, id, username).
async fn user(app: &Router) -> (String, i64, String) {
    let (token, id) = register_user(app).await;
    let r = call(app, "GET", "/api/me", None, Some(&token), None).await;
    (token, id, r.body["username"].as_str().unwrap().to_string())
}

async fn open(app: &Router, token: &str, username: &str) -> (StatusCode, Value) {
    let r = call(
        app,
        "POST",
        "/api/dms",
        Some(json!({ "username": username })),
        Some(token),
        None,
    )
    .await;
    (r.status, r.body)
}

#[tokio::test]
async fn open_privacy_and_unread() {
    let app = test_app().await;
    let (alice, _, alice_name) = user(&app).await;
    let (bob, bob_id, bob_name) = user(&app).await;
    let (eve, _, _) = user(&app).await;

    // Один диалог на пару, с какой стороны его ни открывай.
    let (status, dm) = open(&app, &alice, &bob_name).await;
    assert_eq!(status, StatusCode::OK, "{dm}");
    assert_eq!(dm["userId"], bob_id);
    assert_eq!(dm["status"], "offline");
    let cid = dm["id"].as_i64().unwrap();
    assert_eq!(open(&app, &alice, &bob_name).await.1["id"], cid);
    assert_eq!(open(&app, &bob, &alice_name).await.1["id"], cid);

    assert_eq!(
        open(&app, &alice, &alice_name).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        open(&app, &alice, "no-such-user-xyz").await.0,
        StatusCode::NOT_FOUND
    );

    // Сообщения и непрочитанные.
    let msgs = format!("/api/channels/{cid}/messages");
    let r = call(
        &app,
        "POST",
        &msgs,
        Some(json!({ "content": "привет, Боб" })),
        Some(&alice),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    let mid = r.body["id"].as_i64().unwrap();
    let r = call(&app, "GET", "/api/dms", None, Some(&bob), None).await;
    let bob_dm = r
        .body
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == cid)
        .unwrap()
        .clone();
    assert_eq!(bob_dm["username"], alice_name.as_str());
    assert_eq!(bob_dm["unreadCount"], 1);
    let r = call(
        &app,
        "POST",
        &format!("/api/channels/{cid}/read"),
        Some(json!({ "messageId": mid })),
        Some(&bob),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let r = call(&app, "GET", "/api/dms", None, Some(&bob), None).await;
    assert_eq!(r.body[0]["unreadCount"], 0);

    // Третий не видит диалог ни в каком виде.
    assert_eq!(
        call(&app, "GET", &msgs, None, Some(&eve), None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let r = call(
        &app,
        "POST",
        &msgs,
        Some(json!({ "content": "hi" })),
        Some(&eve),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let r = call(&app, "GET", "/api/dms", None, Some(&eve), None).await;
    assert_eq!(r.body.as_array().unwrap().len(), 0);
    let r = call(
        &app,
        "PATCH",
        &format!("/api/messages/{mid}"),
        Some(json!({ "content": "x" })),
        Some(&eve),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // В ЛС нет модераторов: собеседник не удаляет чужое, автор удаляет своё.
    let del = format!("/api/messages/{mid}");
    assert_eq!(
        call(&app, "DELETE", &del, None, Some(&bob), None)
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(&app, "DELETE", &del, None, Some(&alice), None)
            .await
            .status,
        StatusCode::NO_CONTENT
    );

    // Диалог нельзя удалить как серверный канал.
    let r = call(
        &app,
        "DELETE",
        &format!("/api/channels/{cid}"),
        None,
        Some(&alice),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn realtime_only_for_participants() {
    let (app1, url1) = spawn_instance().await;
    let (app2, url2) = spawn_instance().await;
    let (alice, alice_id, _) = user(&app1).await;
    let (bob, _, bob_name) = user(&app1).await;
    let (eve, _, _) = user(&app1).await;
    let cid = open(&app1, &alice, &bob_name).await.1["id"]
        .as_i64()
        .unwrap();

    let mut bob_ws = Ws::connect(&url2, &bob).await;
    let mut eve_ws = Ws::connect(&url1, &eve).await;

    // Статус собеседника доходит без общего сервера.
    let mut alice_ws = Ws::connect(&url1, &alice).await;
    loop {
        if let ServerMessage::PresenceChanged { user_id, status } = bob_ws.next().await
            && user_id == alice_id
        {
            assert_eq!(status, Presence::Online);
            break;
        }
    }

    alice_ws
        .send(&ClientMessage::Typing { channel_id: cid })
        .await;
    loop {
        if let ServerMessage::Typing { channel_id, .. } = bob_ws.next().await {
            assert_eq!(channel_id, cid);
            break;
        }
    }

    let r = call(
        &app2,
        "POST",
        &format!("/api/channels/{cid}/messages"),
        Some(json!({ "content": "секрет" })),
        Some(&alice),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    loop {
        if let ServerMessage::MessageCreated { message } = bob_ws.next().await {
            assert_eq!(message.content, "секрет");
            break;
        }
    }

    // Посторонний не получил ни статуса, ни «печатает…», ни сообщения; печатать в чужой ЛС не может.
    eve_ws
        .send(&ClientMessage::Typing { channel_id: cid })
        .await;
    assert_eq!(eve_ws.next_within(Duration::from_millis(500)).await, None);
    assert!(
        !matches!(
            bob_ws.next_within(Duration::from_millis(500)).await,
            Some(ServerMessage::Typing { .. })
        ),
        "eve's typing must not reach bob"
    );
}
