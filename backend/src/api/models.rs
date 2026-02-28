use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub services: Vec<ServiceStatus>,
    pub overall_status: OverallStatus,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ServiceStatus {
    pub id: String,
    pub name: String,
    pub url: String,
    pub is_up: bool,
    pub status_code: Option<i32>,
    pub response_time_ms: i64,
    pub error_message: Option<String>,
    pub last_checked: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OverallStatus {
    AllOperational,
    PartialOutage,
    MajorOutage,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub services: Vec<ServiceHistory>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ServiceHistory {
    pub id: String,
    pub name: String,
    pub buckets: Vec<HistoryBucket>,
}

#[derive(Debug, Serialize)]
pub struct HistoryBucket {
    pub timestamp: String,
    pub total_checks: i64,
    pub successful_checks: i64,
    pub uptime_percentage: f64,
    pub avg_response_time_ms: f64,
}
