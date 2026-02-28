use leptos::prelude::*;

#[component]
pub fn Header() -> impl IntoView {
    view! {
        <header class="header">
            <h1 class="header-title">"Tickers"</h1>
            <p class="header-subtitle">"Service Status"</p>
        </header>
    }
}
