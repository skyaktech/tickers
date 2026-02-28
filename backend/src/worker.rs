use crate::config::{Config, DefaultsConfig, ServiceConfig};
use crate::db;
use reqwest::Client;
use sqlx::SqlitePool;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

pub struct Worker {
    config: Config,
    pool: SqlitePool,
    client: Client,
    cancel_token: CancellationToken,
}

impl Worker {
    pub fn new(config: Config, pool: SqlitePool, cancel_token: CancellationToken) -> Self {
        let client = Client::builder()
            .user_agent("tickers/0.1.0")
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            config,
            pool,
            client,
            cancel_token,
        }
    }

    pub fn spawn_all(self) -> Vec<tokio::task::JoinHandle<()>> {
        let mut handles = Vec::new();
        let defaults = self.config.defaults.clone();

        for service in &self.config.services {
            let pool = self.pool.clone();
            let client = self.client.clone();
            let token = self.cancel_token.clone();
            let service = service.clone();
            let defaults = defaults.clone();

            handles.push(tokio::spawn(async move {
                run_check_loop(pool, client, service, defaults, token).await;
            }));
        }

        let pool = self.pool.clone();
        let token = self.cancel_token.clone();
        handles.push(tokio::spawn(async move {
            run_purge_loop(pool, token).await;
        }));

        handles
    }
}

async fn run_check_loop(
    pool: SqlitePool,
    client: Client,
    service: ServiceConfig,
    defaults: DefaultsConfig,
    token: CancellationToken,
) {
    let interval = Duration::from_secs(service.effective_check_interval(&defaults));
    let timeout = Duration::from_secs(service.effective_timeout(&defaults));

    info!(
        service_id = %service.id,
        interval_secs = interval.as_secs(),
        "Starting check loop"
    );

    perform_check(&pool, &client, &service, timeout).await;

    loop {
        tokio::select! {
            _ = tokio::time::sleep(interval) => {
                perform_check(&pool, &client, &service, timeout).await;
            }
            _ = token.cancelled() => {
                info!(service_id = %service.id, "Check loop shutting down");
                return;
            }
        }
    }
}

async fn perform_check(
    pool: &SqlitePool,
    client: &Client,
    service: &ServiceConfig,
    timeout: Duration,
) {
    let start = Instant::now();
    let result = client.get(&service.url).timeout(timeout).send().await;
    let elapsed_ms = start.elapsed().as_millis() as i64;

    match result {
        Ok(response) => {
            let status = response.status().as_u16() as i32;
            let is_up = response.status().as_u16() == service.expected_status;

            let error_message = if !is_up {
                Some(format!(
                    "Expected status {}, got {}",
                    service.expected_status, status
                ))
            } else {
                None
            };

            if let Err(e) = db::insert_check_result(
                pool,
                &service.id,
                is_up,
                Some(status),
                elapsed_ms,
                error_message.as_deref(),
            )
            .await
            {
                error!(service_id = %service.id, error = %e, "Failed to insert check result");
            }
        }
        Err(err) => {
            let error_msg = if err.is_timeout() {
                format!("Timeout after {}ms", timeout.as_millis())
            } else if err.is_connect() {
                format!("Connection failed: {err}")
            } else {
                format!("Request failed: {err}")
            };

            warn!(service_id = %service.id, error = %error_msg, "Health check failed");

            if let Err(e) = db::insert_check_result(
                pool,
                &service.id,
                false,
                None,
                elapsed_ms,
                Some(&error_msg),
            )
            .await
            {
                error!(service_id = %service.id, error = %e, "Failed to insert check result");
            }
        }
    }
}

async fn run_purge_loop(pool: SqlitePool, token: CancellationToken) {
    let interval = Duration::from_secs(3600);
    loop {
        tokio::select! {
            _ = tokio::time::sleep(interval) => {
                match db::purge_old_data(&pool, 90).await {
                    Ok(deleted) => {
                        if deleted > 0 {
                            info!(deleted_rows = deleted, "Purged old check data");
                        }
                    }
                    Err(e) => error!(error = %e, "Failed to purge old data"),
                }
            }
            _ = token.cancelled() => return,
        }
    }
}
