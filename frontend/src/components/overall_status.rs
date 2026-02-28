use leptos::prelude::*;

#[component]
pub fn OverallStatusBanner(status: String) -> impl IntoView {
    let (label, class) = match status.as_str() {
        "all_operational" => ("All Systems Operational", "banner banner-operational"),
        "partial_outage" => ("Partial System Outage", "banner banner-partial"),
        "major_outage" => ("Major System Outage", "banner banner-major"),
        _ => ("Unknown Status", "banner"),
    };

    view! {
        <div class=class>
            <span class="banner-icon">
                {if status == "all_operational" { "\u{2713}" } else { "\u{2717}" }}
            </span>
            <span class="banner-text">{label}</span>
        </div>
    }
}
