//! Настройки и центр уведомлений, слова-триггеры. Нужен PostgreSQL из ../.env (DATABASE_URL).

mod common;

use std::time::Duration;

use axum::{Router, http::StatusCode};
use common::{Ws, call, register_user, spawn_instance, test_app};
use nexus_server::protocol::ServerMessage;
use serde_json::{Value, json};

async fn settings(app: &Router, token: &str) -> Value {
    call(
        app,
        "GET",
        "/api/me/notify-settings",
        None,
        Some(token),
        None,
    )
    .await
    .body
}

async fn set(app: &Router, token: &str, body: Value) -> StatusCode {
    call(
        app,
        "PUT",
        "/api/me/notify-settings",
        Some(body),
        Some(token),
        None,
    )
    .await
    .status
}

async fn center(app: &Router, token: &str) -> Value {
    call(app, "GET", "/api/me/notifications", None, Some(token), None)
        .await
        .body
}

async fn username(app: &Router, token: &str) -> String {
    call(app, "GET", "/api/me", None, Some(token), None)
        .await
        .body["username"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Приглашает пользователя на сервер.
async fn join(app: &Router, owner: &str, sid: i64, token: &str) {
    let r = call(
        app,
        "POST",
        &format!("/api/servers/{sid}/invites"),
        None,
        Some(owner),
        None,
    )
    .await;
    let code = r.body["code"].as_str().unwrap().to_string();
    let r = call(
        app,
        "POST",
        &format!("/api/invites/{code}/join"),
        None,
        Some(token),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
}

async fn send(app: &Router, token: &str, cid: i64, content: &str) -> i64 {
    let r = call(
        app,
        "POST",
        &format!("/api/channels/{cid}/messages"),
        Some(json!({ "content": content })),
        Some(token),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    r.body["id"].as_i64().unwrap()
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

    assert_eq!(settings(&app, &owner).await, json!([]));

    // Настройка сервера.
    assert_eq!(
        set(
            &app,
            &owner,
            json!({ "serverId": sid, "channelId": null, "level": "mentions", "mutedUntil": null, "suppressEveryone": false })
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
            json!({ "serverId": null, "channelId": cid, "level": "none", "mutedUntil": until, "suppressEveryone": true })
        )
        .await,
        StatusCode::NO_CONTENT
    );

    let list = settings(&app, &owner).await;
    let server_row = list
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["serverId"] == sid)
        .unwrap();
    assert_eq!(server_row["level"], "mentions");
    assert_eq!(server_row["mutedUntil"], json!(null));
    assert_eq!(server_row["suppressEveryone"], false);
    let channel_row = list
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["channelId"] == cid)
        .unwrap();
    assert_eq!(channel_row["level"], "none");
    assert_eq!(channel_row["mutedUntil"], until);
    assert_eq!(channel_row["suppressEveryone"], true);

    // Повторная запись перезаписывает, а не дублирует.
    assert_eq!(
        set(
            &app,
            &owner,
            json!({ "serverId": sid, "channelId": null, "level": "all", "mutedUntil": null, "suppressEveryone": false })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    let list = settings(&app, &owner).await;
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
            json!({ "serverId": sid, "channelId": cid, "level": "all", "mutedUntil": null, "suppressEveryone": false })
        )
        .await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        set(
            &app,
            &owner,
            json!({ "serverId": null, "channelId": null, "level": "all", "mutedUntil": null, "suppressEveryone": false })
        )
        .await,
        StatusCode::BAD_REQUEST
    );

    // Посторонний не видит сервер и канал.
    assert_eq!(
        set(
            &app,
            &outsider,
            json!({ "serverId": sid, "channelId": null, "level": "all", "mutedUntil": null, "suppressEveryone": false })
        )
        .await,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        set(
            &app,
            &outsider,
            json!({ "serverId": null, "channelId": cid, "level": "all", "mutedUntil": null, "suppressEveryone": false })
        )
        .await,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn mentions_replies_and_everyone() {
    let app = test_app().await;
    let (owner, owner_id) = register_user(&app).await;
    let (bob, _) = register_user(&app).await;
    let (sid, cid) = server_with_channel(&app, &owner).await;
    join(&app, &owner, sid, &bob).await;
    let bob_name = username(&app, &bob).await;

    // Обычное сообщение без упоминаний не создаёт уведомлений.
    send(&app, &owner, cid, "просто болтовня").await;
    assert_eq!(center(&app, &bob).await, json!([]));

    // Упоминание по имени.
    let mention_id = send(&app, &owner, cid, &format!("привет @{bob_name}")).await;
    let list = center(&app, &bob).await;
    assert_eq!(list.as_array().unwrap().len(), 1, "{list}");
    assert_eq!(list[0]["kind"], "mention");
    assert_eq!(list[0]["actorId"], owner_id);
    assert_eq!(list[0]["messageId"], mention_id);
    assert_eq!(list[0]["read"], false);
    assert_eq!(list[0]["location"], "Notify Club");
    // Автор сам уведомление не получает.
    assert_eq!(center(&app, &owner).await, json!([]));

    // Отметка прочтения: одиночная и «все».
    assert_eq!(
        call(
            &app,
            "POST",
            "/api/me/notifications/read",
            Some(json!({ "id": list[0]["id"] })),
            Some(&bob),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(center(&app, &bob).await[0]["read"], true);
    let unread = call(
        &app,
        "GET",
        "/api/me/notifications?unread=true",
        None,
        Some(&bob),
        None,
    )
    .await;
    assert_eq!(unread.body, json!([]));

    // Ответ на сообщение Боба (без упоминания).
    let bob_msg = send(&app, &bob, cid, "вопрос от боба").await;
    let r = call(
        &app,
        "POST",
        &format!("/api/channels/{cid}/messages"),
        Some(json!({ "content": "отвечаю", "replyToId": bob_msg })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let list = center(&app, &bob).await;
    assert!(
        list.as_array()
            .unwrap()
            .iter()
            .any(|n| n["kind"] == "reply")
    );

    // @everyone: участник без права его не «пингует».
    send(&app, &bob, cid, "@everyone тишина").await;
    let list = center(&app, &owner).await;
    assert!(
        !list
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["kind"] == "everyone"),
        "{list}"
    );

    // Владелец с правом MENTION_EVERYONE — срабатывает.
    send(&app, &owner, cid, "@everyone подъём").await;
    let list = center(&app, &bob).await;
    assert!(
        list.as_array()
            .unwrap()
            .iter()
            .any(|n| n["kind"] == "everyone"),
        "{list}"
    );

    // Блокировка не даёт уведомлений.
    assert_eq!(
        call(
            &app,
            "PUT",
            &format!("/api/blocks/{owner_id}"),
            None,
            Some(&bob),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    let before = center(&app, &bob).await.as_array().unwrap().len();
    send(&app, &owner, cid, &format!("@{bob_name} снова")).await;
    assert_eq!(center(&app, &bob).await.as_array().unwrap().len(), before);
}

#[tokio::test]
async fn mute_and_level_respected() {
    let app = test_app().await;
    let (owner, _) = register_user(&app).await;
    let (bob, _) = register_user(&app).await;
    let (sid, cid) = server_with_channel(&app, &owner).await;
    join(&app, &owner, sid, &bob).await;
    let bob_name = username(&app, &bob).await;

    // mute на канале до 2100 года — упоминания не создают записей.
    let until = 4_102_444_800_000_i64;
    assert_eq!(
        set(
            &app,
            &bob,
            json!({ "serverId": null, "channelId": cid, "level": "all", "mutedUntil": until, "suppressEveryone": false })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    send(&app, &owner, cid, &format!("@{bob_name}")).await;
    assert_eq!(center(&app, &bob).await, json!([]));

    // Снимаем mute: упоминание проходит.
    assert_eq!(
        set(
            &app,
            &bob,
            json!({ "serverId": null, "channelId": cid, "level": "all", "mutedUntil": null, "suppressEveryone": false })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    send(&app, &owner, cid, &format!("@{bob_name}")).await;
    assert_eq!(center(&app, &bob).await.as_array().unwrap().len(), 1);

    // Уровень «ничего» — уведомления не создаются.
    assert_eq!(
        set(
            &app,
            &bob,
            json!({ "serverId": null, "channelId": cid, "level": "none", "mutedUntil": null, "suppressEveryone": false })
        )
        .await,
        StatusCode::NO_CONTENT
    );
    send(&app, &owner, cid, &format!("@{bob_name}")).await;
    assert_eq!(center(&app, &bob).await.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn notification_words() {
    let app = test_app().await;
    let (owner, _) = register_user(&app).await;
    let (bob, _) = register_user(&app).await;
    let (sid, cid) = server_with_channel(&app, &owner).await;
    join(&app, &owner, sid, &bob).await;

    // Добавление и валидация.
    let add = |word: &str, case_sensitive: bool| {
        let app = app.clone();
        let bob = bob.clone();
        let word = word.to_string();
        async move {
            call(
                &app,
                "POST",
                "/api/me/notification-words",
                Some(json!({ "word": word, "caseSensitive": case_sensitive })),
                Some(&bob),
                None,
            )
            .await
            .status
        }
    };
    assert_eq!(add("x", false).await, StatusCode::BAD_REQUEST);
    assert_eq!(add("немедленно", false).await, StatusCode::NO_CONTENT);

    let list = call(
        &app,
        "GET",
        "/api/me/notification-words",
        None,
        Some(&bob),
        None,
    )
    .await
    .body;
    assert_eq!(list[0]["word"], "немедленно");
    assert_eq!(list[0]["caseSensitive"], false);

    // Слово срабатывает без учёта регистра.
    send(&app, &owner, cid, "Сделай это НЕМЕДЛЕННО").await;
    let list = center(&app, &bob).await;
    assert!(
        list.as_array().unwrap().iter().any(|n| n["kind"] == "word"),
        "{list}"
    );

    // В своих сообщениях слово не считается.
    let before = center(&app, &bob).await.as_array().unwrap().len();
    send(&app, &bob, cid, "немедленно моё").await;
    assert_eq!(center(&app, &bob).await.as_array().unwrap().len(), before);

    // Удаление.
    assert_eq!(
        call(
            &app,
            "DELETE",
            "/api/me/notification-words/%D0%BD%D0%B5%D0%BC%D0%B5%D0%B4%D0%BB%D0%B5%D0%BD%D0%BD%D0%BE",
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
            "GET",
            "/api/me/notification-words",
            None,
            Some(&bob),
            None
        )
        .await
        .body,
        json!([])
    );
}

#[tokio::test]
async fn notification_event_reaches_addressee_only() {
    let (app1, url1) = spawn_instance().await;
    let (_app2, url2) = spawn_instance().await;
    let (owner, _) = register_user(&app1).await;
    let (bob, _) = register_user(&app1).await;
    let (sid, cid) = server_with_channel(&app1, &owner).await;
    join(&app1, &owner, sid, &bob).await;
    let bob_name = username(&app1, &bob).await;

    let mut bob_ws = Ws::connect(&url2, &bob).await;
    let mut owner_ws = Ws::connect(&url1, &owner).await;

    send(&app1, &owner, cid, &format!("@{bob_name} привет")).await;

    let mut bob_saw = false;
    for _ in 0..10 {
        if let Some(ServerMessage::NotificationCreated { notification }) =
            bob_ws.next_within(Duration::from_secs(2)).await
        {
            assert_eq!(
                notification.kind,
                nexus_server::protocol::NotificationKind::Mention
            );
            bob_saw = true;
            break;
        }
    }
    assert!(bob_saw, "адресат должен получить уведомление");

    // Автору событие не приходит.
    let owner_got = owner_ws
        .next_within(Duration::from_millis(800))
        .await
        .is_some_and(|m| matches!(m, ServerMessage::NotificationCreated { .. }));
    assert!(!owner_got, "автор не должен получать своё уведомление");
}
