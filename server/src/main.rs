use nexus_server::{AppState, app, events::start_listener, make_pool, run_migrations};
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::EnvFilter;
use std::time::Duration;
use diesel_async::RunQueryDsl;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::from_filename("../.env").ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,nexus_server=debug".into()),
        )
        .init();

    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL is not set"))?;
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_default();
    anyhow::ensure!(
        jwt_secret.len() >= 32,
        "JWT_SECRET must be set and at least 32 characters"
    );
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);

    let pool = make_pool(&database_url)?;
    run_migrations(&pool).await?;

    let storage = nexus_server::attachments::storage_from_env()?;
    let state = AppState::new(pool, &jwt_secret, storage);
    start_listener(state.clone(), database_url).await?;
    nexus_server::presence::spawn_sweeper(state.clone());
    nexus_server::attachments::spawn_gc(state.clone());
    tokio::task::spawn(nexus_server::scheduled_messages::spawn_sender_task(state.clone()));
    spawn_vacuum_analyze_task(state.clone());

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!("NexusChat server listening on :{port}");
    let mut router = app(state);
    // В продакшене сервер сам отдаёт собранный клиент: веб и десктоп-приложение открывают один адрес.
    if let Ok(dist) = std::env::var("CLIENT_DIST") {
        let index = std::path::Path::new(&dist).join("index.html");
        router = router.fallback_service(ServeDir::new(&dist).fallback(ServeFile::new(index)));
        tracing::info!("serving client from {dist}");
    }
    let graceful = axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal());
    if let Err(e) = graceful.await {
        tracing::error!("server error: {e}");
    }
    tracing::info!("server shut down complete");
    Ok(())
}

/// Периодический VACUUM ANALYZE каждые 6 часов — поддерживает актуальность планов запросов.
fn spawn_vacuum_analyze_task(state: AppState) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(6 * 60 * 60)).await;
            let pool = state.pool.clone();
            match pool.get().await {
                Ok(mut conn) => {
                    if let Err(e) = diesel::sql_query("VACUUM ANALYZE")
                        .execute(&mut conn)
                        .await
                    {
                        tracing::warn!("VACUUM ANALYZE failed: {e}");
                    } else {
                        tracing::info!("VACUUM ANALYZE completed");
                    }
                }
                Err(e) => {
                    tracing::warn!("VACUUM ANALYZE skipped (pool unavailable): {e}");
                }
            }
        }
    });
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("received shutdown signal");
}
