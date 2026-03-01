use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use js_sys::Date;
use leptos::prelude::*;
use leptos::web_sys::MouseEvent;

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

fn format_timestamp(ts: &str, is_daily: bool) -> String {
    if let Ok(dt) = ts.parse::<DateTime<Utc>>() {
        if is_daily {
            format!("{} UTC", dt.format("%b %d"))
        } else {
            let end = dt + TimeDelta::hours(1);
            format!("{} \u{2013} {} UTC", dt.format("%b %d, %H:%M"), end.format("%H:%M"))
        }
    } else if is_daily {
        if let Ok(nd) = NaiveDate::parse_from_str(ts, "%Y-%m-%d") {
            format!("{} UTC", nd.format("%b %d"))
        } else {
            ts.to_string()
        }
    } else {
        ts.to_string()
    }
}

fn format_local_timestamp(ts: &str) -> Option<String> {
    let dt = ts.parse::<DateTime<Utc>>().ok()?;
    let millis = dt.timestamp_millis() as f64;
    let js_start = Date::new(&millis.into());
    let end_millis = millis + 3_600_000.0;
    let js_end = Date::new(&end_millis.into());
    let h_start = format!("{:02}:{:02}", js_start.get_hours(), js_start.get_minutes());
    let h_end = format!("{:02}:{:02}", js_end.get_hours(), js_end.get_minutes());
    Some(format!("{} \u{2013} {} local", h_start, h_end))
}

#[derive(Clone)]
struct TooltipData {
    bucket: HistoryBucket,
    x: f64,
    y: f64,
}

#[component]
pub fn StatusBar(
    buckets: Vec<HistoryBucket>,
    expected_count: usize,
    #[prop(into)] label: String,
) -> impl IntoView {
    let is_daily = label == "30 days";
    let (tooltip, set_tooltip) = signal(None::<TooltipData>);

    let mut marks: Vec<AnyView> = buckets
        .iter()
        .map(|b| {
            let class = tick_class(b.uptime_percentage);
            let symbol = tick_symbol(b.uptime_percentage);
            let bucket = b.clone();
            let bucket_leave = b.clone();
            view! {
                <span
                    class=class
                    on:mouseenter=move |ev: MouseEvent| {
                        set_tooltip.set(Some(TooltipData {
                            bucket: bucket.clone(),
                            x: ev.client_x() as f64,
                            y: ev.client_y() as f64,
                        }));
                    }
                    on:mousemove=move |ev: MouseEvent| {
                        set_tooltip.update(|t| {
                            if let Some(data) = t {
                                data.x = ev.client_x() as f64;
                                data.y = ev.client_y() as f64;
                            }
                        });
                    }
                    on:mouseleave=move |_: MouseEvent| {
                        let _ = &bucket_leave;
                        set_tooltip.set(None);
                    }
                >
                    {symbol}
                </span>
            }
            .into_any()
        })
        .collect();

    // Fill remaining slots with gray dots (no data)
    let remaining = expected_count.saturating_sub(marks.len());
    for _ in 0..remaining {
        marks.push(
            view! {
                <span class="tick gray">{"\u{00B7}"}</span>
            }
            .into_any(),
        );
    }

    view! {
        <div class="status-bar">
            {marks}
            {move || tooltip.get().map(|data| {
                let ts = format_timestamp(&data.bucket.timestamp, is_daily);
                let local_ts = if is_daily { None } else { format_local_timestamp(&data.bucket.timestamp) };
                let has_local = local_ts.is_some();
                let uptime = format!("{:.1}% uptime", data.bucket.uptime_percentage);
                let checks = format!("{} checks", data.bucket.total_checks);
                let resp = format!("{:.0}ms avg", data.bucket.avg_response_time_ms);
                let offset = if has_local { 85.0 } else { 70.0 };
                let style = format!(
                    "left: {}px; top: {}px;",
                    data.x,
                    data.y - offset,
                );
                view! {
                    <div class="custom-tooltip" style=style>
                        <div class="tooltip-ts">{ts}</div>
                        {local_ts.map(|lt| view! { <div class="tooltip-local">{lt}</div> })}
                        <div class="tooltip-row">{uptime}" · "{checks}</div>
                        <div class="tooltip-row">{resp}</div>
                    </div>
                }
            })}
        </div>
    }
}
