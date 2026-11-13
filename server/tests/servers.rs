//! Серверы, каналы, инвайты и права. Нужен PostgreSQL из ../.env (DATABASE_URL).

mod common;

use axum::http::StatusCode;
use common::{call, register_user, test_app};
use serde_json::{Value, json};

#[tokio::test]
async fn roles_channels_and_invites() {
    let app = test_app().await;
    let (owner, _) = register_user(&app).await;
    let (member, member_id) = register_user(&app).await;
    let (outsider, _) = register_user(&app).await;

    // Создание: создатель становится owner, появляется канал #общий.
    let r = call(
        &app,
        "POST",
        "/api/servers",
        Some(json!({ "name": "  Rust Club " })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert_eq!(r.body["name"], "Rust Club");
    assert_eq!(r.body["myRole"], "owner");
    let sid = r.body["id"].as_i64().unwrap();
    let channels = format!("/api/servers/{sid}/channels");

    let r = call(&app, "GET", &channels, None, Some(&owner), None).await;
    // Новый сервер: текстовый #общий и голосовой «голосовой».
    assert_eq!(r.body.as_array().unwrap().len(), 2);
    assert_eq!(r.body[0]["name"], "общий");
    assert_eq!(r.body[0]["kind"], "text");
    assert_eq!(r.body[1]["kind"], "voice");

    // Посторонний не видит, что сервер существует.
    for (method, uri) in [
        ("GET", channels.clone()),
        ("GET", format!("/api/servers/{sid}/members")),
    ] {
        assert_eq!(
            call(&app, method, &uri, None, Some(&outsider), None)
                .await
                .status,
            StatusCode::NOT_FOUND
        );
    }
    let r = call(
        &app,
        "POST",
        &format!("/api/servers/{sid}/invites"),
        None,
        Some(&outsider),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);

    // Инвайт на одно использование.
    let r = call(
        &app,
        "POST",
        &format!("/api/servers/{sid}/invites"),
        Some(json!({ "maxUses": 1 })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let code = r.body["code"].as_str().unwrap().to_string();
    let join = format!("/api/invites/{code}/join");

    let r = call(&app, "POST", &join, None, Some(&member), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["myRole"], "member");
    // Повторный join участника не тратит использование и не падает.
    assert_eq!(
        call(&app, "POST", &join, None, Some(&member), None)
            .await
            .status,
        StatusCode::OK
    );
    // Лимит исчерпан для нового человека.
    assert_eq!(
        call(&app, "POST", &join, None, Some(&outsider), None)
            .await
            .status,
        StatusCode::GONE
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/invites/NOPE0000/join",
            None,
            Some(&outsider),
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );

    let r = call(&app, "GET", "/api/servers", None, Some(&member), None).await;
    assert!(r.body.as_array().unwrap().iter().any(|s| s["id"] == sid));

    // Member не может создавать каналы и назначать роли.
    let new_channel = Some(json!({ "name": "Новости Проекта" }));
    let r = call(
        &app,
        "POST",
        &channels,
        new_channel.clone(),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let promote = format!("/api/servers/{sid}/members/{member_id}");
    let r = call(
        &app,
        "PATCH",
        &promote,
        Some(json!({ "role": "admin" })),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);

    // Owner делает его admin, передать владение нельзя.
    let r = call(
        &app,
        "PATCH",
        &promote,
        Some(json!({ "role": "owner" })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    let r = call(
        &app,
        "PATCH",
        &promote,
        Some(json!({ "role": "admin" })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);

    let r = call(
        &app,
        "GET",
        &format!("/api/servers/{sid}/members"),
        None,
        Some(&member),
        None,
    )
    .await;
    let roles: Vec<&Value> = r
        .body
        .as_array()
        .unwrap()
        .iter()
        .map(|m| &m["role"])
        .collect();
    assert_eq!(roles, [&json!("owner"), &json!("admin")]);

    // Admin создаёт канал (имя нормализуется) и удаляет его.
    let r = call(&app, "POST", &channels, new_channel, Some(&member), None).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert_eq!(r.body["name"], "новости-проекта");
    assert_eq!(r.body["position"], 2);
    let cid = r.body["id"].as_i64().unwrap();

    let r = call(
        &app,
        "POST",
        &channels,
        Some(json!({ "name": "bad!name" })),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);

    let del = format!("/api/channels/{cid}");
    assert_eq!(
        call(&app, "DELETE", &del, None, Some(&outsider), None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(&app, "DELETE", &del, None, Some(&member), None)
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(&app, "DELETE", &del, None, Some(&owner), None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn validation_and_unlimited_invites() {
    let app = test_app().await;
    let (owner, _) = register_user(&app).await;
    let (other, _) = register_user(&app).await;

    let r = call(
        &app,
        "POST",
        "/api/servers",
        Some(json!({ "name": "x" })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        call(&app, "GET", "/api/servers", None, None, None)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );

    let r = call(
        &app,
        "POST",
        "/api/servers",
        Some(json!({ "name": "Test" })),
        Some(&owner),
        None,
    )
    .await;
    let sid = r.body["id"].as_i64().unwrap();
    let invites = format!("/api/servers/{sid}/invites");
    for bad in [
        json!({ "maxUses": 0 }),
        json!({ "expiresInHours": 0 }),
        json!({ "expiresInHours": 9999 }),
    ] {
        let r = call(&app, "POST", &invites, Some(bad), Some(&owner), None).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.body);
    }

    // Бессрочный инвайт без тела запроса.
    let r = call(&app, "POST", &invites, None, Some(&owner), None).await;
    assert_eq!(r.status, StatusCode::CREATED);
    assert_eq!(r.body["expiresAt"], Value::Null);
    let r = call(
        &app,
        "POST",
        &format!("/api/invites/{}/join", r.body["code"].as_str().unwrap()),
        None,
        Some(&other),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
}
