# Show available recipes
default:
    @just --list

# --- Aggregates ---

# Run all checks before commit
all: fmt clippy build test

# Run pre-commit checks
pre-commit: fmt clippy test
    @echo "✅ All pre-commit checks passed!"

# --- Format & lint ---

# Format code
fmt:
    cargo fmt --all

# Check formatting without making changes
fmt-check:
    cargo fmt --all -- --check

# Run clippy lints
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# --- Test ---

# Run tests
test:
    cargo test

# --- Build ---

# Check compilation without producing binaries
check:
    cargo check

# Build the backend binary (debug, native)
build:
    cargo build

# Build the frontend WASM bundle into frontend/dist (requires trunk)
build-frontend:
    cd frontend && trunk build --release

# Full release build: frontend WASM bundle + backend binary (requires trunk)
build-release: build-frontend
    cargo build --release

# --- Run ---

# Run the backend (serves API + frontend/dist on :8080)
run:
    cargo run

# Dev servers: backend on :8080 + trunk hot-reload on :3000 (requires trunk)
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo run &
    backend=$!
    trap 'kill $backend 2>/dev/null' EXIT
    cd frontend && trunk serve

# --- Utility ---

# Clean build artifacts
clean:
    cargo clean
