use leptos::prelude::*;

use crate::api::{ServiceHistory, ServiceStatus};
use crate::components::status_bar::StatusBar;

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

    let error_view = service.error_message.as_ref().map(|msg| {
        view! { <span class="error-message">{msg.clone()}</span> }
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
                    <span class="response-time">{response_time}</span>
                </div>
            </div>

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
