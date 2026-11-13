//! Роли, права, категории, переопределения и журнал аудита. Нужен PostgreSQL.

mod common;

use axum::http::StatusCode;
use common::{call, register_user, test_app};
use serde_json::json;

// Те же биты, что в server/src/permissions.rs.
const VIEW_CHANNEL: i64 = 1 << 0;
const SEND_MESSAGES: i64 = 1 << 1;
const MANAGE_MESSAGES: i64 = 1 << 6;
const MANAGE_CHANNELS: i64 = 1 << 7;
const ADMINISTRATOR: i64 = 1 << 25;

/// Регистрирует владельца/участника/постороннего, создаёт сервер, вступление по инвайту.
async fn setup(app: &axum::Router) -> (String, String, String, i64, i64, i64) {
    let (owner, _) = register_user(app).await;
    let (member, member_id) = register_user(app).await;
    let (outsider, _) = register_user(app).await;

    let r = call(
        app,
        "POST",
        "/api/servers",
        Some(json!({ "name": "Guild" })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let sid = r.body["id"].as_i64().unwrap();

    let r = call(
        app,
        "POST",
        &format!("/api/servers/{sid}/invites"),
        None,
        Some(&owner),
        None,
    )
    .await;
    let code = r.body["code"].as_str().unwrap().to_string();
    let r = call(
        app,
        "POST",
        &format!("/api/invites/{code}/join"),
        None,
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);

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
    (owner, member, outsider, sid, cid, member_id)
}

async fn default_role_id(app: &axum::Router, owner: &str, sid: i64) -> i64 {
    let r = call(
        app,
        "GET",
        &format!("/api/servers/{sid}/roles"),
        None,
        Some(owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    r.body
        .as_array()
        .unwrap()
        .iter()
        .find(|role| role["isDefault"].as_bool() == Some(true))
        .expect("@everyone role")["id"]
        .as_i64()
        .unwrap()
}

#[tokio::test]
async fn role_permissions_categories_overwrites_and_audit() {
    let app = test_app().await;
    let (owner, member, outsider, sid, cid, member_id) = setup(&app).await;
    let roles_url = format!("/api/servers/{sid}/roles");

    // Посторонний не видит роли; у сервера есть @everyone.
    assert_eq!(
        call(&app, "GET", &roles_url, None, Some(&outsider), None)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let everyone = default_role_id(&app, &owner, sid).await;

    // Участник без прав не управляет ролями.
    assert_eq!(
        call(
            &app,
            "POST",
            &roles_url,
            Some(json!({ "name": "Nope" })),
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    // Владелец создаёт роль модератора с правом удалять чужие сообщения.
    let r = call(
        &app,
        "POST",
        &roles_url,
        Some(json!({ "name": "Модератор", "color": 0x3498db, "permissions": MANAGE_MESSAGES })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    assert_eq!(r.body["isDefault"].as_bool(), Some(false));
    assert_eq!(r.body["permissions"], MANAGE_MESSAGES);
    let mod_role = r.body["id"].as_i64().unwrap();

    // Нельзя удалить @everyone.
    assert_eq!(
        call(
            &app,
            "DELETE",
            &format!("/api/servers/{sid}/roles/{everyone}"),
            None,
            Some(&owner),
            None
        )
        .await
        .status,
        StatusCode::BAD_REQUEST
    );

    // Назначаем роль участнику и видим её в списке.
    let assign_url = format!("/api/servers/{sid}/members/{member_id}/roles");
    assert_eq!(
        call(
            &app,
            "PUT",
            &assign_url,
            Some(json!({ "roleIds": [mod_role] })),
            Some(&owner),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    let r = call(
        &app,
        "GET",
        &format!("/api/servers/{sid}/members"),
        None,
        Some(&member),
        None,
    )
    .await;
    let assigned = r
        .body
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["userId"] == member_id)
        .unwrap();
    assert_eq!(assigned["roleIds"], json!([mod_role]));

    // Теперь участник может удалить чужое сообщение, но каналы создавать всё ещё нет.
    let r = call(
        &app,
        "POST",
        &format!("/api/channels/{cid}/messages"),
        Some(json!({ "content": "привет" })),
        Some(&owner),
        None,
    )
    .await;
    let mid = r.body["id"].as_i64().unwrap();
    assert_eq!(
        call(
            &app,
            "DELETE",
            &format!("/api/messages/{mid}"),
            None,
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/servers/{sid}/channels"),
            Some(json!({ "name": "mod-room" })),
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    // Владелец расширяет права роли: теперь участник управляет каналами.
    let r = call(
        &app,
        "PATCH",
        &format!("/api/servers/{sid}/roles/{mod_role}"),
        Some(json!({ "permissions": MANAGE_MESSAGES | MANAGE_CHANNELS })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["permissions"], MANAGE_MESSAGES | MANAGE_CHANNELS);
    let r = call(
        &app,
        "POST",
        &format!("/api/servers/{sid}/channels"),
        Some(json!({ "name": "mod-room" })),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);

    // Категории и перенос канала.
    let r = call(
        &app,
        "POST",
        &format!("/api/servers/{sid}/categories"),
        Some(json!({ "name": "Тексты" })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let category_id = r.body["id"].as_i64().unwrap();
    let r = call(
        &app,
        "PATCH",
        &format!("/api/channels/{cid}"),
        Some(json!({ "categoryId": category_id, "position": 5 })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.body);
    let r = call(
        &app,
        "GET",
        &format!("/api/servers/{sid}/channels"),
        None,
        Some(&owner),
        None,
    )
    .await;
    let moved = r
        .body
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == cid)
        .unwrap();
    assert_eq!(moved["categoryId"], category_id);
    assert_eq!(moved["position"], 5);

    // @everyone теряет право писать в канал; роль модератора возвращает его.
    let overwrite_url = format!("/api/channels/{cid}/overwrites");
    assert_eq!(
        call(
            &app,
            "PUT",
            &overwrite_url,
            Some(json!({ "roleId": everyone, "allow": 0, "deny": SEND_MESSAGES })),
            Some(&owner),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/channels/{cid}/messages"),
            Some(json!({ "content": "нельзя" })),
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    // Владелец обходит переопределения.
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/channels/{cid}/messages"),
            Some(json!({ "content": "можно" })),
            Some(&owner),
            None
        )
        .await
        .status,
        StatusCode::CREATED
    );
    assert_eq!(
        call(
            &app,
            "PUT",
            &overwrite_url,
            Some(json!({ "roleId": mod_role, "allow": SEND_MESSAGES, "deny": 0 })),
            Some(&owner),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/channels/{cid}/messages"),
            Some(json!({ "content": "снова можно" })),
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::CREATED
    );

    // Журнал аудита: владелец видит, участник без права — нет.
    let audit_url = format!("/api/servers/{sid}/audit");
    let r = call(&app, "GET", &audit_url, None, Some(&owner), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert!(
        r.body
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["action"] == "role.create"),
        "{}",
        r.body
    );
    assert_eq!(
        call(&app, "GET", &audit_url, None, Some(&member), None)
            .await
            .status,
        StatusCode::FORBIDDEN
    );

    // Удаление роли убирает назначение и права.
    assert_eq!(
        call(
            &app,
            "DELETE",
            &format!("/api/servers/{sid}/roles/{mod_role}"),
            None,
            Some(&owner),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(
            &app,
            "POST",
            &format!("/api/servers/{sid}/channels"),
            Some(json!({ "name": "after-delete" })),
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn role_validation_and_overwrite_targets() {
    let app = test_app().await;
    let (owner, member, outsider, sid, cid, member_id) = setup(&app).await;
    let roles_url = format!("/api/servers/{sid}/roles");
    let everyone = default_role_id(&app, &owner, sid).await;

    for bad in [
        json!({ "name": "" }),
        json!({ "name": "x".repeat(101) }),
        json!({ "name": "Bad", "color": 0x1000000 }),
        json!({ "name": "Bad", "permissions": -1 }),
        json!({ "name": "Bad", "permissions": 1i64 << 40 }),
    ] {
        let r = call(&app, "POST", &roles_url, Some(bad), Some(&owner), None).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.body);
    }

    // @everyone нельзя переименовать, но права менять можно.
    assert_eq!(
        call(
            &app,
            "PATCH",
            &format!("/api/servers/{sid}/roles/{everyone}"),
            Some(json!({ "name": "все" })),
            Some(&owner),
            None
        )
        .await
        .status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(
            &app,
            "PATCH",
            &format!("/api/servers/{sid}/roles/{everyone}"),
            Some(json!({ "permissions": SEND_MESSAGES })),
            Some(&owner),
            None
        )
        .await
        .status,
        StatusCode::OK
    );

    // Переопределение: ровно одна цель, известная роль, ненулевые биты.
    let url = format!("/api/channels/{cid}/overwrites");
    for bad in [
        json!({ "allow": 1, "deny": 0 }),
        json!({ "roleId": everyone, "userId": 1, "allow": 1, "deny": 0 }),
        json!({ "roleId": 999999999, "allow": 1, "deny": 0 }),
        json!({ "roleId": everyone, "allow": 1, "deny": 1 }),
    ] {
        let r = call(&app, "PUT", &url, Some(bad), Some(&owner), None).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", r.body);
    }
    // Посторонний получает 404, участник без MANAGE_CHANNELS — 403.
    assert_eq!(
        call(
            &app,
            "PUT",
            &url,
            Some(json!({ "roleId": everyone, "allow": 1, "deny": 0 })),
            Some(&outsider),
            None
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        call(
            &app,
            "PUT",
            &url,
            Some(json!({ "roleId": everyone, "allow": 1, "deny": 0 })),
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );

    // Персональное переопределение тоже принимается и видно в списке.
    assert_eq!(
        call(
            &app,
            "PUT",
            &url,
            Some(json!({ "userId": member_id, "allow": 2, "deny": 0 })),
            Some(&owner),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    let r = call(&app, "GET", &url, None, Some(&owner), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body.as_array().unwrap().len(), 1);
    assert_eq!(r.body[0]["userId"], member_id);
assert_eq!(r.body[0]["allow"], 2);
}

#[tokio::test]
async fn hidden_channels_and_overwrite_escalation() {
    let app = test_app().await;
    let (owner, member, _outsider, sid, cid, member_id) = setup(&app).await;
    let everyone = default_role_id(&app, &owner, sid).await;
    let overwrite_url = format!("/api/channels/{cid}/overwrites");

    // @everyone теряет доступ к каналу.
    assert_eq!(
        call(
            &app,
            "PUT",
            &overwrite_url,
            Some(json!({ "roleId": everyone, "allow": 0, "deny": VIEW_CHANNEL })),
            Some(&owner),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );

    let channels_url = format!("/api/servers/{sid}/channels");
    let r = call(&app, "GET", &channels_url, None, Some(&member), None).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert!(
        r.body.as_array().unwrap().iter().all(|c| c["id"] != cid),
        "{}",
        r.body
    );
    assert_eq!(
        call(
            &app,
            "GET",
            &format!("/api/channels/{cid}/messages"),
            None,
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    let r = call(&app, "GET", &channels_url, None, Some(&owner), None).await;
    assert!(r.body.as_array().unwrap().iter().any(|c| c["id"] == cid));

    // Категория без видимых каналов тоже скрыта.
    let r = call(
        &app,
        "POST",
        &format!("/api/servers/{sid}/categories"),
        Some(json!({ "name": "Секреты" })),
        Some(&owner),
        None,
    )
    .await;
    let category_id = r.body["id"].as_i64().unwrap();
    call(
        &app,
        "PATCH",
        &format!("/api/channels/{cid}"),
        Some(json!({ "categoryId": category_id })),
        Some(&owner),
        None,
    )
    .await;
    let cats_url = format!("/api/servers/{sid}/categories");
    let r = call(&app, "GET", &cats_url, None, Some(&member), None).await;
    assert!(
        r.body.as_array().unwrap().iter().all(|c| c["id"] != category_id),
        "{}",
        r.body
    );

    // Роль с доступом к каналу возвращает его участнику.
    let roles_url = format!("/api/servers/{sid}/roles");
    let r = call(
        &app,
        "POST",
        &roles_url,
        Some(json!({ "name": "VIP", "permissions": 0 })),
        Some(&owner),
        None,
    )
    .await;
    let vip = r.body["id"].as_i64().unwrap();
    assert_eq!(
        call(
            &app,
            "PUT",
            &overwrite_url,
            Some(json!({ "roleId": vip, "allow": VIEW_CHANNEL, "deny": 0 })),
            Some(&owner),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(
            &app,
            "PUT",
            &format!("/api/servers/{sid}/members/{member_id}/roles"),
            Some(json!({ "roleIds": [vip] })),
            Some(&owner),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    let r = call(&app, "GET", &channels_url, None, Some(&member), None).await;
    assert!(
        r.body.as_array().unwrap().iter().any(|c| c["id"] == cid),
        "{}",
        r.body
    );

    // Участник с MANAGE_CHANNELS не может выдать право, которого у него нет,
    // и не может менять переопределения роли выше себя.
    let r = call(
        &app,
        "POST",
        &roles_url,
        Some(json!({ "name": "Мод", "permissions": MANAGE_CHANNELS })),
        Some(&owner),
        None,
    )
    .await;
    let mod_role = r.body["id"].as_i64().unwrap();
    assert_eq!(
        call(
            &app,
            "PUT",
            &format!("/api/servers/{sid}/members/{member_id}/roles"),
            Some(json!({ "roleIds": [vip, mod_role] })),
            Some(&owner),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(
            &app,
            "PUT",
            &overwrite_url,
            Some(json!({ "roleId": vip, "allow": ADMINISTRATOR, "deny": 0 })),
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    let r = call(
        &app,
        "POST",
        &roles_url,
        Some(json!({ "name": "Высокая" })),
        Some(&owner),
        None,
    )
    .await;
    let high_role = r.body["id"].as_i64().unwrap();
    assert_eq!(
        call(
            &app,
            "PUT",
            &overwrite_url,
            Some(json!({ "roleId": high_role, "allow": VIEW_CHANNEL, "deny": 0 })),
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
    // А @everyone ниже участника — можно, если биты не выше собственных.
    assert_eq!(
        call(
            &app,
            "PUT",
            &overwrite_url,
            Some(json!({ "roleId": everyone, "allow": SEND_MESSAGES, "deny": 0 })),
            Some(&member),
            None
        )
        .await
        .status,
        StatusCode::NO_CONTENT
    );
}
