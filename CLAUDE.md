# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**tickers** — a self-hosted status page. A Rust backend polls HTTP endpoints, stores results in SQLite, and serves a Leptos/WASM frontend that renders uptime as colored tick marks. Cargo workspace, single deployable binary. See `README.md` for the user-facing config/API reference and `PLAN.md` for the original design.

## Commands

Use the `justfile` — it's the canonical entry point (`just` lists all recipes).

| Task | Command |
|------|---------|
| Pre-commit gate (run before committing) | `just pre-commit` (= fmt + clippy + test) |
| Everything incl. build | `just all` |
| Lint (must be clean — `-D warnings`) | `just clippy` |
| Test | `just test` (= `cargo test`) |
| Single test | `cargo test humanize_formats` (tests currently live only in `backend/src/notifier.rs`) |
| Run backend (serves API + `frontend/dist`) | `just run` (= `cargo run`) |
| Dev w/ frontend hot-reload | `just dev` — backend on :8080, Trunk on :3000 proxying `/api` |
| Build WASM frontend | `just build-frontend` (needs `trunk` + `wasm32-unknown-unknown`) |
| Full release | `just build-release` (frontend bundle → release binary) |

**Toolchain:** Rust 1.94+, edition 2024. Frontend needs `rustup target add wasm32-unknown-unknown` and `cargo install trunk`.

**Gotcha:** plain `cargo build`/`cargo run` does **not** produce the WASM bundle — it only builds the native backend, which then serves whatever is already in `static_dir` (`frontend/dist`). Run `trunk build` (or `just dev`) first or the UI won't load.

## Architecture

Workspace crates: `backend` (binary `tickers`) and `frontend` (cdylib WASM lib). The backend's Axum router serves the compiled frontend as static files, so production is one process.

**Config is the source of truth for services — there is no services table.** Services exist only in `tickers.toml` (path via `TICKERS_CONFIG`, default `tickers.toml`). The DB stores only `check_results` rows keyed by `service_id` string. The API joins config services against their latest DB rows; a configured service with no rows still renders ("No check results yet"), and removing a service from config just stops showing it (its rows linger until purged). When changing what a "service" means, edit `config.rs`, not the schema.

**Data flow:** `worker.rs` → `db::insert_check_result` → SQLite `check_results` → `db.rs` aggregation queries → `api/handlers.rs` JSON → frontend polling.

- **Worker** (`worker.rs`): `Worker::spawn_all()` spawns one independent tokio task per service (its own interval/timeout) plus one purge task (hourly, deletes rows >90 days). All share a `CancellationToken` for graceful shutdown wired up in `main.rs`. One shared `reqwest::Client` (rustls, 5-redirect limit) is reused across all checks and Telegram sends.
- **Checks**: status-code match + optional body check (`expected_body`: plain substring, or `/regex/` / `/regex/i`). Connect failures are classified into human-readable messages (DNS/TLS/refused/...) in `describe_connect_error`; the frontend keys chips off the canonical prefix, so don't change those prefixes casually.
- **Notifications** (`notifier.rs` + `NotifyState` in `worker.rs`): per-service in-memory state machine, seeded on startup from the last persisted `is_up` (`db::get_last_is_up`) so a restart mid-outage doesn't re-alert. DOWN fires after `failure_threshold` consecutive failures; recovery on the first success. `Notifier::from_config` returns `None` when Telegram isn't fully configured (token + ≥1 chat) and the whole path is skipped.
- **API** (`api/`): three read-only endpoints — `/api/status`, `/api/history/hourly`, `/api/history/daily` — each with a `Cache-Control` matched to the frontend poll cadence. Hourly/daily buckets are computed in SQL via `strftime` bucketing (24h / 30d windows). `error.rs` maps `sqlx::Error` → 500 JSON.
- **Router** (`api/mod.rs`): API routes are registered first; everything else falls through to `ServeDir` with an `index.html` fallback (SPA).
- **Frontend** (`frontend/src/`): Leptos CSR. `lib.rs` holds signals and three independent poll loops (status 30s, hourly 5m, daily 30m). Tick color/symbol thresholds live in `components/status_bar.rs` (`tick_class`/`tick_symbol`: green=100, yellow≥95, orange≥50, red>0, purple=0; ✓ at ≥95 else ✗).

## Conventions & gotchas

- **SQLx is runtime-checked** (`query`/`query_as`, not the compile-time macros) — no `DATABASE_URL`, sqlx-cli, or offline prep needed. Migrations are embedded via `sqlx::migrate!("../migrations")` (path is relative to the `backend` crate) and run on startup; SQLite opens in WAL mode, `create_if_missing`.
- Service-ID lists are passed to queries as a JSON array expanded with `json_each(?)` rather than a dynamically-built `IN (...)` clause — follow that pattern for new multi-id queries.
- `checked_at` is stored as ISO-8601 UTC **text**; ties are broken by `id` (`ORDER BY checked_at DESC, id DESC`).
- **Security invariant:** never log a `reqwest::Error` via `Display` in the notifier — its URL embeds the bot token. Use `worker::root_cause(&e)` to log only the deepest source. (See the comment in `notifier.rs::send`.)
- Config uses serde `#[serde(default)]` + per-field `default_*` fns with matching `Default` impls; per-service values override `[defaults]` via `effective_*` helpers. Mirror this when adding config fields.
- Commit style: single-line, conventional prefix (`feat:`, `fix:`, `chore:`, `test:`).
