//! Реакции: идемпотентность, состав участников и проверки доступа.
//! Нужен PostgreSQL из ../.env (DATABASE_URL).

mod common;

use axum::{Router, http::StatusCode};
use common::{call, register_user, test_app};
use serde_json::{Value, json};

use nexus_server::permissions::{ADD_REACTIONS, VIEW_CHANNEL};

/// Сервер с владельцем и одним участником: (owner, member, member_id, server_id, channel_id).
async fn setup(app: &Router) -> (String, String, i64, i64, i64) {
    let (owner, _) = register_user(app).await;
    let (member, member_id) = register_user(app).await;
    let r = call(
        app,
        "POST",
        "/api/servers",
        Some(json!({ "name": "React" })),
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

async fn post(app: &Router, token: &str, cid: i64) -> i64 {
    let r = call(
        app,
        "POST",
        &format!("/api/channels/{cid}/messages"),
        Some(json!({ "content": "hi" })),
        Some(token),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    r.body["id"].as_i64().unwrap()
}

fn reaction<'a>(body: &'a Value, emoji: &str) -> Option<&'a Value> {
    body["reactions"]
        .as_array()?
        .iter()
        .find(|r| r["emoji"] == emoji)
}

fn users(body: &Value, emoji: &str) -> Vec<i64> {
    reaction(body, emoji)
        .and_then(|r| r["userIds"].as_array())
        .map(|a| a.iter().map(|v| v.as_i64().unwrap()).collect())
        .unwrap_or_default()
}

async fn react(app: &Router, token: &str, id: i64, emoji: &str, method: &str) -> Value {
    let r = call(
        app,
        method,
        &format!("/api/messages/{id}/reactions/{emoji}"),
        None,
        Some(token),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    r.body
}

async fn default_role_id(app: &Router, token: &str, sid: i64) -> i64 {
    let r = call(
        app,
        "GET",
        &format!("/api/servers/{sid}/roles"),
        None,
        Some(token),
        None,
    )
    .await;
    r.body
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["isDefault"] == true)
        .unwrap()["id"]
        .as_i64()
        .unwrap()
}

#[tokio::test]
async fn add_remove_reactions_and_permissions() {
    let app = test_app().await;
    let (owner, member, member_id, sid, cid) = setup(&app).await;
    let (outsider, _) = register_user(&app).await;
    let id = post(&app, &owner, cid).await;

    // Пустой список реакций в свежем сообщении.
    let r = call(
        &app,
        "GET",
        &format!("/api/channels/{cid}/messages"),
        None,
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.body[0]["reactions"], json!([]));

    // Повторная реакция не дублируется.
    let m = react(&app, &member, id, "👍", "PUT").await;
    assert_eq!(users(&m, "👍"), vec![member_id]);
    let m = react(&app, &member, id, "👍", "PUT").await;
    assert_eq!(users(&m, "👍"), vec![member_id]);

    // Второй участник добавляется к той же реакции.
    let m = react(&app, &owner, id, "👍", "PUT").await;
    let ids = users(&m, "👍");
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0], member_id);

    // История с реакциями отдаётся списком.
    let r = call(
        &app,
        "GET",
        &format!("/api/channels/{cid}/messages"),
        None,
        Some(&member),
        None,
    )
    .await;
    assert_eq!(users(&r.body[0], "👍").len(), 2);

    // Снятие убирает только свою реакцию.
    let m = react(&app, &member, id, "👍", "DELETE").await;
    assert_eq!(users(&m, "👍").len(), 1);

    // Посторонний не видит канал — как будто нет сообщения.
    let r = call(
        &app,
        "PUT",
        &format!("/api/messages/{id}/reactions/%F0%9F%94%A5"),
        None,
        Some(&outsider),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND, "{}", r.body);

    // Пустой/пробельный эмодзи не принимается.
    let r = call(
        &app,
        "PUT",
        &format!("/api/messages/{id}/reactions/%20"),
        None,
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.body);

    // Запрет ADD_REACTIONS оверрайдом @everyone закрывает реакцию участнику.
    let everyone = default_role_id(&app, &owner, sid).await;
    assert_eq!(
        call(
            &app,
            "PUT",
            &format!("/api/channels/{cid}/overwrites"),
            Some(json!({ "roleId": everyone, "allow": 0, "deny": ADD_REACTIONS })),
            Some(&owner),
            None,
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    let r = call(
        &app,
        "PUT",
        &format!("/api/messages/{id}/reactions/%F0%9F%94%A5"),
        None,
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.body);

    // Владелец обходит оверрайды (у него ADMINISTRATOR).
    react(&app, &owner, id, "🔥", "PUT").await;

    // Но при запрете VIEW_CHANNEL участник теряет доступ к каналу.
    assert_eq!(
        call(
            &app,
            "PUT",
            &format!("/api/channels/{cid}/overwrites"),
            Some(json!({ "roleId": everyone, "allow": 0, "deny": VIEW_CHANNEL })),
            Some(&owner),
            None,
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    let r = call(
        &app,
        "PUT",
        &format!("/api/messages/{id}/reactions/%F0%9F%91%80"),
        None,
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.body);
}
