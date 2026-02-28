use axum::Json;
use axum::extract::State;
use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;

use crate::api::models::*;
use crate::config::Config;
use crate::db;
use crate::error::AppError;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
}

pub async fn get_status(
    State(state): State<AppState>,
) -> Result<Json<StatusResponse>, AppError> {
    let service_ids: Vec<String> = state.config.services.iter().map(|s| s.id.clone()).collect();
    let latest = db::get_latest_per_service(&state.pool, &service_ids).await?;

    let latest_map: HashMap<_, _> = latest.into_iter().map(|cr| (cr.service_id.clone(), cr)).collect();

    let mut services = Vec::new();
    for svc in &state.config.services {
        if let Some(check) = latest_map.get(&svc.id) {
            services.push(ServiceStatus {
                id: svc.id.clone(),
                name: svc.name.clone(),
                url: svc.url.clone(),
                is_up: check.is_up,
                status_code: check.status_code,
                response_time_ms: check.response_time_ms,
                error_message: check.error_message.clone(),
                last_checked: check.checked_at,
            });
        } else {
            services.push(ServiceStatus {
                id: svc.id.clone(),
                name: svc.name.clone(),
                url: svc.url.clone(),
                is_up: false,
                status_code: None,
                response_time_ms: 0,
                error_message: Some("No check results yet".to_string()),
                last_checked: Utc::now(),
            });
        }
    }

    let overall_status = compute_overall_status(&services);
    Ok(Json(StatusResponse {
        services,
        overall_status,
        generated_at: Utc::now(),
    }))
}

pub async fn get_hourly_history(
    State(state): State<AppState>,
) -> Result<Json<HistoryResponse>, AppError> {
    let service_ids: Vec<String> = state.config.services.iter().map(|s| s.id.clone()).collect();
    let buckets = db::get_hourly_aggregation(&state.pool, &service_ids).await?;
    let response = build_history_response(&state.config, buckets);
    Ok(Json(response))
}

pub async fn get_daily_history(
    State(state): State<AppState>,
) -> Result<Json<HistoryResponse>, AppError> {
    let service_ids: Vec<String> = state.config.services.iter().map(|s| s.id.clone()).collect();
    let buckets = db::get_daily_aggregation(&state.pool, &service_ids).await?;
    let response = build_history_response(&state.config, buckets);
    Ok(Json(response))
}

fn compute_overall_status(services: &[ServiceStatus]) -> OverallStatus {
    if services.is_empty() {
        return OverallStatus::AllOperational;
    }
    let down_count = services.iter().filter(|s| !s.is_up).count();
    if down_count == 0 {
        OverallStatus::AllOperational
    } else if down_count == services.len() {
        OverallStatus::MajorOutage
    } else {
        OverallStatus::PartialOutage
    }
}

fn build_history_response(
    config: &Config,
    buckets: Vec<db::AggregatedBucket>,
) -> HistoryResponse {
    let mut grouped: HashMap<String, Vec<HistoryBucket>> = HashMap::new();

    for bucket in buckets {
        grouped
            .entry(bucket.service_id.clone())
            .or_default()
            .push(HistoryBucket {
                timestamp: bucket.bucket,
                total_checks: bucket.total_checks,
                successful_checks: bucket.successful_checks,
                uptime_percentage: bucket.uptime_percentage,
                avg_response_time_ms: bucket.avg_response_time_ms,
            });
    }

    let services = config
        .services
        .iter()
        .map(|svc| ServiceHistory {
            id: svc.id.clone(),
            name: svc.name.clone(),
            buckets: grouped.remove(&svc.id).unwrap_or_default(),
        })
        .collect();

    HistoryResponse {
        services,
        generated_at: Utc::now(),
    }
}
