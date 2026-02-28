use leptos::prelude::*;

use crate::api::HistoryBucket;

fn tick_class(uptime: f64) -> &'static str {
    if uptime >= 100.0 {
        "tick green"
    } else if uptime >= 95.0 {
        "tick yellow"
    } else if uptime >= 50.0 {
        "tick orange"
    } else if uptime > 0.0 {
        "tick red"
    } else {
        "tick purple"
    }
}

fn tick_symbol(uptime: f64) -> &'static str {
    if uptime >= 95.0 {
        "\u{2713}"
    } else {
        "\u{2717}"
    }
}

fn format_tooltip(bucket: &HistoryBucket) -> String {
    format!(
        "{}: {:.1}% uptime, {:.0}ms avg, {} checks",
        bucket.timestamp,
        bucket.uptime_percentage,
        bucket.avg_response_time_ms,
        bucket.total_checks
    )
}

#[component]
pub fn StatusBar(buckets: Vec<HistoryBucket>, expected_count: usize) -> impl IntoView {
    let mut marks: Vec<_> = buckets
        .iter()
        .map(|b| {
            let class = tick_class(b.uptime_percentage);
            let symbol = tick_symbol(b.uptime_percentage);
            let tooltip = format_tooltip(b);
            view! {
                <span class=class title=tooltip>{symbol}</span>
            }
        })
        .collect();

    // Fill remaining slots with gray dots (no data)
    let remaining = expected_count.saturating_sub(marks.len());
    for _ in 0..remaining {
        marks.push(view! {
            <span class="tick gray" title="No data".to_string()>{"\u{00B7}"}</span>
        });
    }

    view! {
        <div class="status-bar">
            {marks}
        </div>
    }
}
