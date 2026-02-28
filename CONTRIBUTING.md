# Contributing to Tickers

Thank you for considering contributing to Tickers! Here are some guidelines to help you get started.

## Getting Started

### Prerequisites

- Rust 1.85+ (edition 2024)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [Trunk](https://trunkrs.dev/) for building the frontend: `cargo install trunk`
- SQLite3 (usually pre-installed on most systems)

### Development Setup

1. Clone the repository:

   ```sh
   git clone https://github.com/skyaktech/tickers.git
   cd tickers
   ```

2. Copy and edit the config file:

   ```sh
   cp tickers.toml tickers.dev.toml
   # Edit tickers.dev.toml with your test services
   ```

3. Run the backend:

   ```sh
   cd backend
   TICKERS_CONFIG=../tickers.dev.toml cargo run
   ```

4. In a separate terminal, run the frontend dev server:

   ```sh
   cd frontend
   trunk serve
   ```

   The frontend dev server runs on `http://localhost:3000` and proxies API requests to the backend on port 8080 (configured in `frontend/Trunk.toml`).

## Making Changes

1. Fork the repository and create a feature branch from `main`.
2. Make your changes, ensuring the code compiles without warnings.
3. Test your changes locally with the development setup above.
4. Commit with a clear, conventional message (e.g., `feat: add timeout configuration`, `fix: handle empty service list`).
5. Open a pull request against `main`.

## Code Style

- Follow standard Rust formatting (`cargo fmt`).
- Run `cargo clippy` and address any warnings.
- Keep commits focused — one logical change per commit.

## Reporting Issues

- Use [GitHub Issues](https://github.com/skyaktech/tickers/issues) to report bugs or request features.
- Include steps to reproduce for bug reports.
- Check existing issues before opening a new one.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
