//! Друзья, заявки и блокировки. Нужен PostgreSQL из ../.env (DATABASE_URL).

mod common;

use std::time::Duration;

use axum::{Router, http::StatusCode};
use common::{Ws, call, register_user, spawn_instance, test_app};
use nexus_server::protocol::ServerMessage;
use serde_json::{Value, json};

/// Пользователь без общих серверов: (token, id, username).
async fn user(app: &Router) -> (String, i64, String) {
    let (token, id) = register_user(app).await;
    let r = call(app, "GET", "/api/me", None, Some(&token), None).await;
    (token, id, r.body["username"].as_str().unwrap().to_string())
}

async fn request(app: &Router, token: &str, username: &str) -> (StatusCode, Value) {
    let r = call(
        app,
        "POST",
        "/api/friends/requests",
        Some(json!({ "username": username })),
        Some(token),
        None,
    )
    .await;
    (r.status, r.body)
}

async fn requests(app: &Router, token: &str) -> Value {
    call(app, "GET", "/api/friends/requests", None, Some(token), None)
        .await
        .body
}

async fn friends(app: &Router, token: &str) -> Value {
    call(app, "GET", "/api/friends", None, Some(token), None)
        .await
        .body
}

async fn blocks(app: &Router, token: &str) -> Value {
    call(app, "GET", "/api/blocks", None, Some(token), None)
        .await
        .body
}

#[tokio::test]
async fn request_accept_remove_flow() {
    let app = test_app().await;
    let (alice, alice_id, alice_name) = user(&app).await;
    let (bob, bob_id, bob_name) = user(&app).await;
    let (eve, _, _) = user(&app).await;

    // Заявка и её валидация.
    let (status, req) = request(&app, &alice, &bob_name).await;
    assert_eq!(status, StatusCode::CREATED, "{req}");
    assert_eq!(req["fromUserId"], alice_id);
    assert_eq!(req["fromUsername"], alice_name.as_str());
    assert_eq!(req["toUserId"], bob_id);
    assert_eq!(req["toUsername"], bob_name.as_str());
    let request_id = req["id"].as_i64().unwrap();

    assert_eq!(
        request(&app, &alice, &bob_name).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(&app, &alice, &alice_name).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(&app, &alice, "no-such-user-xyz").await.0,
        StatusCode::NOT_FOUND
    );

    // Входящие и исходящие видны только задействованным.
    let alice_reqs = requests(&app, &alice).await;
    assert_eq!(alice_reqs["incoming"].as_array().unwrap().len(), 0);
    assert_eq!(alice_reqs["outgoing"][0]["id"], request_id);
    let bob_reqs = requests(&app, &bob).await;
    assert_eq!(bob_reqs["incoming"][0]["id"], request_id);
    assert_eq!(bob_reqs["outgoing"].as_array().unwrap().len(), 0);
    assert_eq!(
        requests(&app, &eve).await,
        json!({ "incoming": [], "outgoing": [] })
    );

    // Принять может только получатель.
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/friends/requests/{request_id}/accept"),
            None,
            Some(&eve),
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/friends/requests/{request_id}/accept"),
            None,
            Some(&bob),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(requests(&app, &bob).await["incoming"], json!([]));
    assert_eq!(friends(&app, &alice).await[0]["userId"], bob_id);
    assert_eq!(friends(&app, &bob).await[0]["userId"], alice_id);
    assert_eq!(
        request(&app, &alice, &bob_name).await.0,
        StatusCode::CONFLICT
    );

    // Отклонение входящей и отмена исходящей.
    let (_, req) = request(&app, &eve, &alice_name).await;
    let id = req["id"].as_i64().unwrap();
    assert_eq!(
        call(
            &app,
            "DELETE",
            &format!("/api/friends/requests/{id}"),
            None,
            Some(&alice),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(requests(&app, &eve).await["outgoing"], json!([]));

    // Удаление дружбы симметрично.
    assert_eq!(
        call(
            &app,
            "DELETE",
            &format!("/api/friends/{bob_id}"),
            None,
            Some(&alice),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(friends(&app, &alice).await, json!([]));
    assert_eq!(friends(&app, &bob).await, json!([]));
}

#[tokio::test]
async fn blocking_removes_friends_and_requests() {
    let app = test_app().await;
    let (alice, _, alice_name) = user(&app).await;
    let (bob, bob_id, bob_name) = user(&app).await;

    // Блокировка отменяет висящую заявку.
    let (_, req) = request(&app, &alice, &bob_name).await;
    assert!(req["id"].is_i64());
    assert_eq!(
        call(
            &app,
            "PUT",
            &format!("/api/blocks/{bob_id}"),
            None,
            Some(&alice),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(requests(&app, &alice).await["outgoing"], json!([]));
    assert_eq!(requests(&app, &bob).await["incoming"], json!([]));
    assert_eq!(blocks(&app, &alice).await[0]["userId"], bob_id);
    assert_eq!(blocks(&app, &alice).await[0]["username"], bob_name.as_str());

    // Заблокированный не может отправить заявку.
    assert_eq!(
        request(&app, &bob, &alice_name).await.0,
        StatusCode::FORBIDDEN
    );

    // Разблокировка возвращает возможность заявки; дружба после блокировки исчезает.
    assert_eq!(
        call(
            &app,
            "DELETE",
            &format!("/api/blocks/{bob_id}"),
            None,
            Some(&alice),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    let (_, req) = request(&app, &bob, &alice_name).await;
    let id = req["id"].as_i64().unwrap();
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/friends/requests/{id}/accept"),
            None,
            Some(&alice),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(friends(&app, &alice).await.as_array().unwrap().len(), 1);

    assert_eq!(
        call(
            &app,
            "PUT",
            &format!("/api/blocks/{bob_id}"),
            None,
            Some(&alice),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(friends(&app, &alice).await, json!([]));
    assert_eq!(friends(&app, &bob).await, json!([]));
}

#[tokio::test]
async fn block_stops_dm_messages() {
    let app = test_app().await;
    let (_, alice_id, alice_name) = user(&app).await;
    let (bob, _, _) = user(&app).await;

    let r = call(
        &app,
        "POST",
        "/api/dms",
        Some(json!({ "username": alice_name })),
        Some(&bob),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let cid = r.body["id"].as_i64().unwrap();
    let msgs = format!("/api/channels/{cid}/messages");
    assert_eq!(
        call(
            &app,
            "POST",
            &msgs,
            Some(json!({ "content": "привет" })),
            Some(&bob),
            None
        )
        .await
        .status,
        StatusCode::CREATED
    );

    // Блокировка запрещает писать в существующий диалог и открывать новый.
    assert_eq!(
        call(
            &app,
            "PUT",
            &format!("/api/blocks/{alice_id}"),
            None,
            Some(&bob),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    let r = call(
        &app,
        "POST",
        &msgs,
        Some(json!({ "content": "ещё" })),
        Some(&bob),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.body);
    assert_eq!(r.body["error"], "blocked");
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/dms",
            Some(json!({ "username": alice_name })),
            Some(&bob),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    // Разблокировка снимает запрет.
    assert_eq!(
        call(
            &app,
            "DELETE",
            &format!("/api/blocks/{alice_id}"),
            None,
            Some(&bob),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &msgs,
            Some(json!({ "content": "снова" })),
            Some(&bob),
            None
        )
        .await
        .status,
        StatusCode::CREATED
    );
}

#[tokio::test]
async fn friends_changed_reaches_both_users() {
    let (app1, url1) = spawn_instance().await;
    let (_app2, url2) = spawn_instance().await;
    let (alice, _, _) = user(&app1).await;
    let (bob, _, bob_name) = user(&app1).await;

    let mut alice_ws = Ws::connect(&url1, &alice).await;
    let mut bob_ws = Ws::connect(&url2, &bob).await;

    assert_eq!(
        request(&app1, &alice, &bob_name).await.0,
        StatusCode::CREATED
    );

    let mut alice_saw = false;
    let mut bob_saw = false;
    for _ in 0..10 {
        if !bob_saw
            && let Some(ServerMessage::FriendsChanged) =
                bob_ws.next_within(Duration::from_secs(2)).await
        {
            bob_saw = true;
        }
        if !alice_saw
            && let Some(ServerMessage::FriendsChanged) =
                alice_ws.next_within(Duration::from_secs(2)).await
        {
            alice_saw = true;
        }
        if alice_saw && bob_saw {
            break;
        }
    }
    assert!(alice_saw, "sender must learn about the change");
    assert!(bob_saw, "recipient must learn about the request");
}
