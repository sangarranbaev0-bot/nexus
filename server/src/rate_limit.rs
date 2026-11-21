use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use axum::http::{HeaderMap, StatusCode};

const WINDOW: Duration = Duration::from_secs(60);

static LIMITS: Mutex<Option<HashMap<String, (Instant, u32)>>> = Mutex::new(None);

/// Простой in-memory лимит: N запросов в минуту с одного ключа (IP или User-Agent).
pub fn check(headers: &HeaderMap, key: &str, max: u32) -> Result<(), (StatusCode, String)> {
    let identity = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)
        .or_else(|| headers.get("user-agent").and_then(|v| v.to_str().ok()))
        .unwrap_or("unknown")
        .to_string();
    let id = format!("{key}:{identity}");
    let now = Instant::now();
    let mut guard = LIMITS.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    let entry = map.entry(id).or_insert((now, 0));
    if now.duration_since(entry.0) > WINDOW {
        *entry = (now, 0);
    }
    entry.1 += 1;
    if entry.1 > max {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "too many requests, try again later".into(),
        ));
    }
    if map.len() > 10_000 {
        map.retain(|_, (t, _)| now.duration_since(*t) <= WINDOW);
    }
    Ok(())
}

pub fn reset() {
    let mut guard = LIMITS.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
}

pub fn auth(headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    check(headers, "auth", 10)
}

pub fn messages(headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    check(headers, "messages", 30)
}

pub fn sticker_create(headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    check(headers, "sticker_create", 5)
}
