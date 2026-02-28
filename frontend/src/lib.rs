pub mod api;
pub mod components;

use leptos::prelude::*;
use leptos::task::spawn_local;

use components::footer::Footer;
use components::header::Header;
use components::overall_status::OverallStatusBanner;
use components::service_card::ServiceCard;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
pub fn App() -> impl IntoView {
    let (status_data, set_status_data) = signal(None::<api::StatusResponse>);
    let (hourly_data, set_hourly_data) = signal(None::<api::HistoryResponse>);
    let (daily_data, set_daily_data) = signal(None::<api::HistoryResponse>);
    let (error, set_error) = signal(None::<String>);

    // Initial fetch
    spawn_local(async move {
        fetch_all_data(set_status_data, set_hourly_data, set_daily_data, set_error).await;
    });

    // 30-second polling loop
    spawn_local(async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(30_000).await;
            fetch_all_data(set_status_data, set_hourly_data, set_daily_data, set_error).await;
        }
    });

    view! {
        <div class="status-page">
            <Header />
            {move || error.get().map(|e| view! { <div class="error-banner">{e}</div> })}
            {move || status_data.get().map(|data| view! {
                <OverallStatusBanner status=data.overall_status.clone() />
            })}
            {move || status_data.get().map(|status| {
                let hourly = hourly_data.get();
                let daily = daily_data.get();
                status.services.into_iter().map(|svc| {
                    let svc_hourly = hourly.as_ref()
                        .and_then(|h| h.services.iter().find(|s| s.id == svc.id).cloned());
                    let svc_daily = daily.as_ref()
                        .and_then(|d| d.services.iter().find(|s| s.id == svc.id).cloned());
                    view! {
                        <ServiceCard
                            service=svc
                            hourly_history=svc_hourly
                            daily_history=svc_daily
                        />
                    }
                }).collect_view()
            })}
            <Footer />
        </div>
    }
}

async fn fetch_all_data(
    set_status: WriteSignal<Option<api::StatusResponse>>,
    set_hourly: WriteSignal<Option<api::HistoryResponse>>,
    set_daily: WriteSignal<Option<api::HistoryResponse>>,
    set_error: WriteSignal<Option<String>>,
) {
    match api::fetch_status().await {
        Ok(data) => {
            set_status.set(Some(data));
            set_error.set(None);
        }
        Err(e) => set_error.set(Some(format!("Failed to fetch status: {e}"))),
    }
    if let Ok(data) = api::fetch_hourly_history().await {
        set_hourly.set(Some(data));
    }
    if let Ok(data) = api::fetch_daily_history().await {
        set_daily.set(Some(data));
    }
}
