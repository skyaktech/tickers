# Stage 1: Build frontend (WASM)
FROM rust:1 AS frontend-builder
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY frontend/ frontend/
COPY backend/Cargo.toml backend/Cargo.toml
RUN mkdir -p backend/src && echo "fn main() {}" > backend/src/main.rs

WORKDIR /app/frontend
RUN trunk build --release

# Stage 2: Build backend (native binary)
FROM rust:1 AS backend-builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY backend/ backend/
COPY migrations/ migrations/
COPY frontend/Cargo.toml frontend/Cargo.toml
RUN mkdir -p frontend/src && echo "" > frontend/src/lib.rs

WORKDIR /app/backend
RUN cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=backend-builder /app/target/release/tickers /app/tickers
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist

RUN useradd -m -u 1001 tickers && mkdir -p /app/data && chown -R tickers:tickers /app
COPY tickers.toml /app/tickers.toml

USER tickers
EXPOSE 8080
ENV TICKERS_CONFIG=/app/tickers.toml
CMD ["/app/tickers"]
