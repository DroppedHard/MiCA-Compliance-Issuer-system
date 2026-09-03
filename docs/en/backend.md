# Issuer backend and mockBank

The Rust backend observes the contract and mockBank, persists data in SQLite, and provides HTTP and SSE. It owns supply observations, ESG estimates, reserve coverage, issuance and redemption orders, wind-down, address restrictions, and CASP reporting ingestion.

## Source layout

`src/api` is split into router construction, area-specific routes, request models, validators, handlers, and error responses. Handlers delegate use cases to `src/services`; they do not implement SQL or blockchain calls directly.

Business models and rules remain in the domain layer, while port adapters are grouped in `src/infrastructure/sqlite`, `src/infrastructure/blockchain`, `src/infrastructure/bank`, and `src/infrastructure/casp`. This separates issuer logic from persistence and external-system connections.

mockBank is a separate process from the same Rust project. It starts at USD 0, after which the issuer initializes demonstration coverage at 110% of current supply. Later issuance increases reserves and liabilities at 1:1.

Copy `.env.example` to `.env`, configure `TOKEN_ADDRESS`, and run:

```powershell
cargo run --bin mock-bank
cargo run --bin crypto-asset-backend
```

The main API areas are `/api/v1/token`, `/esg`, `/reserves`, `/asset-state`, `issuance-orders`, `redemption-orders`, and administrative reserve, wind-down, and blacklist commands. Issuance waits for matching fiat and is idempotent. A failed post-payment issuance triggers compensation. Redemption burns rUSD and pays the 1:1 nominal USD value.

## Planned OpenAPI support

HTTP contracts are currently maintained manually between the backend, frontend, and CASP. A planned extension is to generate an OpenAPI specification from routes, request and response models, and error types. It is intended to simplify integration and contract verification, but it does not replace the current manually maintained contracts and is not part of the present demonstration implementation.

```powershell
cargo fmt --all --check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```
