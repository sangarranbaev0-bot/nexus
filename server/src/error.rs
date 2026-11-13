use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::protocol::ApiError;

/// Ошибка обработчика: код и сообщение для клиента. Внутренние детали только в лог.
#[derive(Debug)]
pub struct AppError(pub StatusCode, pub String);

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self(StatusCode::BAD_REQUEST, msg.into())
    }
    pub fn unauthorized() -> Self {
        Self(StatusCode::UNAUTHORIZED, "unauthorized".into())
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self(StatusCode::CONFLICT, msg.into())
    }
    pub fn forbidden() -> Self {
        Self(StatusCode::FORBIDDEN, "forbidden".into())
    }
    pub fn not_found(what: &str) -> Self {
        Self(StatusCode::NOT_FOUND, format!("{what} not found"))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.0, Json(ApiError { error: self.1 })).into_response()
    }
}

/// Любая непредвиденная ошибка превращается в 500 без утечки деталей.
macro_rules! internal_from {
    ($($t:ty),*) => {$(
        impl From<$t> for AppError {
            fn from(e: $t) -> Self {
                tracing::error!("internal error: {e}");
                Self(StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
            }
        }
    )*};
}

internal_from!(
    diesel::result::Error,
    diesel_async::pooled_connection::deadpool::PoolError,
    bcrypt::BcryptError,
    jsonwebtoken::errors::Error,
    tokio::task::JoinError,
    object_store::Error
);
