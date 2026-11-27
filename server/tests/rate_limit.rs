//! Интеграционные тесты rate limit через HTTP.

mod common;

use axum::http::StatusCode;
use common::{call, register_user, test_app};
use nexus_server::rate_limit;
use serde_json::json;

#[tokio::test]
async fn auth_rate_limit_blocks_excess() {
    let app = test_app().await;
    rate_limit::reset();

    // Регистрируем пользователя — это один запрос к /auth/register.
    let (_, _) = register_user(&app).await;

    // Теперь делаем ещё 10 запросов — все должны пройти.
    for _ in 0..10 {
        let r = call(
            &app,
            "POST",
            "/api/auth/login",
            Some(json!({ "email": "nonexistent@test.dev", "password": "x" })),
            None,
            None,
        )
        .await;
        assert_ne!(r.status, StatusCode::TOO_MANY_REQUESTS, "слишком рано попал в rate limit");
    }

    // Ещё несколько — должен сработать лимит (максимум 10 для auth).
    let mut hit_limit = false;
    for _ in 0..5 {
        let r = call(
            &app,
            "POST",
            "/api/auth/login",
            Some(json!({ "email": "nonexistent@test.dev", "password": "x" })),
            None,
            None,
        )
        .await;
        if r.status == StatusCode::TOO_MANY_REQUESTS {
            hit_limit = true;
            break;
        }
    }
    assert!(hit_limit, "rate limit для auth не сработал");
    rate_limit::reset();
}

#[tokio::test]
async fn messages_rate_limit_blocks_spam() {
    let app = test_app().await;
    rate_limit::reset();

    let (user, _) = register_user(&app).await;

    // Создаём сервер и канал.
    let srv = call(
        &app,
        "POST",
        "/api/servers",
        Some(json!({ "name": "SpamTest" })),
        Some(&user),
        None,
    )
    .await;
    let sid = srv.body["id"].as_i64().unwrap();
    let channels: Vec<_> = call(
        &app,
        "GET",
        &format!("/api/servers/{sid}/channels"),
        None,
        Some(&user),
        None,
    )
    .await
    .body
    .as_array()
    .unwrap()
    .to_vec();
    let cid = channels.iter().find(|c| c["kind"] == "text").unwrap()["id"].as_i64().unwrap();

    // Отправляем 30 сообщений — все должны пройти.
    for i in 0..30 {
        let r = call(
            &app,
            "POST",
            &format!("/api/channels/{cid}/messages"),
            Some(json!({ "content": format!("spam message #{i}") })),
            Some(&user),
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "message #{} failed", i);
    }

    // 31-е — должно получить 429.
    let r = call(
        &app,
        "POST",
        &format!("/api/channels/{cid}/messages"),
        Some(json!({ "content": "this should be blocked" })),
        Some(&user),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS, "expected 429, got {:?}", r.status);
    rate_limit::reset();
}
