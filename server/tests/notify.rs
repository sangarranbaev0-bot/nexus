//! Настройки уведомлений на сервер и канал. Нужен PostgreSQL из ../.env (DATABASE_URL).

mod common;

use axum::{Router, http::StatusCode};
use common::{call, register_user, test_app};
use serde_json::{Value, json};

async fn notifications(app: &Router, token: &str) -> Value {
    call(app, "GET", "/api/me/notifications", None, Some(token), None)
        .await
        .body
}

async fn set(app: &Router, token: &str, body: Value) -> StatusCode {
    call(
        app,
        "PUT",
        "/api/me/notifications",
        Some(body),
        Some(token),
        None,
    )
    .await
    .status
}

/// Сервер с текстовым каналом #общий. Возвращает (server_id, channel_id).
async fn server_with_channel(app: &Router, token: &str) -> (i64, i64) {
    let r = call(
        app,
        "POST",
        "/api/servers",
        Some(json!({ "name": "Notify Club" })),
        Some(token),
        None,
    )
    .await;
    let sid = r.body["id"].as_i64().unwrap();
    let r = call(
        app,
        "GET",
        &format!("/api/servers/{sid}/channels"),
        None,
        Some(token),
        None,
    )
    .await;
    let cid = r
        .body
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["kind"] == "text")
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    (sid, cid)
}

#[tokio::test]
async fn server_and_channel_settings() {
    let app = test_app().await;
    let (owner, _) = register_user(&app).await;
    let (sid, cid) = server_with_channel(&app, &owner).await;

    assert_eq!(notifications(&app, &owner).await, json!([]));

    // Настройка сервера.
    assert_eq!(
        set(
            &app,
            &owner,
            json!({ "serverId": sid, "channelId": null, "level": "mentions", "mutedUntil": null })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    // Настройка канала с mute.
    let until = 4_102_444_800_000_i64; // 2100-01-01
    assert_eq!(
        set(
            &app,
            &owner,
            json!({ "serverId": null, "channelId": cid, "level": "none", "mutedUntil": until })
        )
        .await,
        StatusCode::NO_CONTENT
    );

    let list = notifications(&app, &owner).await;
    let server_row = list
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["serverId"] == sid)
        .unwrap();
    assert_eq!(server_row["level"], "mentions");
    assert_eq!(server_row["mutedUntil"], json!(null));
    let channel_row = list
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["channelId"] == cid)
        .unwrap();
    assert_eq!(channel_row["level"], "none");
    assert_eq!(channel_row["mutedUntil"], until);

    // Повторная запись перезаписывает, а не дублирует.
    assert_eq!(
        set(
            &app,
            &owner,
            json!({ "serverId": sid, "channelId": null, "level": "all", "mutedUntil": null })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    let list = notifications(&app, &owner).await;
    assert_eq!(list.as_array().unwrap().len(), 2);
    let server_row = list
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["serverId"] == sid)
        .unwrap();
    assert_eq!(server_row["level"], "all");
}

#[tokio::test]
async fn validation_and_access() {
    let app = test_app().await;
    let (owner, _) = register_user(&app).await;
    let (outsider, _) = register_user(&app).await;
    let (sid, cid) = server_with_channel(&app, &owner).await;

    // Оба поля заданы или ни одного — ошибка.
    assert_eq!(
        set(
            &app,
            &owner,
            json!({ "serverId": sid, "channelId": cid, "level": "all", "mutedUntil": null })
        )
        .await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        set(
            &app,
            &owner,
            json!({ "serverId": null, "channelId": null, "level": "all", "mutedUntil": null })
        )
        .await,
        StatusCode::BAD_REQUEST
    );

    // Посторонний не видит сервер и канал.
    assert_eq!(
        set(
            &app,
            &outsider,
            json!({ "serverId": sid, "channelId": null, "level": "all", "mutedUntil": null })
        )
        .await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        set(
            &app,
            &outsider,
            json!({ "serverId": null, "channelId": cid, "level": "all", "mutedUntil": null })
        )
        .await,
        StatusCode::NOT_FOUND
    );
}
