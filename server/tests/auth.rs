//! Интеграционные тесты аутентификации. Нужен PostgreSQL из ../.env (DATABASE_URL).

mod common;

use axum::http::StatusCode;
use common::{call, test_app, unique};
use nexus_server::protocol::AuthResponse;
use serde_json::json;

#[tokio::test]
async fn full_session_lifecycle() {
    let app = test_app().await;
    let id = unique();
    let email = format!("user{id}@test.dev");
    let reg = json!({ "email": email, "username": format!("u{id}"), "password": "correct-horse" });

    // Регистрация выдаёт access-токен и httpOnly refresh-cookie.
    let r = call(
        &app,
        "POST",
        "/api/auth/register",
        Some(reg.clone()),
        None,
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let auth: AuthResponse = serde_json::from_value(r.body).unwrap();
    assert_eq!(auth.user.email, email);
    assert!(r.cookie.as_deref().unwrap().starts_with("refresh_token="));

    // Повторная регистрация с тем же email запрещена.
    let r = call(&app, "POST", "/api/auth/register", Some(reg), None, None).await;
    assert_eq!(r.status, StatusCode::CONFLICT);

    // Неверный пароль и неизвестный email дают одинаковый ответ.
    for (e, p) in [
        (email.as_str(), "wrong-password"),
        ("nobody@test.dev", "correct-horse"),
    ] {
        let r = call(
            &app,
            "POST",
            "/api/auth/login",
            Some(json!({ "email": e, "password": p })),
            None,
            None,
        )
        .await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED);
        assert_eq!(r.body["error"], "invalid email or password");
    }

    // Вход, email регистронезависим.
    let r = call(
        &app,
        "POST",
        "/api/auth/login",
        Some(json!({ "email": email.to_uppercase(), "password": "correct-horse" })),
        None,
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let access = r.body["accessToken"].as_str().unwrap().to_string();
    let cookie = r.cookie.unwrap();

    // /me: с токеном работает, без него нет.
    let r = call(&app, "GET", "/api/me", None, Some(&access), None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body["email"], email);
    assert_eq!(
        call(&app, "GET", "/api/me", None, None, None).await.status,
        StatusCode::UNAUTHORIZED
    );

    // Refresh выдаёт новую пару, старый refresh после ротации не работает.
    let r = call(&app, "POST", "/api/auth/refresh", None, None, Some(&cookie)).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    let new_cookie = r.cookie.unwrap();
    assert_ne!(new_cookie, cookie);
    let r = call(&app, "POST", "/api/auth/refresh", None, None, Some(&cookie)).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);

    // Logout отзывает refresh.
    let r = call(
        &app,
        "POST",
        "/api/auth/logout",
        None,
        None,
        Some(&new_cookie),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let r = call(
        &app,
        "POST",
        "/api/auth/refresh",
        None,
        None,
        Some(&new_cookie),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn register_validates_input() {
    let app = test_app().await;
    let id = unique();
    let cases = [
        json!({ "email": "bad", "username": format!("v{id}"), "password": "12345678" }),
        json!({ "email": format!("v{id}@t.dev"), "username": "x", "password": "12345678" }),
        json!({ "email": format!("v{id}@t.dev"), "username": format!("v{id}"), "password": "short" }),
    ];
    for body in cases {
        let r = call(&app, "POST", "/api/auth/register", Some(body), None, None).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.body);
    }
}
