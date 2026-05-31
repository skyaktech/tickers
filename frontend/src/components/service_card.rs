use leptos::prelude::*;
use web_sys::MouseEvent;

use crate::api::{ServiceHistory, ServiceStatus};
use crate::components::status_bar::StatusBar;

/// Best-effort copy to the system clipboard via the async Clipboard API.
/// `navigator.clipboard` only exists in secure contexts (https / localhost);
/// over plain HTTP from a non-localhost host it is `undefined`, and calling
/// `write_text` on it would throw synchronously — so we feature-detect first.
/// Returns whether a write was actually dispatched; a later promise rejection
/// is ignored.
fn copy_to_clipboard(text: &str) -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let clipboard = window.navigator().clipboard();
    if clipboard.is_undefined() || clipboard.is_null() {
        return false;
    }
    let promise = clipboard.write_text(text);
    wasm_bindgen_futures::spawn_local(async move {
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    });
    true
}

#[derive(Clone)]
struct ErrorTooltipData {
    message: String,
    x: f64,
    y: f64,
}

fn short_error_label(raw: &str) -> String {
    let msg = raw.trim();
    if msg.is_empty() {
        return "Error".into();
    }
    if let Some(rest) = msg.strip_prefix("Expected status ") {
        if let Some((_, after_got)) = rest.rsplit_once("got ") {
            let code = after_got.trim().trim_end_matches('.');
            if code.parse::<u16>().is_ok() {
                return format!("HTTP {code}");
            }
        }
        return "HTTP error".into();
    }
    if msg.starts_with("Body did not contain ") || msg.starts_with("Body did not match regex ") {
        return "Body mismatch".into();
    }
    if msg.starts_with("Invalid regex:") {
        return "Config error".into();
    }
    if msg.starts_with("Timeout after ") || msg.starts_with("Connection timed out") {
        return "Timeout".into();
    }
    if msg.starts_with("Connection refused") {
        return "Refused".into();
    }
    if msg.starts_with("Connection reset") {
        return "Conn reset".into();
    }
    if msg.starts_with("Connection aborted") {
        return "Aborted".into();
    }
    if msg.starts_with("DNS resolution failed") {
        return "DNS error".into();
    }
    if msg.starts_with("Network unreachable") || msg.starts_with("Host unreachable") {
        return "Unreachable".into();
    }
    if msg.starts_with("TLS error") {
        return "TLS error".into();
    }
    if msg.starts_with("Connection failed") {
        return "Connection failed".into();
    }
    if msg.starts_with("Request failed:") {
        return "Request failed".into();
    }
    if msg == "No check results yet" {
        return "Pending".into();
    }
    "Error".into()
}

#[component]
pub fn ServiceCard(
    service: ServiceStatus,
    hourly_history: Option<ServiceHistory>,
    daily_history: Option<ServiceHistory>,
) -> impl IntoView {
    let status_class = if service.is_up {
        "status-indicator up"
    } else {
        "status-indicator down"
    };

    let status_symbol = if service.is_up {
        "\u{2713}"
    } else {
        "\u{2717}"
    };

    let response_time = format!("{}ms", service.response_time_ms);

    let uptime_view = daily_history.as_ref().and_then(|d| {
        let total: i64 = d.buckets.iter().map(|b| b.total_checks).sum();
        let successful: i64 = d.buckets.iter().map(|b| b.successful_checks).sum();
        if total > 0 {
            let pct = (successful as f64 / total as f64) * 100.0;
            let class = if pct >= 99.0 {
                "uptime-pct green"
            } else if pct >= 95.0 {
                "uptime-pct yellow"
            } else {
                "uptime-pct red"
            };
            Some(view! { <span class=class>{format!("{:.2}% uptime", pct)}</span> })
        } else {
            None
        }
    });

    let (err_tip, set_err_tip) = signal(None::<ErrorTooltipData>);

    let error_view = service.error_message.as_ref().and_then(|raw| {
        let msg = raw.trim();
        if msg.is_empty() {
            return None;
        }
        let label = short_error_label(msg);
        let full = msg.to_string();
        let full_enter = full.clone();
        let full_click = full.clone();
        Some(view! {
            <span
                class="error-message"
                on:mouseenter=move |ev: MouseEvent| {
                    set_err_tip.set(Some(ErrorTooltipData {
                        message: full_enter.clone(),
                        x: ev.client_x() as f64,
                        y: ev.client_y() as f64,
                    }));
                }
                on:mousemove=move |ev: MouseEvent| {
                    set_err_tip.update(|t| {
                        if let Some(data) = t {
                            data.x = ev.client_x() as f64;
                            data.y = ev.client_y() as f64;
                        }
                    });
                }
                on:mouseleave=move |_: MouseEvent| {
                    set_err_tip.set(None);
                }
                on:click=move |_: MouseEvent| {
                    // Only confirm if the clipboard write was actually dispatched
                    // (no-op in insecure contexts where clipboard is unavailable).
                    if copy_to_clipboard(&full_click) {
                        set_err_tip.update(|t| {
                            if let Some(data) = t {
                                data.message = "Copied!".to_string();
                            }
                        });
                        let restore = full_click.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            gloo_timers::future::TimeoutFuture::new(1000).await;
                            set_err_tip.update(|t| {
                                if let Some(data) = t {
                                    data.message = restore.clone();
                                }
                            });
                        });
                    }
                }
            >
                {label}
            </span>
        })
    });

    view! {
        <div class="service-card">
            <div class="service-header">
                <div class="service-info">
                    <span class=status_class>{status_symbol}</span>
                    <span class="service-name">{service.name.clone()}</span>
                </div>
                <div class="service-meta">
                    {error_view}
                    {uptime_view}
                    <span class="response-time">{response_time}</span>
                </div>
            </div>

            {move || err_tip.get().map(|data| {
                let style = format!("left: {}px; top: {}px;", data.x, data.y - 16.0);
                view! {
                    <div class="error-tooltip" style=style>{data.message}</div>
                }
            })}

            <div class="service-bars">
                <div class="bar-section">
                    <span class="bar-label">"24 hours"</span>
                    <StatusBar
                        buckets=hourly_history.map(|h| h.buckets).unwrap_or_default()
                        expected_count=24
                        label="24 hours"
                    />
                </div>
                <div class="bar-section">
                    <span class="bar-label">"30 days"</span>
                    <StatusBar
                        buckets=daily_history.map(|d| d.buckets).unwrap_or_default()
                        expected_count=30
                        label="30 days"
                    />
                </div>
            </div>
        </div>
    }
}
