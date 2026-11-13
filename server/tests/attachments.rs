//! Файлы в сообщениях. Нужны PostgreSQL и S3 из ../.env (DATABASE_URL, S3_*).

mod common;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use common::{call, register_user, test_app};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR fake image body";

/// Загружает файл как браузер: multipart с полем `file`.
async fn upload(
    app: &Router,
    token: &str,
    cid: i64,
    name: &str,
    bytes: &[u8],
) -> (StatusCode, Value) {
    let boundary = "nexusboundary";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\n\
         Content-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let req = Request::post(format!("/api/channels/{cid}/attachments"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// GET без авторизации, как <img src>: (статус, заголовки, тело).
async fn fetch(app: &Router, url: &str) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let res = app
        .clone()
        .oneshot(Request::get(url).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let (parts, body) = res.into_parts();
    (
        parts.status,
        parts.headers,
        body.collect().await.unwrap().to_bytes().to_vec(),
    )
}

/// Сервер с каналом и вторым участником: (owner, member, channel_id).
async fn setup(app: &Router) -> (String, String, i64) {
    let (owner, _) = register_user(app).await;
    let (member, _) = register_user(app).await;
    let r = call(
        app,
        "POST",
        "/api/servers",
        Some(json!({ "name": "Files" })),
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
    (owner, member, cid)
}

#[tokio::test]
async fn upload_validate_attach_and_download() {
    let app = test_app().await;
    let (owner, member, cid) = setup(&app).await;
    let (outsider, _) = register_user(&app).await;

    // Тип по содержимому: переименованный exe не проходит, svg/html тоже.
    let (status, body) = upload(&app, &owner, cid, "virus.png", b"MZ\x90\0this is an exe").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"], "unsupported file type");
    let (status, _) = upload(&app, &owner, cid, "x.svg", b"<svg onload=alert(1)>").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Больше 10 МБ — 413.
    let mut big = PNG.to_vec();
    big.resize(10 * 1024 * 1024 + 1, 0);
    let (status, _) = upload(&app, &owner, cid, "big.png", &big).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);

    // Посторонний не грузит в чужой канал.
    let (status, _) = upload(&app, &outsider, cid, "a.png", PNG).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Нормальная загрузка: имя очищено, тип определён сервером.
    let (status, file) = upload(&app, &owner, cid, "../../screenshot 1.png", PNG).await;
    assert_eq!(status, StatusCode::CREATED, "{file}");
    assert_eq!(file["filename"], "screenshot 1.png");
    assert_eq!(file["contentType"], "image/png");
    assert_eq!(file["sizeBytes"], PNG.len());
    let fid = file["id"].as_i64().unwrap();

    // Чужой файл к своему сообщению не прикрепить.
    let msgs = format!("/api/channels/{cid}/messages");
    let r = call(
        &app,
        "POST",
        &msgs,
        Some(json!({ "content": "", "attachmentIds": [fid] })),
        Some(&member),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    // Ни текста, ни файлов — нельзя.
    let r = call(
        &app,
        "POST",
        &msgs,
        Some(json!({ "content": " " })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);

    // Сообщение только из картинки.
    let r = call(
        &app,
        "POST",
        &msgs,
        Some(json!({ "content": "", "attachmentIds": [fid] })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let mid = r.body["id"].as_i64().unwrap();
    assert_eq!(r.body["attachments"][0]["id"], fid);
    // Повторно этот же файл уже не прикрепить.
    let r = call(
        &app,
        "POST",
        &msgs,
        Some(json!({ "content": "again", "attachmentIds": [fid] })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);

    // Участник видит вложение в ленте и скачивает его без заголовка авторизации.
    let r = call(&app, "GET", &msgs, None, Some(&member), None).await;
    let url = r
        .body
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == mid)
        .unwrap()["attachments"][0]["url"]
        .as_str()
        .unwrap()
        .to_string();
    let (status, headers, bytes) = fetch(&app, &url).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(bytes, PNG);
    assert_eq!(headers[header::CONTENT_TYPE], "image/png");
    assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    assert!(
        headers[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .starts_with("inline")
    );

    // Подделанная или просроченная подпись — 403.
    let tampered = url.replace("sig=", "sig=00");
    assert_eq!(fetch(&app, &tampered).await.0, StatusCode::FORBIDDEN);
    let other_id = url.replacen(
        &format!("/attachments/{fid}/"),
        &format!("/attachments/{}/", fid + 1),
        1,
    );
    assert_eq!(fetch(&app, &other_id).await.0, StatusCode::FORBIDDEN);
    let expired = format!("/api/attachments/{fid}/x.png?exp=1&sig=00");
    assert_eq!(fetch(&app, &expired).await.0, StatusCode::FORBIDDEN);

    // Текст можно стереть, если остаётся картинка.
    let r = call(
        &app,
        "PATCH",
        &format!("/api/messages/{mid}"),
        Some(json!({ "content": "" })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);

    // Удаление сообщения удаляет и файл: ссылка перестаёт работать сразу.
    let r = call(
        &app,
        "DELETE",
        &format!("/api/messages/{mid}"),
        None,
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(fetch(&app, &url).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn pdf_downloads_as_attachment_and_gc_removes_orphans() {
    let state = common::test_state().await;
    let app = nexus_server::app(state.clone());
    let (owner, _, cid) = setup(&app).await;

    let (status, pdf) = upload(&app, &owner, cid, "отчёт.pdf", b"%PDF-1.7 tiny").await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, headers, _) = fetch(&app, pdf["url"].as_str().unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    let disposition = headers[header::CONTENT_DISPOSITION].to_str().unwrap();
    assert!(disposition.starts_with("attachment"), "{disposition}");
    assert!(
        disposition.contains("%D0%BE%D1%82"),
        "utf-8 filename: {disposition}"
    );

    // Загрузка, так и не отправленная за сутки, убирается уборкой.
    let mut db = state.pool.get().await.unwrap();
    diesel_async::RunQueryDsl::execute(
        diesel::sql_query(format!(
            "UPDATE attachments SET created_at = now() - interval '2 days' WHERE id = {}",
            pdf["id"]
        )),
        &mut db,
    )
    .await
    .unwrap();
    drop(db);
    assert!(nexus_server::attachments::gc(&state).await.unwrap() >= 1);
    assert_eq!(
        fetch(&app, pdf["url"].as_str().unwrap()).await.0,
        StatusCode::NOT_FOUND
    );
}
