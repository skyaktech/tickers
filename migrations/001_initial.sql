CREATE TABLE IF NOT EXISTS check_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    service_id TEXT NOT NULL,
    is_up BOOLEAN NOT NULL,
    status_code INTEGER,
    response_time_ms INTEGER NOT NULL,
    error_message TEXT,
    checked_at TIMESTAMP NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_check_results_lookup
    ON check_results(service_id, checked_at DESC);
