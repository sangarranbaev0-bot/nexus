//! Общие помощники интеграционных тестов. Нужен PostgreSQL из ../.env (DATABASE_URL).
#![allow(dead_code)] // каждый тестовый файл использует свою часть помощников

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use nexus_server::{AppState, app, events::start_listener, make_pool, run_migrations};
use serde_json::Value;
use tower::ServiceExt;

pub const SECRET: &str = "auth-test-secret-auth-test-secret-auth";

/// Отдельный «экземпляр сервера»: свой пул, свой слушатель событий, общая БД.
pub async fn test_state() -> AppState {
    dotenvy::from_filename("../.env").ok();
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL for integration tests");
    let pool = make_pool(&url).unwrap();
    run_migrations(&pool).await.unwrap();
    let storage =
        nexus_server::attachments::storage_from_env().expect("S3_* for integration tests");
    let state = AppState::new(pool, SECRET, storage);
    start_listener(state.clone(), url).await.unwrap();
    state
}

pub async fn test_app() -> Router {
    app(test_state().await)
}

/// Экземпляр, слушающий настоящий порт (для WebSocket). Возвращает роутер того же
/// экземпляра для HTTP-вызовов и адрес `ws://…/ws`.
pub async fn spawn_instance() -> (Router, String) {
    let router = test_app().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, router.clone()).into_future());
    (router, format!("ws://{addr}/ws"))
}

/// Уникальный суффикс для email и имён: тесты идут параллельно в одной БД.
pub fn unique() -> String {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!(
        "{}{}{n}",
        std::process::id(),
        nexus_server::protocol::now_ms() % 1_000_000
    )
}

pub struct Resp {
    pub status: StatusCode,
    pub cookie: Option<String>,
    pub body: Value,
}

pub async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    auth: Option<&str>,
    cookie: Option<&str>,
) -> Resp {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(t) = auth {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, c);
    }
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    // Берём только пару name=value из Set-Cookie, как это сделает браузер при отправке.
    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Resp {
        status,
        cookie,
        body,
    }
}

/// Регистрирует нового пользователя и возвращает (access-токен, id).
pub async fn register_user(app: &Router) -> (String, i64) {
    let id = unique();
    let body = serde_json::json!({
        "email": format!("u{id}@test.dev"), "username": format!("u{id}"), "password": "password-123"
    });
    let r = call(app, "POST", "/api/auth/register", Some(body), None, None).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    (
        r.body["accessToken"].as_str().unwrap().to_string(),
        r.body["user"]["id"].as_i64().unwrap(),
    )
}

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Тестовый WebSocket-клиент.
pub struct Ws {
    pub stream: WsStream,
    pub hello_user_id: i64,
    pub hello_conn_id: i64,
}

impl Ws {
    pub async fn raw(url: &str) -> Self {
        let (stream, _) = tokio_tungstenite::connect_async(url).await.unwrap();
        Ws {
            stream,
            hello_user_id: 0,
            hello_conn_id: 0,
        }
    }

    /// Подключается, проходит auth и ждёт `hello`.
    pub async fn connect(url: &str, token: &str) -> Self {
        let mut ws = Self::raw(url).await;
        ws.send(&nexus_server::protocol::ClientMessage::Auth {
            token: token.into(),
        })
        .await;
        match ws.next().await {
            nexus_server::protocol::ServerMessage::Hello { user_id, conn_id } => {
                ws.hello_user_id = user_id;
                ws.hello_conn_id = conn_id;
            }
            other => panic!("expected hello, got {other:?}"),
        }
        ws
    }

    pub async fn send(&mut self, msg: &nexus_server::protocol::ClientMessage) {
        use futures_util::SinkExt;
        let text = serde_json::to_string(msg).unwrap();
        self.stream
            .send(tokio_tungstenite::tungstenite::Message::Text(text.into()))
            .await
            .unwrap();
    }

    /// Следующее сообщение сервера; паника, если его нет 5 секунд.
    pub async fn next(&mut self) -> nexus_server::protocol::ServerMessage {
        self.next_within(std::time::Duration::from_secs(5))
            .await
            .expect("no message from server within 5s")
    }

    pub async fn next_within(
        &mut self,
        d: std::time::Duration,
    ) -> Option<nexus_server::protocol::ServerMessage> {
        use futures_util::StreamExt;
        let msg = tokio::time::timeout(d, self.stream.next())
            .await
            .ok()??
            .ok()?;
        match msg {
            tokio_tungstenite::tungstenite::Message::Text(t) => {
                Some(serde_json::from_str(&t).unwrap())
            }
            other => panic!("unexpected frame {other:?}"),
        }
    }
}
