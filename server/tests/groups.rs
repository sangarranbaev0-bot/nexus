//! Групповые личные сообщения. Нужен PostgreSQL из ../.env (DATABASE_URL).

mod common;

use axum::{Router, http::StatusCode};
use common::{Ws, call, register_user, spawn_instance};
use nexus_server::protocol::ServerMessage;
use serde_json::{Value, json};

async fn name(app: &Router, token: &str) -> String {
    call(app, "GET", "/api/me", None, Some(token), None)
        .await
        .body["username"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn create(app: &Router, token: &str, body: Value) -> (StatusCode, Value) {
    let r = call(app, "POST", "/api/dms/group", Some(body), Some(token), None).await;
    (r.status, r.body)
}

#[tokio::test]
async fn group_dms() {
    let (app, url) = spawn_instance().await;
    let (a, _) = register_user(&app).await;
    let (b, _) = register_user(&app).await;
    let (c, c_id) = register_user(&app).await;
    let (d, _) = register_user(&app).await;
    let (bn, cn) = (name(&app, &b).await, name(&app, &c).await);
    let mut c_ws = Ws::connect(&url, &c).await;

    // Меньше трёх человек — не группа; больше десяти — тоже.
    assert_eq!(
        create(&app, &a, json!({ "usernames": [bn] })).await.0,
        StatusCode::BAD_REQUEST
    );
    let many: Vec<String> = (0..10).map(|i| format!("u{i}")).collect();
    assert_eq!(
        create(&app, &a, json!({ "usernames": many })).await.0,
        StatusCode::NOT_FOUND
    );

    let (status, g) = create(
        &app,
        &a,
        json!({ "usernames": [bn, cn], "name": "Тусовка" }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{g}");
    let gid = g["id"].as_i64().unwrap();
    assert_eq!(
        (g["isGroup"].clone(), g["name"].clone()),
        (json!(true), json!("Тусовка"))
    );
    assert_eq!(g["members"].as_array().unwrap().len(), 2);

    // Все участники получают событие и видят группу в списке.
    loop {
        if matches!(c_ws.next().await, ServerMessage::DmsChanged) {
            break;
        }
    }
    let r = call(&app, "GET", "/api/dms", None, Some(&c), None).await;
    assert!(
        r.body
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["id"] == gid && d["isGroup"] == true && d["name"] == "Тусовка")
    );

    // Сообщение видят все участники.
    let r = call(
        &app,
        "POST",
        &format!("/api/channels/{gid}/messages"),
        Some(json!({ "content": "всем привет" })),
        Some(&a),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    loop {
        if let ServerMessage::MessageCreated { message } = c_ws.next().await {
            assert_eq!(message.channel_id, gid);
            break;
        }
    }

    // Переименование участником; посторонний — 404.
    assert_eq!(
        call(
            &app,
            "PATCH",
            &format!("/api/dms/{gid}"),
            Some(json!({ "name": "Новое" })),
            Some(&b),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(
            &app,
            "PATCH",
            &format!("/api/dms/{gid}"),
            Some(json!({ "name": "x" })),
            Some(&d),
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );

    // Выход: теряется доступ к переписке; можно добавить обратно.
    assert_eq!(
        call(
            &app,
            "DELETE",
            &format!("/api/dms/{gid}/members/me"),
            None,
            Some(&c),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("/api/channels/{gid}/messages"),
            None,
            Some(&c),
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
            &format!("/api/dms/{gid}/members"),
            Some(json!({ "username": cn })),
            Some(&a),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );

    // Новый участник получает группу в списке.
    let r = call(&app, "GET", "/api/dms", None, Some(&a), None).await;
    let group = r
        .body
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == gid)
        .unwrap()
        .clone();
    assert_eq!(group["name"], "Новое");
    assert!(
        group["members"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["userId"] == c_id)
    );

    // Нельзя добавить заблокированного.
    assert_eq!(
        call(
            &app,
            "PUT",
            &format!("/api/blocks/{}", c_id),
            None,
            Some(&a),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    let r = call(
        &app,
        "POST",
        "/api/dms/group",
        Some(json!({ "usernames": [bn, cn] })),
        Some(&a),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn only_members_see_group() {
    let app = common::test_app().await;
    let (a, _) = register_user(&app).await;
    let (b, _) = register_user(&app).await;
    let (c, _) = register_user(&app).await;
    let (eve, _) = register_user(&app).await;
    let (bn, cn) = (name(&app, &b).await, name(&app, &c).await);
    let (_, g) = create(&app, &a, json!({ "usernames": [bn, cn] })).await;
    let gid = g["id"].as_i64().unwrap();

    assert_eq!(
        call(
            &app,
            "GET",
            &format!("/api/channels/{gid}/messages"),
            None,
            Some(&eve),
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    let r = call(
        &app,
        "POST",
        &format!("/api/channels/{gid}/messages"),
        Some(json!({ "content": "чужой" })),
        Some(&eve),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let r = call(&app, "GET", "/api/dms", None, Some(&eve), None).await;
    assert_eq!(r.body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn leaving_last_member_deletes_group() {
    let app = common::test_app().await;
    let (a, _) = register_user(&app).await;
    let (b, _) = register_user(&app).await;
    let (c, _) = register_user(&app).await;
    let (bn, cn) = (name(&app, &b).await, name(&app, &c).await);
    let (_, g) = create(&app, &a, json!({ "usernames": [bn, cn] })).await;
    let gid = g["id"].as_i64().unwrap();

    for t in [&a, &b, &c] {
        assert_eq!(
            call(
                &app,
                "DELETE",
                &format!("/api/dms/{gid}/members/me"),
                None,
                Some(t),
                None
            )
            .await
            .status,
            StatusCode::NO_CONTENT
        );
    }
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("/api/channels/{gid}/messages"),
            None,
            Some(&a),
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
}
