# Tickers

A lightweight, self-hosted status page that monitors HTTP endpoints and displays uptime with tick mark (&#10003;/&#10007;) UI.

**[ticke.rs](https://ticke.rs)**

<!-- ![Tickers Screenshot](docs/screenshot.png) -->

## Features

- **HTTP endpoint monitoring** with configurable check intervals and timeouts
- **Tick mark UI** — checkmarks (&#10003;) and crosses (&#10007;) colored by uptime percentage
- **24-hour hourly** and **30-day daily** history rows per service
- **Color-coded uptime**: green (100%) → yellow (≥95%) → orange (≥50%) → red (>0%) → purple (0%)
- **Single binary** — Rust backend serves the WASM frontend as static files
- **SQLite** storage with WAL mode — no external database required
- **TOML configuration** with sensible defaults and per-service overrides
- **Docker-ready** with multi-stage build and compose support
- **Lightweight** — optimized release binary with `opt-level = "z"` and LTO

## Quick Start

### Docker Compose (recommended)

1. Create a `tickers.toml` configuration file:

   ```toml
   [server]
   port = 8080

   [defaults]
   check_interval = 60  # seconds
   timeout = 10         # seconds

   [[services]]
   id = "my-api"
   name = "My API"
   url = "https://api.example.com/health"
   expected_status = 200
   ```

2. Create a `docker-compose.yml`:

   ```yaml
   services:
     tickers:
       image: ghcr.io/skyaktech/tickers:latest
       ports:
         - "8080:8080"
       volumes:
         - tickers-data:/app/data
         - ./tickers.toml:/app/tickers.toml:ro
       restart: unless-stopped

   volumes:
     tickers-data:
   ```

3. Start:

   ```sh
   docker compose up -d
   ```

4. Open `http://localhost:8080` in your browser.

### Docker

```sh
docker run -d \
  -p 8080:8080 \
  -v ./tickers.toml:/app/tickers.toml:ro \
  -v tickers-data:/app/data \
  ghcr.io/skyaktech/tickers:latest
```

## Configuration

Tickers is configured via a TOML file. Set the path with the `TICKERS_CONFIG` environment variable (defaults to `tickers.toml` in the working directory).

### Full Reference

```toml
[server]
port = 8080                      # HTTP server port (default: 8080)
static_dir = "./frontend/dist"   # Path to frontend assets (default: "./frontend/dist")

[database]
url = "sqlite://data/tickers.db" # SQLite database URL (default: "sqlite://data/tickers.db")

[defaults]
check_interval = 60              # Default check interval in seconds (default: 60)
timeout = 10                     # Default request timeout in seconds (default: 10)

[[services]]
id = "my-api"                    # Unique service identifier (required)
name = "My API"                  # Display name (required)
url = "https://api.example.com"  # URL to monitor (required)
expected_status = 200            # Expected HTTP status code (default: 200)
check_interval = 30              # Override default interval for this service
timeout = 5                      # Override default timeout for this service
```

### Service Configuration

Each `[[services]]` entry defines a monitored endpoint:

| Field            | Required | Default           | Description                        |
|------------------|----------|-------------------|------------------------------------|
| `id`             | Yes      | —                 | Unique identifier for the service  |
| `name`           | Yes      | —                 | Display name shown in the UI       |
| `url`            | Yes      | —                 | HTTP(S) URL to monitor             |
| `expected_status`| No       | `200`             | Expected HTTP response status code |
| `check_interval` | No       | from `[defaults]` | Check interval in seconds          |
| `timeout`        | No       | from `[defaults]` | Request timeout in seconds         |

## Development

### Prerequisites

- **Rust 1.85+** (edition 2024)
- **wasm32-unknown-unknown** target: `rustup target add wasm32-unknown-unknown`
- **Trunk**: `cargo install trunk`

### Running Locally

1. Start the backend:

   ```sh
   cd backend
   cargo run
   ```

2. In a separate terminal, start the frontend dev server:

   ```sh
   cd frontend
   trunk serve
   ```

   The frontend runs on `http://localhost:3000` and proxies `/api` requests to the backend on port 8080.

### Building for Production

```sh
# Build frontend WASM
cd frontend
trunk build --release

# Build backend binary
cd ../backend
cargo build --release
```

The release binary is at `target/release/tickers`. It serves the frontend from the `static_dir` configured in `tickers.toml`.

### Docker Build

```sh
docker build -t tickers .
docker run -p 8080:8080 -v ./tickers.toml:/app/tickers.toml:ro tickers
```

## API

All endpoints return JSON.

### `GET /api/status`

Returns the current status of all monitored services.

```json
{
  "services": [
    {
      "id": "my-api",
      "name": "My API",
      "url": "https://api.example.com",
      "is_up": true,
      "status_code": 200,
      "response_time_ms": 142,
      "error_message": null,
      "last_checked": "2026-02-28T12:00:00Z"
    }
  ],
  "overall_status": "all_operational",
  "generated_at": "2026-02-28T12:00:01Z"
}
```

`overall_status` is one of: `all_operational`, `partial_outage`, `major_outage`.

### `GET /api/history/hourly`

Returns hourly aggregated uptime data for the last 24 hours.

```json
{
  "services": [
    {
      "id": "my-api",
      "name": "My API",
      "buckets": [
        {
          "timestamp": "2026-02-28T11:00:00Z",
          "total_checks": 60,
          "successful_checks": 59,
          "uptime_percentage": 98.33,
          "avg_response_time_ms": 145.2
        }
      ]
    }
  ],
  "generated_at": "2026-02-28T12:00:01Z"
}
```

### `GET /api/history/daily`

Returns daily aggregated uptime data for the last 30 days. Same response shape as the hourly endpoint, with date-level `timestamp` values (e.g., `"2026-02-28"`).

## Architecture

```
tickers/
├── backend/           # Axum HTTP server + health check workers
│   └── src/
│       ├── main.rs    # Entrypoint, server startup, graceful shutdown
│       ├── config.rs  # TOML configuration parsing with defaults
│       ├── db.rs      # SQLx/SQLite pool, queries, migrations
│       ├── worker.rs  # Per-service check loops + data purge task
│       ├── error.rs   # Error types and Axum error responses
│       └── api/       # HTTP handlers and response models
├── frontend/          # Leptos CSR (WASM) single-page application
│   └── src/
│       ├── lib.rs     # App root, data fetching, tiered polling
│       ├── api.rs     # HTTP client and response types
│       └── components/
├── migrations/        # SQLx SQL migration files
├── Dockerfile         # Multi-stage: frontend WASM → backend binary → slim runtime
└── tickers.toml       # Example configuration
```

The backend spawns one async task per monitored service, each running an independent check loop. A separate task purges data older than 90 days every hour. The frontend compiles to WASM via Trunk and is served as static files by the backend's Axum router, with API routes taking priority over the static file fallback.

## License

This project is licensed under the [MIT License](LICENSE).
