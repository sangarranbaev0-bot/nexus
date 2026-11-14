pub mod attachments;
pub mod auth;
mod dms;
pub mod error;
pub mod events;
mod friends;
mod messages;
pub mod permissions;
pub mod presence;
pub mod protocol;
mod reactions;
mod reads;
mod roles;
pub mod schema;
pub mod servers;
pub mod voice;
mod ws;

use std::sync::Arc;

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use diesel_async::{
    AsyncMigrationHarness, AsyncPgConnection, RunQueryDsl,
    pooled_connection::{AsyncDieselConnectionManager, deadpool::Pool},
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use tower_http::trace::TraceLayer;

use protocol::{HealthResponse, now_ms};

pub type DbPool = Pool<AsyncPgConnection>;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub jwt: Arc<auth::JwtKeys>,
    pub events: tokio::sync::broadcast::Sender<events::Event>,
    pub storage: Arc<attachments::Storage>,
    /// Ключ подписи ссылок на файлы; выводится из JWT_SECRET, отдельной настройки не нужно.
    pub file_key: Arc<[u8]>,
}

impl AppState {
    pub fn new(pool: DbPool, jwt_secret: &str, storage: attachments::Storage) -> Self {
        use sha2::Digest;
        let file_key = sha2::Sha256::digest(format!("nexus-attachments:{jwt_secret}"));
        Self {
            pool,
            jwt: Arc::new(auth::JwtKeys::new(jwt_secret)),
            events: events::channel(),
            storage: Arc::new(storage),
            file_key: file_key.to_vec().into(),
        }
    }
}

/// Пул создаётся лениво: подключение к БД происходит при первом запросе.
pub fn make_pool(database_url: &str) -> anyhow::Result<DbPool> {
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    Ok(Pool::builder(manager).build()?)
}

pub async fn run_migrations(pool: &DbPool) -> anyhow::Result<()> {
    // Advisory lock: несколько процессов (реплики, параллельные тесты) не накатят миграции дважды.
    const LOCK_ID: i64 = 0x004e_4558_5553; // "NEXUS"
    let mut conn = pool.get().await?;
    diesel::sql_query(format!("SELECT pg_advisory_lock({LOCK_ID})"))
        .execute(&mut conn)
        .await?;

    // Харнесс синхронный, поэтому уводим его с async-потока.
    let (result, mut conn) = tokio::task::spawn_blocking(move || {
        let mut harness = AsyncMigrationHarness::new(conn);
        let result = harness
            .run_pending_migrations(MIGRATIONS)
            .map(|applied| applied.len())
            .map_err(|e| anyhow::anyhow!(e));
        (result, harness.into_inner())
    })
    .await?;

    diesel::sql_query(format!("SELECT pg_advisory_unlock({LOCK_ID})"))
        .execute(&mut conn)
        .await?;
    tracing::info!("migrations applied: {}", result?);
    Ok(())
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/ws", get(ws::handler))
        .merge(auth::routes())
        .merge(servers::routes())
        .merge(roles::routes())
        .merge(messages::routes())
        .merge(reactions::routes())
        .merge(reads::routes())
        .merge(dms::routes())
        .merge(friends::routes())
        .merge(attachments::routes())
        .merge(voice::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let db = match state.pool.get().await {
        Ok(mut conn) => diesel::sql_query("SELECT 1")
            .execute(&mut conn)
            .await
            .is_ok(),
        Err(_) => false,
    };
    let code = if db {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        code,
        Json(HealthResponse {
            db,
            server_time: now_ms(),
        }),
    )
}
