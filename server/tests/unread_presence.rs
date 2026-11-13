//! Непрочитанные и присутствие. Нужен PostgreSQL из ../.env (DATABASE_URL).

mod common;

use std::time::Duration;

use axum::{Router, http::StatusCode};
use common::{Ws, call, register_user, spawn_instance, test_app, test_state};
use nexus_server::protocol::{ClientMessage, Presence, ServerMessage};
use serde_json::{Value, json};

/// (owner, member, member_id, server_id, channel_id)
async fn setup(app: &Router) -> (String, String, i64, i64, i64) {
    let (owner, _) = register_user(app).await;
    let (member, member_id) = register_user(app).await;
    let r = call(
        app,
        "POST",
        "/api/servers",
        Some(json!({ "name": "Unread" })),
        Some(&owner),
        None,
    )
    .await;
    let sid = r.body["id"].as_i64().unwrap();
    let r = call(
        app,
        "GET",
        &format!("/api/servers/{sid}/channels"),
        None,
        Some(&owner),
        None,
    )
    .await;
    let cid = r.body[0]["id"].as_i64().unwrap();
    let r = call(
        app,
        "POST",
        &format!("/api/servers/{sid}/invites"),
        None,
        Some(&owner),
        None,
    )
    .await;
    let code = r.body["code"].as_str().unwrap();
    call(
        app,
        "POST",
        &format!("/api/invites/{code}/join"),
        None,
        Some(&member),
        None,
    )
    .await;
    (owner, member, member_id, sid, cid)
}

async fn post(app: &Router, token: &str, cid: i64, text: &str) -> i64 {
    let r = call(
        app,
        "POST",
        &format!("/api/channels/{cid}/messages"),
        Some(json!({ "content": text })),
        Some(token),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED);
    r.body["id"].as_i64().unwrap()
}

/// (непрочитано в канале, непрочитано на сервере)
async fn unread(app: &Router, token: &str, sid: i64, cid: i64) -> (i64, i64) {
    let r = call(
        app,
        "GET",
        &format!("/api/servers/{sid}/channels"),
        None,
        Some(token),
        None,
    )
    .await;
    let ch = r
        .body
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == cid)
        .unwrap()["unreadCount"]
        .as_i64()
        .unwrap();
    let r = call(app, "GET", "/api/servers", None, Some(token), None).await;
    let sv = r
        .body
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == sid)
        .unwrap()["unreadCount"]
        .as_i64()
        .unwrap();
    (ch, sv)
}

#[tokio::test]
async fn unread_counts_and_mark_read() {
    let app = test_app().await;
    let (owner, member, _, sid, cid) = setup(&app).await;
    let read = format!("/api/channels/{cid}/read");

    let _m1 = post(&app, &owner, cid, "one").await;
    let m2 = post(&app, &owner, cid, "two").await;
    post(&app, &member, cid, "own messages are not unread").await;
    assert_eq!(unread(&app, &member, sid, cid).await, (2, 2));
    assert_eq!(unread(&app, &owner, sid, cid).await, (1, 1));

    // Второй канал добавляется в сумму сервера.
    let r = call(
        &app,
        "POST",
        &format!("/api/servers/{sid}/channels"),
        Some(json!({ "name": "two" })),
        Some(&owner),
        None,
    )
    .await;
    let cid2 = r.body["id"].as_i64().unwrap();
    post(&app, &owner, cid2, "elsewhere").await;
    assert_eq!(unread(&app, &member, sid, cid).await, (2, 3));

    // Отметка прочтения: до m2 включительно.
    let r = call(
        &app,
        "POST",
        &read,
        Some(json!({ "messageId": m2 })),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(unread(&app, &member, sid, cid).await, (0, 1));

    // Запоздавший запрос со старым id не откатывает отметку.
    call(
        &app,
        "POST",
        &read,
        Some(json!({ "messageId": 1 })),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(unread(&app, &member, sid, cid).await.0, 0);

    post(&app, &owner, cid, "three").await;
    assert_eq!(unread(&app, &member, sid, cid).await.0, 1);

    // Новичок не видит старую историю как непрочитанную.
    let (late, _) = register_user(&app).await;
    let r = call(
        &app,
        "POST",
        &format!("/api/servers/{sid}/invites"),
        None,
        Some(&owner),
        None,
    )
    .await;
    let code = r.body["code"].as_str().unwrap().to_string();
    let r = call(
        &app,
        "POST",
        &format!("/api/invites/{code}/join"),
        None,
        Some(&late),
        None,
    )
    .await;
    assert_eq!(r.body["unreadCount"], 0);
    assert_eq!(unread(&app, &late, sid, cid).await, (0, 0));

    // Посторонний не может отметить чужой канал.
    let (outsider, _) = register_user(&app).await;
    let r = call(
        &app,
        "POST",
        &read,
        Some(json!({ "messageId": m2 })),
        Some(&outsider),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

/// Ждёт PresenceChanged для `user`, пропуская прочие события.
async fn next_status(ws: &mut Ws, user: i64) -> Presence {
    loop {
        if let ServerMessage::PresenceChanged { user_id, status } = ws.next().await
            && user_id == user
        {
            return status;
        }
    }
}

async fn member_status(app: &Router, token: &str, sid: i64, user: i64) -> Value {
    let r = call(
        app,
        "GET",
        &format!("/api/servers/{sid}/members"),
        None,
        Some(token),
        None,
    )
    .await;
    r.body
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["userId"] == user)
        .unwrap()["status"]
        .clone()
}

#[tokio::test]
async fn presence_across_instances() {
    let (app1, url1) = spawn_instance().await;
    let (app2, url2) = spawn_instance().await;
    let (owner, member, member_id, sid, cid) = setup(&app1).await;

    assert_eq!(
        member_status(&app1, &owner, sid, member_id).await,
        "offline"
    );
    let mut owner_ws = Ws::connect(&url1, &owner).await;

    // Участник открыл вкладку на другом экземпляре.
    let mut tab1 = Ws::connect(&url2, &member).await;
    assert_eq!(
        next_status(&mut owner_ws, member_id).await,
        Presence::Online
    );
    assert_eq!(member_status(&app1, &owner, sid, member_id).await, "online");

    // Одна вкладка бездействует, вторая активна: всё ещё online.
    let mut tab2 = Ws::connect(&url1, &member).await;
    tab1.send(&ClientMessage::SetIdle { idle: true }).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(member_status(&app1, &owner, sid, member_id).await, "online");

    // Обе бездействуют: idle.
    tab2.send(&ClientMessage::SetIdle { idle: true }).await;
    loop {
        if next_status(&mut owner_ws, member_id).await == Presence::Idle {
            break;
        }
    }

    // Все вкладки закрыты: offline.
    drop(tab1);
    drop(tab2);
    loop {
        if next_status(&mut owner_ws, member_id).await == Presence::Offline {
            break;
        }
    }
    assert_eq!(
        member_status(&app2, &owner, sid, member_id).await,
        "offline"
    );

    // Синхронизация прочтения между вкладками одного пользователя.
    let mut owner_tab2 = Ws::connect(&url2, &owner).await;
    let mid = post(&app1, &member, cid, "read me").await;
    call(
        &app1,
        "POST",
        &format!("/api/channels/{cid}/read"),
        Some(json!({ "messageId": mid })),
        Some(&owner),
        None,
    )
    .await;
    loop {
        if let ServerMessage::ChannelRead { channel_id, .. } = owner_tab2.next().await {
            assert_eq!(channel_id, cid);
            break;
        }
    }
}

#[tokio::test]
async fn crashed_instance_goes_offline_after_sweep() {
    let state = test_state().await;
    let app = nexus_server::app(state.clone());
    let (owner, _member, member_id, sid, _) = setup(&app).await;

    // Имитируем подключение упавшего экземпляра: строка есть, пульса нет 5 минут.
    let mut db = state.pool.get().await.unwrap();
    diesel_async::RunQueryDsl::execute(
        diesel::sql_query(format!(
            "INSERT INTO presence_connections (user_id, heartbeat_at) VALUES ({member_id}, now() - interval '5 minutes')"
        )),
        &mut db,
    )
    .await
    .unwrap();
    assert_eq!(
        member_status(&app, &owner, sid, member_id).await,
        "offline",
        "stale row is ignored"
    );

    assert!(nexus_server::presence::sweep(&state).await.unwrap() >= 1);
    let left: i64 = diesel_async::RunQueryDsl::get_result(
        diesel::sql_query(format!(
            "SELECT count(*) AS n FROM presence_connections WHERE user_id = {member_id}"
        )),
        &mut db,
    )
    .await
    .map(|r: Count| r.n)
    .unwrap();
    assert_eq!(left, 0);
}

#[derive(diesel::QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    n: i64,
}
