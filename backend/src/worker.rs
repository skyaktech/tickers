use crate::config::{BodyExpectation, Config, DefaultsConfig, ServiceConfig};
use crate::db;
use crate::notifier::Notifier;
use reqwest::Client;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

pub struct Worker {
    config: Config,
    pool: SqlitePool,
    client: Client,
    notifier: Option<Arc<Notifier>>,
    cancel_token: CancellationToken,
}

impl Worker {
    pub fn new(config: Config, pool: SqlitePool, cancel_token: CancellationToken) -> Self {
        let client = Client::builder()
            .user_agent(concat!("tickers/", env!("CARGO_PKG_VERSION")))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .expect("Failed to build HTTP client");

        // Reuse the same HTTP client for outbound Telegram calls. `None` when disabled.
        let notifier = Notifier::from_config(&config.telegram, client.clone()).map(Arc::new);
        if notifier.is_some() {
            info!(
                chats = config.telegram.chat_ids.len(),
                failure_threshold = config.telegram.failure_threshold,
                "Telegram notifications enabled"
            );
        }

        Self {
            config,
            pool,
            client,
            notifier,
            cancel_token,
        }
    }

    pub fn spawn_all(self) -> Vec<tokio::task::JoinHandle<()>> {
        let mut handles = Vec::new();
        let defaults = self.config.defaults.clone();
        let failure_threshold = self.config.telegram.failure_threshold.max(1);

        for service in &self.config.services {
            let pool = self.pool.clone();
            let client = self.client.clone();
            let token = self.cancel_token.clone();
            let service = service.clone();
            let defaults = defaults.clone();
            let notifier = self.notifier.clone();

            handles.push(tokio::spawn(async move {
                run_check_loop(
                    pool,
                    client,
                    service,
                    defaults,
                    notifier,
                    failure_threshold,
                    token,
                )
                .await;
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
    notifier: Option<Arc<Notifier>>,
    failure_threshold: u32,
    token: CancellationToken,
) {
    let interval = Duration::from_secs(service.effective_check_interval(&defaults));
    let timeout = Duration::from_secs(service.effective_timeout(&defaults));

    info!(
        service_id = %service.id,
        interval_secs = interval.as_secs(),
        "Starting check loop"
    );

    // Per-service transition tracking, seeded from history so a restart doesn't
    // re-alert an already-down service. Only kept when notifications are enabled.
    let mut notify_state = match &notifier {
        Some(_) => Some(NotifyState::seed(&pool, &service.id).await),
        None => None,
    };

    let outcome = perform_check(&pool, &client, &service, timeout).await;
    if let (Some(n), Some(st)) = (&notifier, notify_state.as_mut()) {
        process_outcome(n, &service, failure_threshold, &outcome, st).await;
    }

    loop {
        tokio::select! {
            _ = tokio::time::sleep(interval) => {
                let outcome = perform_check(&pool, &client, &service, timeout).await;
                if let (Some(n), Some(st)) = (&notifier, notify_state.as_mut()) {
                    process_outcome(n, &service, failure_threshold, &outcome, st).await;
                }
            }
            _ = token.cancelled() => {
                info!(service_id = %service.id, "Check loop shutting down");
                return;
            }
        }
    }
}

/// Outcome of a single check, surfaced to the notification state machine.
struct CheckOutcome {
    is_up: bool,
    error_message: Option<String>,
}

/// Per-service notification state: how many checks have failed in a row, whether a
/// DOWN alert has already been sent for the current outage, and when it began.
struct NotifyState {
    consecutive_failures: u32,
    alerted_down: bool,
    down_since: Option<Instant>,
}

impl NotifyState {
    /// Seeds `alerted_down` from the last persisted result so a restart mid-outage
    /// doesn't fire a duplicate DOWN alert (a later recovery still fires). When seeded
    /// down, `down_since` stays unknown, so that recovery omits the duration line.
    async fn seed(pool: &SqlitePool, service_id: &str) -> Self {
        let alerted_down = match db::get_last_is_up(pool, service_id).await {
            Ok(Some(is_up)) => !is_up,
            Ok(None) => false,
            Err(e) => {
                warn!(service_id = %service_id, error = %e, "Failed to seed notification state");
                false
            }
        };
        Self {
            consecutive_failures: 0,
            alerted_down,
            down_since: None,
        }
    }
}

/// Drives DOWN/recovery notifications from a check outcome. Alerts after
/// `failure_threshold` consecutive failures; recovers on the first success.
async fn process_outcome(
    notifier: &Notifier,
    service: &ServiceConfig,
    failure_threshold: u32,
    outcome: &CheckOutcome,
    state: &mut NotifyState,
) {
    if outcome.is_up {
        state.consecutive_failures = 0;
        if state.alerted_down {
            let down_for = state.down_since.map(|t| t.elapsed());
            notifier.notify_recovery(service, down_for).await;
            state.alerted_down = false;
            state.down_since = None;
            info!(service_id = %service.id, "Sent recovery notification");
        }
    } else {
        // Mark the start of a fresh outage (not one we've already alerted/seeded).
        if state.consecutive_failures == 0 && !state.alerted_down {
            state.down_since = Some(Instant::now());
        }
        state.consecutive_failures += 1;
        if !state.alerted_down && state.consecutive_failures >= failure_threshold {
            notifier
                .notify_down(service, outcome.error_message.as_deref())
                .await;
            state.alerted_down = true;
            info!(
                service_id = %service.id,
                failures = state.consecutive_failures,
                "Sent downtime notification"
            );
        }
    }
}

async fn perform_check(
    pool: &SqlitePool,
    client: &Client,
    service: &ServiceConfig,
    timeout: Duration,
) -> CheckOutcome {
    let start = Instant::now();
    let result = client.get(&service.url).timeout(timeout).send().await;
    let elapsed_ms = start.elapsed().as_millis() as i64;

    let (is_up, status_code, error_message) = match result {
        Ok(response) => {
            let status = response.status().as_u16() as i32;
            let status_ok = response.status().as_u16() == service.expected_status;

            let (is_up, error_message) = if !status_ok {
                (
                    false,
                    Some(format!(
                        "Expected status {}, got {}",
                        service.expected_status, status
                    )),
                )
            } else {
                match service.parse_expected_body() {
                    Ok(Some(expectation)) => {
                        let body = response.text().await.unwrap_or_default();
                        match expectation {
                            BodyExpectation::Contains(ref s) => {
                                if body.contains(s.as_str()) {
                                    (true, None)
                                } else {
                                    (false, Some(format!("Body did not contain '{s}'")))
                                }
                            }
                            BodyExpectation::Regex(ref re) => {
                                if re.is_match(&body) {
                                    (true, None)
                                } else {
                                    (false, Some(format!("Body did not match regex '{re}'")))
                                }
                            }
                        }
                    }
                    Ok(None) => (true, None),
                    Err(e) => (false, Some(format!("Invalid regex: {e}"))),
                }
            };

            (is_up, Some(status), error_message)
        }
        Err(err) => {
            let error_msg = if err.is_timeout() {
                format!("Timeout after {}ms", timeout.as_millis())
            } else if err.is_connect() {
                describe_connect_error(&err)
            } else {
                format!("Request failed: {}", root_cause(&err))
            };

            warn!(service_id = %service.id, error = %error_msg, "Health check failed");

            (false, None, Some(error_msg))
        }
    };

    if let Err(e) = db::insert_check_result(
        pool,
        &service.id,
        is_up,
        status_code,
        elapsed_ms,
        error_message.as_deref(),
    )
    .await
    {
        error!(service_id = %service.id, error = %e, "Failed to insert check result");
    }

    CheckOutcome {
        is_up,
        error_message,
    }
}

/// Returns the deepest error in the source chain, formatted as a string.
/// reqwest's top-level `Display` only says "error sending request for url (...)";
/// the actionable cause (e.g. "Connection refused (os error 61)") lives deeper.
fn root_cause(err: &(dyn std::error::Error + 'static)) -> String {
    let mut current: &dyn std::error::Error = err;
    while let Some(source) = current.source() {
        current = source;
    }
    current.to_string()
}

/// Walks the source chain for an underlying `std::io::Error` and returns its
/// `ErrorKind`, which is stable across platforms (unlike OS error numbers/text).
fn io_error_kind(err: &(dyn std::error::Error + 'static)) -> Option<std::io::ErrorKind> {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(err);
    while let Some(e) = current {
        if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
            return Some(io_err.kind());
        }
        current = e.source();
    }
    None
}

/// Turns a reqwest connect-phase error into a specific, human-readable message.
/// DNS and TLS failures aren't surfaced as `io::ErrorKind`s, so they're matched on
/// the root-cause text; the rest key off the underlying socket error kind.
fn describe_connect_error(err: &reqwest::Error) -> String {
    let dyn_err: &(dyn std::error::Error + 'static) = err;
    let cause = root_cause(dyn_err);
    let lower = cause.to_lowercase();

    if lower.contains("certificate")
        || lower.contains("tls")
        || lower.contains("ssl")
        || lower.contains("handshake")
    {
        return format!("TLS error: {cause}");
    }
    if lower.contains("dns")
        || lower.contains("lookup address")
        || lower.contains("name or service not known")
        || lower.contains("nodename nor servname")
        || lower.contains("no such host")
        || lower.contains("name resolution")
    {
        return format!("DNS resolution failed: {cause}");
    }

    let label = match io_error_kind(dyn_err) {
        Some(std::io::ErrorKind::ConnectionRefused) => "Connection refused",
        Some(std::io::ErrorKind::TimedOut) => "Connection timed out",
        Some(std::io::ErrorKind::NetworkUnreachable) => "Network unreachable",
        Some(std::io::ErrorKind::HostUnreachable) => "Host unreachable",
        Some(std::io::ErrorKind::ConnectionReset) => "Connection reset",
        Some(std::io::ErrorKind::ConnectionAborted) => "Connection aborted",
        _ => return format!("Connection failed: {cause}"),
    };

    // Keep the canonical label as the prefix (the frontend chip keys off it),
    // appending the OS detail only when it adds something beyond the label.
    if lower.starts_with(&label.to_lowercase()) {
        cause
    } else {
        format!("{label} — {cause}")
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
