pub mod handlers;
pub mod models;

use axum::{Router, routing::get};
use tower_http::services::{ServeDir, ServeFile};

use crate::api::handlers::AppState;

pub fn create_router(state: AppState, static_dir: &str) -> Router {
    let api_routes = Router::new()
        .route("/api/status", get(handlers::get_status))
        .route("/api/history/hourly", get(handlers::get_hourly_history))
        .route("/api/history/daily", get(handlers::get_daily_history));

    let serve_dir =
        ServeDir::new(static_dir).fallback(ServeFile::new(format!("{static_dir}/index.html")));

    api_routes.fallback_service(serve_dir).with_state(state)
}
