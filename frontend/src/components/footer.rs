use leptos::prelude::*;

#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="footer">
            <p>"Powered by "<a href="https://ticke.rs" class="footer-link">"Tickers"</a></p>
        </footer>
    }
}
