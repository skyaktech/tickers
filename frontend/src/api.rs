use gloo_net::http::Request;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct StatusResponse {
    pub services: Vec<ServiceStatus>,
    pub overall_status: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceStatus {
    pub id: String,
    pub name: String,
    pub url: String,
    pub is_up: bool,
    pub status_code: Option<i32>,
    pub response_time_ms: i64,
    pub error_message: Option<String>,
    pub last_checked: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HistoryResponse {
    pub services: Vec<ServiceHistory>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceHistory {
    pub id: String,
    pub name: String,
    pub buckets: Vec<HistoryBucket>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HistoryBucket {
    pub timestamp: String,
    pub total_checks: i64,
    pub successful_checks: i64,
    pub uptime_percentage: f64,
    pub avg_response_time_ms: f64,
}

pub async fn fetch_status() -> Result<StatusResponse, String> {
    Request::get("/api/status")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<StatusResponse>()
        .await
        .map_err(|e| e.to_string())
}

pub async fn fetch_hourly_history() -> Result<HistoryResponse, String> {
    Request::get("/api/history/hourly")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<HistoryResponse>()
        .await
        .map_err(|e| e.to_string())
}

pub async fn fetch_daily_history() -> Result<HistoryResponse, String> {
    Request::get("/api/history/daily")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<HistoryResponse>()
        .await
        .map_err(|e| e.to_string())
}
