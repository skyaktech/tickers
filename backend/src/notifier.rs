use crate::config::{ServiceConfig, TelegramConfig};
use reqwest::Client;
use std::time::Duration;
use tracing::{error, warn};

/// Cap on a single Telegram API call so a slow/hung send can't stall a check loop.
const SEND_TIMEOUT: Duration = Duration::from_secs(10);

/// Sends downtime/recovery messages to one or more Telegram chats via the Bot API.
/// One bot (token) per tickers instance; every notification fans out to all `chat_ids`.
pub struct Notifier {
    client: Client,
    bot_token: String,
    chat_ids: Vec<String>,
}

impl Notifier {
    /// Returns `None` when Telegram isn't fully configured, so callers simply skip notifying.
    pub fn from_config(cfg: &TelegramConfig, client: Client) -> Option<Self> {
        if !cfg.is_enabled() {
            return None;
        }
        Some(Self {
            client,
            bot_token: cfg.bot_token.clone(),
            chat_ids: cfg.chat_ids.clone(),
        })
    }

    pub async fn notify_down(&self, service: &ServiceConfig, error: Option<&str>) {
        self.send(&format_down(service, error, &now_utc())).await;
    }

    pub async fn notify_recovery(&self, service: &ServiceConfig, down_for: Option<Duration>) {
        self.send(&format_recovery(service, down_for, &now_utc()))
            .await;
    }

    /// Posts `text` to every configured chat. Sends are plain text (no `parse_mode`),
    /// so message content needs no Markdown/HTML escaping. A failure on one chat is
    /// logged and does not stop delivery to the others.
    async fn send(&self, text: &str) {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);
        for chat_id in &self.chat_ids {
            let body = serde_json::json!({ "chat_id": chat_id, "text": text });
            match self
                .client
                .post(&url)
                .timeout(SEND_TIMEOUT)
                .json(&body)
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {}
                Ok(resp) => {
                    let status = resp.status();
                    let detail = resp.text().await.unwrap_or_default();
                    warn!(%chat_id, %status, detail = %detail, "Telegram API returned an error");
                }
                // Log only the deepest source: reqwest's `Display` appends the request
                // URL, which embeds the bot token. Never log `%e` here.
                Err(e) => {
                    let cause = crate::worker::root_cause(&e);
                    error!(%chat_id, error = %cause, "Failed to send Telegram notification");
                }
            }
        }
    }
}

fn now_utc() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string()
}

fn format_down(service: &ServiceConfig, error: Option<&str>, timestamp: &str) -> String {
    format!(
        "🔴 {} is DOWN\n{}\n{}\n{}",
        service.name,
        service.url,
        error.unwrap_or("Check failed"),
        timestamp,
    )
}

fn format_recovery(service: &ServiceConfig, down_for: Option<Duration>, timestamp: &str) -> String {
    let mut msg = format!("✅ {} recovered\n", service.name);
    if let Some(d) = down_for {
        msg.push_str(&format!("was down for {}\n", humanize(d)));
    }
    msg.push_str(timestamp);
    msg
}

/// Coarse, human-readable duration: "45s", "4m 12s", "1h 3m".
fn humanize(d: Duration) -> String {
    let total = d.as_secs();
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_service() -> ServiceConfig {
        ServiceConfig {
            id: "example".into(),
            name: "Example Service".into(),
            url: "https://api.example.com/health".into(),
            expected_status: 200,
            check_interval: None,
            timeout: None,
            expected_body: None,
        }
    }

    #[test]
    fn humanize_formats() {
        assert_eq!(humanize(Duration::from_secs(0)), "0s");
        assert_eq!(humanize(Duration::from_secs(45)), "45s");
        assert_eq!(humanize(Duration::from_secs(4 * 60 + 12)), "4m 12s");
        assert_eq!(humanize(Duration::from_secs(3600 + 3 * 60)), "1h 3m");
    }

    #[test]
    fn down_message_matches_format() {
        let msg = format_down(
            &sample_service(),
            Some("Timeout after 10000ms"),
            "2026-06-03 14:32 UTC",
        );
        assert_eq!(
            msg,
            "🔴 Example Service is DOWN\n\
             https://api.example.com/health\n\
             Timeout after 10000ms\n\
             2026-06-03 14:32 UTC"
        );
    }

    #[test]
    fn down_message_falls_back_without_error() {
        let msg = format_down(&sample_service(), None, "2026-06-03 14:32 UTC");
        assert!(msg.contains("Check failed"));
    }

    #[test]
    fn recovery_message_with_duration() {
        let msg = format_recovery(
            &sample_service(),
            Some(Duration::from_secs(4 * 60 + 12)),
            "2026-06-03 14:36 UTC",
        );
        assert_eq!(
            msg,
            "✅ Example Service recovered\n\
             was down for 4m 12s\n\
             2026-06-03 14:36 UTC"
        );
    }

    #[test]
    fn recovery_message_omits_unknown_duration() {
        let msg = format_recovery(&sample_service(), None, "2026-06-03 14:36 UTC");
        assert_eq!(
            msg,
            "✅ Example Service recovered\n2026-06-03 14:36 UTC"
        );
    }
}
