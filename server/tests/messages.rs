//! Сообщения: права, пагинация и доставка событий между двумя экземплярами сервера.
//! Нужен PostgreSQL из ../.env (DATABASE_URL).

mod common;

use std::time::Duration;

use axum::{Router, http::StatusCode};
use common::{Ws, call, register_user, spawn_instance, test_app};
use nexus_server::protocol::{ClientMessage, ServerMessage};
use serde_json::{Value, json};

/// Сервер с владельцем и одним участником: (owner, member, member_id, server_id, channel_id).
async fn setup(app: &Router) -> (String, String, i64, i64, i64) {
    let (owner, _) = register_user(app).await;
    let (member, member_id) = register_user(app).await;
    let r = call(
        app,
        "POST",
        "/api/servers",
        Some(json!({ "name": "Chat" })),
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

async fn post(app: &Router, token: &str, cid: i64, text: &str) -> Value {
    let r = call(
        app,
        "POST",
        &format!("/api/channels/{cid}/messages"),
        Some(json!({ "content": text })),
        Some(token),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    r.body
}

fn ids(body: &Value) -> Vec<i64> {
    body.as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_i64().unwrap())
        .collect()
}

#[tokio::test]
async fn permissions_and_cursor_pagination() {
    let app = test_app().await;
    let (owner, member, _, _, cid) = setup(&app).await;
    let (outsider, _) = register_user(&app).await;
    let msgs = format!("/api/channels/{cid}/messages");

    let mut posted = vec![];
    for i in 0..5 {
        let m = post(
            &app,
            if i % 2 == 0 { &owner } else { &member },
            cid,
            &format!("  msg {i}  "),
        )
        .await;
        posted.push(m["id"].as_i64().unwrap());
    }
    let r = call(&app, "GET", &msgs, None, Some(&member), None).await;
    assert_eq!(ids(&r.body), posted, "all messages, oldest first");
    assert_eq!(r.body[0]["content"], "msg 0", "content is trimmed");

    // Курсор: последние 2, затем 2 перед ними, затем остаток.
    let r = call(
        &app,
        "GET",
        &format!("{msgs}?limit=2"),
        None,
        Some(&member),
        None,
    )
    .await;
    assert_eq!(ids(&r.body), posted[3..]);
    let r = call(
        &app,
        "GET",
        &format!("{msgs}?limit=2&before={}", posted[3]),
        None,
        Some(&member),
        None,
    )
    .await;
    assert_eq!(ids(&r.body), posted[1..3]);
    let r = call(
        &app,
        "GET",
        &format!("{msgs}?limit=2&before={}", posted[1]),
        None,
        Some(&member),
        None,
    )
    .await;
    assert_eq!(ids(&r.body), posted[..1]);

    // Посторонний не читает и не пишет.
    assert_eq!(
        call(&app, "GET", &msgs, None, Some(&outsider), None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let r = call(
        &app,
        "POST",
        &msgs,
        Some(json!({ "content": "hi" })),
        Some(&outsider),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let r = call(
        &app,
        "POST",
        &msgs,
        Some(json!({ "content": "   " })),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);

    // Правка: только автор.
    let owner_msg = format!("/api/messages/{}", posted[0]);
    let member_msg = format!("/api/messages/{}", posted[1]);
    let r = call(
        &app,
        "PATCH",
        &owner_msg,
        Some(json!({ "content": "hack" })),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    let r = call(
        &app,
        "PATCH",
        &owner_msg,
        Some(json!({ "content": "edited" })),
        Some(&outsider),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let r = call(
        &app,
        "PATCH",
        &owner_msg,
        Some(json!({ "content": "edited" })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body["content"], "edited");
    assert!(r.body["editedAt"].is_i64());

    // Удаление: member не трогает чужое, owner (admin+) может.
    assert_eq!(
        call(&app, "DELETE", &owner_msg, None, Some(&member), None)
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(&app, "DELETE", &member_msg, None, Some(&owner), None)
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(&app, "DELETE", &owner_msg, None, Some(&owner), None)
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    let r = call(&app, "GET", &msgs, None, Some(&member), None).await;
    assert_eq!(ids(&r.body), posted[2..]);
}

/// Ждёт событие, пропуская посторонние (например, typing или изменение каналов).
async fn expect(ws: &mut Ws, pred: impl Fn(&ServerMessage) -> bool) -> ServerMessage {
    loop {
        let msg = ws.next().await;
        if pred(&msg) {
            return msg;
        }
    }
}

#[tokio::test]
async fn events_cross_instances_and_respect_membership() {
    // Два экземпляра с общей БД: событие из одного должно дойти до сокета другого.
    let (app1, url1) = spawn_instance().await;
    let (app2, url2) = spawn_instance().await;
    let (owner, member, _, sid, cid) = setup(&app1).await;
    let (outsider, _) = register_user(&app1).await;
    let (late, _) = register_user(&app1).await;

    let mut owner_ws = Ws::connect(&url2, &owner).await;
    let mut member_ws = Ws::connect(&url1, &member).await;
    let mut outsider_ws = Ws::connect(&url1, &outsider).await;
    let mut late_ws = Ws::connect(&url2, &late).await; // подключился до вступления

    // Сообщение через экземпляр 1 видно владельцу на экземпляре 2.
    let m = post(&app1, &member, cid, "hello from instance 1").await;
    let mid = m["id"].as_i64().unwrap();
    match expect(&mut owner_ws, |e| {
        matches!(e, ServerMessage::MessageCreated { .. })
    })
    .await
    {
        ServerMessage::MessageCreated { message } => {
            assert_eq!(message.id, mid);
            assert_eq!(message.content, "hello from instance 1");
        }
        _ => unreachable!(),
    }

    // «Печатает…» тоже идёт через NOTIFY.
    member_ws
        .send(&ClientMessage::Typing { channel_id: cid })
        .await;
    match expect(&mut owner_ws, |e| matches!(e, ServerMessage::Typing { .. })).await {
        ServerMessage::Typing {
            channel_id,
            username,
            ..
        } => {
            assert_eq!(channel_id, cid);
            assert!(username.starts_with('u'));
        }
        _ => unreachable!(),
    }

    // Правка и удаление через экземпляр 2.
    let r = call(
        &app2,
        "PATCH",
        &format!("/api/messages/{mid}"),
        Some(json!({ "content": "edited" })),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    expect(
        &mut owner_ws,
        |e| matches!(e, ServerMessage::MessageUpdated { message } if message.content == "edited"),
    )
    .await;
    call(
        &app2,
        "DELETE",
        &format!("/api/messages/{mid}"),
        None,
        Some(&member),
        None,
    )
    .await;
    expect(
        &mut owner_ws,
        |e| matches!(e, ServerMessage::MessageDeleted { message_id, .. } if *message_id == mid),
    )
    .await;

    // Новый канал: участникам приходит ChannelsChanged.
    call(
        &app1,
        "POST",
        &format!("/api/servers/{sid}/channels"),
        Some(json!({ "name": "news" })),
        Some(&owner),
        None,
    )
    .await;
    expect(
        &mut member_ws,
        |e| matches!(e, ServerMessage::ChannelsChanged { server_id } if *server_id == sid),
    )
    .await;

    // Вступивший после подключения начинает получать события без переподключения.
    let r = call(
        &app1,
        "POST",
        &format!("/api/servers/{sid}/invites"),
        None,
        Some(&owner),
        None,
    )
    .await;
    let code = r.body["code"].as_str().unwrap().to_string();
    call(
        &app1,
        "POST",
        &format!("/api/invites/{code}/join"),
        None,
        Some(&late),
        None,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(300)).await; // событие вступления доходит асинхронно
    post(&app1, &owner, cid, "welcome").await;
    expect(
        &mut late_ws,
        |e| matches!(e, ServerMessage::MessageCreated { message } if message.content == "welcome"),
    )
    .await;

    // Посторонний за всё это время не получил ничего.
    assert_eq!(
        outsider_ws.next_within(Duration::from_millis(500)).await,
        None
    );
}
