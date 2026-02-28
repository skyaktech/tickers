use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;

pub async fn init_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::migrate!("../migrations").run(&pool).await?;

    Ok(pool)
}

#[derive(Debug, sqlx::FromRow)]
pub struct CheckResult {
    pub service_id: String,
    pub is_up: bool,
    pub status_code: Option<i32>,
    pub response_time_ms: i64,
    pub error_message: Option<String>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct AggregatedBucket {
    pub service_id: String,
    pub bucket: String,
    pub total_checks: i64,
    pub successful_checks: i64,
    pub avg_response_time_ms: f64,
    pub uptime_percentage: f64,
}

// --- Write operations ---

pub async fn insert_check_result(
    pool: &SqlitePool,
    service_id: &str,
    is_up: bool,
    status_code: Option<i32>,
    response_time_ms: i64,
    error_message: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO check_results (service_id, is_up, status_code, response_time_ms, error_message)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(service_id)
    .bind(is_up)
    .bind(status_code)
    .bind(response_time_ms)
    .bind(error_message)
    .execute(pool)
    .await?;
    Ok(())
}

// --- Read operations ---

pub async fn get_latest_per_service(
    pool: &SqlitePool,
    service_ids: &[String],
) -> Result<Vec<CheckResult>, sqlx::Error> {
    if service_ids.is_empty() {
        return Ok(vec![]);
    }
    let ids_json = serde_json::to_string(service_ids).unwrap();
    sqlx::query_as::<_, CheckResult>(
        r#"
        SELECT cr.service_id, cr.is_up, cr.status_code, cr.response_time_ms, cr.error_message, cr.checked_at
        FROM check_results cr
        INNER JOIN (
            SELECT service_id, MAX(checked_at) as max_checked_at
            FROM check_results
            WHERE service_id IN (SELECT value FROM json_each(?))
            GROUP BY service_id
        ) latest ON cr.service_id = latest.service_id
                 AND cr.checked_at = latest.max_checked_at
        "#,
    )
    .bind(&ids_json)
    .fetch_all(pool)
    .await
}

pub async fn get_hourly_aggregation(
    pool: &SqlitePool,
    service_ids: &[String],
) -> Result<Vec<AggregatedBucket>, sqlx::Error> {
    if service_ids.is_empty() {
        return Ok(vec![]);
    }
    let ids_json = serde_json::to_string(service_ids).unwrap();
    let rows = sqlx::query(
        r#"
        SELECT
            service_id,
            strftime('%Y-%m-%dT%H:00:00Z', checked_at) as bucket,
            COUNT(*) as total_checks,
            SUM(CASE WHEN is_up THEN 1 ELSE 0 END) as successful_checks,
            AVG(response_time_ms) as avg_response_time_ms,
            (CAST(SUM(CASE WHEN is_up THEN 1 ELSE 0 END) AS REAL) / COUNT(*)) * 100.0 as uptime_percentage
        FROM check_results
        WHERE service_id IN (SELECT value FROM json_each(?1))
          AND checked_at >= datetime('now', '-24 hours')
        GROUP BY service_id, strftime('%Y-%m-%dT%H:00:00Z', checked_at)
        ORDER BY service_id, bucket
        "#,
    )
    .bind(&ids_json)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(map_aggregated_row).collect())
}

pub async fn get_daily_aggregation(
    pool: &SqlitePool,
    service_ids: &[String],
) -> Result<Vec<AggregatedBucket>, sqlx::Error> {
    if service_ids.is_empty() {
        return Ok(vec![]);
    }
    let ids_json = serde_json::to_string(service_ids).unwrap();
    let rows = sqlx::query(
        r#"
        SELECT
            service_id,
            strftime('%Y-%m-%d', checked_at) as bucket,
            COUNT(*) as total_checks,
            SUM(CASE WHEN is_up THEN 1 ELSE 0 END) as successful_checks,
            AVG(response_time_ms) as avg_response_time_ms,
            (CAST(SUM(CASE WHEN is_up THEN 1 ELSE 0 END) AS REAL) / COUNT(*)) * 100.0 as uptime_percentage
        FROM check_results
        WHERE service_id IN (SELECT value FROM json_each(?1))
          AND checked_at >= datetime('now', '-30 days')
        GROUP BY service_id, strftime('%Y-%m-%d', checked_at)
        ORDER BY service_id, bucket
        "#,
    )
    .bind(&ids_json)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(map_aggregated_row).collect())
}

pub async fn purge_old_data(pool: &SqlitePool, days: i64) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM check_results WHERE checked_at < datetime('now', ?)")
        .bind(format!("-{days} days"))
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

fn map_aggregated_row(row: &sqlx::sqlite::SqliteRow) -> AggregatedBucket {
    AggregatedBucket {
        service_id: row.get("service_id"),
        bucket: row.get("bucket"),
        total_checks: row.get("total_checks"),
        successful_checks: row.get("successful_checks"),
        avg_response_time_ms: row.get("avg_response_time_ms"),
        uptime_percentage: row.get("uptime_percentage"),
    }
}
