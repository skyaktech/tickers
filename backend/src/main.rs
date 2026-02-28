mod api;
mod config;
mod db;
mod error;
mod worker;

use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tickers=debug")),
        )
        .init();

    let config_path =
        std::env::var("TICKERS_CONFIG").unwrap_or_else(|_| "tickers.toml".to_string());
    let config = config::Config::load_or_default(&config_path)?;
    tracing::info!(
        services = config.services.len(),
        port = config.server.port,
        "Configuration loaded"
    );

    let pool = db::init_pool(&config.database.url).await?;
    tracing::info!("Database initialized");

    let cancel_token = CancellationToken::new();

    let worker = worker::Worker::new(config.clone(), pool.clone(), cancel_token.clone());
    let worker_handles = worker.spawn_all();
    tracing::info!("Worker tasks spawned");

    let state = api::handlers::AppState {
        pool: pool.clone(),
        config: Arc::new(config.clone()),
    };
    let app = api::create_router(state, &config.server.static_dir);
    let addr = format!("0.0.0.0:{}", config.server.port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!(address = %addr, "Server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(cancel_token.clone()))
        .await?;

    tracing::info!("Waiting for worker tasks to shut down...");
    for handle in worker_handles {
        let _ = handle.await;
    }

    pool.close().await;
    tracing::info!("Shutdown complete");
    Ok(())
}

async fn shutdown_signal(cancel_token: CancellationToken) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Received Ctrl+C"),
        _ = terminate => tracing::info!("Received SIGTERM"),
    }

    cancel_token.cancel();
}
