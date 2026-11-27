//! Интеграционные тесты тикетов. Нужен PostgreSQL из ../.env (DATABASE_URL).

mod common;

use axum::http::StatusCode;
use common::{call, register_user, test_app};
use serde_json::json;

#[tokio::test]
async fn ticket_crud() {
    let app = test_app().await;
    let (owner, owner_id) = register_user(&app).await;
    let (member, _) = register_user(&app).await;

    // Создаём сервер — владелец получает доступ к тикетам.
    let r = call(
        &app,
        "POST",
        "/api/servers",
        Some(json!({ "name": "Support Server" })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    let sid = r.body["id"].as_i64().unwrap();

    //Member не видит тикеты чужого сервера.
    let r = call(
        &app,
        "GET",
        &format!("/api/servers/{sid}/tickets"),
        None,
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // Owner создаёт тикет.
    let r = call(
        &app,
        "POST",
        &format!("/api/servers/{sid}/tickets"),
        Some(json!({ "title": "Не работает вход" })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let tid = r.body["id"].as_i64().unwrap();
    assert_eq!(r.body["status"], "open");
    assert_eq!(r.body["creatorId"], owner_id);

    // Читаем список тикетов.
    let r = call(
        &app,
        "GET",
        &format!("/api/servers/{sid}/tickets"),
        None,
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body.as_array().unwrap().len(), 1);
    assert_eq!(r.body[0]["id"], tid);
    assert_eq!(r.body[0]["title"], "Не работает вход");

    // Обновляем статус на closed.
    let r = call(
        &app,
        "PATCH",
        &format!("/api/servers/{sid}/tickets/{tid}"),
        Some(json!({ "status": "closed" })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body["status"], "closed");
    assert!(r.body["closedAt"].is_number());

    // Меняем title.
    let r = call(
        &app,
        "PATCH",
        &format!("/api/servers/{sid}/tickets/{tid}"),
        Some(json!({ "title": "Починили" })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body["title"], "Починили");

    // Неавторизованный пользователь не может обновить.
    let r = call(
        &app,
        "PATCH",
        &format!("/api/servers/{sid}/tickets/{tid}"),
        Some(json!({ "status": "open" })),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // Несуществующий тикет → NOT FOUND.
    let r = call(
        &app,
        "PATCH",
        &format!("/api/servers/{sid}/tickets/99999"),
        Some(json!({ "status": "closed" })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn ticket_empty_list() {
    let app = test_app().await;
    let (user, _) = register_user(&app).await;

    let r = call(
        &app,
        "POST",
        "/api/servers",
        Some(json!({ "name": "Empty" })),
        Some(&user),
        None,
    )
    .await;
    let sid = r.body["id"].as_i64().unwrap();

    let r = call(
        &app,
        "GET",
        &format!("/api/servers/{sid}/tickets"),
        None,
        Some(&user),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body.as_array().unwrap().len(), 0);
}
