# Crypto-asset backend

## ESG daily estimates

The service observes ERC-20 `Transfer` logs on every polling cycle and groups them by transaction hash. Mint and burn are included because ERC-20 represents them with the same event. SQLite stores only a restart checkpoint and daily aggregates; raw blockchain logs are deliberately not copied. The default database is `data/backend.sqlite` and `DATABASE_PATH` can override it.

The hard-coded demonstration methodology lives in `src/config/esg.rs`, including its Cambridge source. It propagates Cambridge's 1.26 / 7.87 / 11.49 GWh annual scenarios into lower, best-guess and upper per-transaction allocations. `/api/v1/esg`, `/api/v1/esg/daily` and `/api/v1/esg/stream` expose estimates, not direct energy measurements or statistical confidence intervals. The current UTC day is provisional; older activity is finalized when the observer reaches a later day.

Seed the optional 17–21 August 2026 demonstration history after setting the same token address used by the backend:

```powershell
$env:TOKEN_ADDRESS="0x5FbDB2315678afecb367f032d93F642f64180aa3"
cargo run --bin seed-esg-demo
```

The command is idempotent and never replaces an existing day. Seeded rows carry the `demoSeed` origin in the API.

Minimal Rust service that polls the deployed `ResearchEuroEMT` contract through Ethereum JSON-RPC, retains one day of observations in memory, and exposes the latest cached observation through a frontend-oriented HTTP API.

This first integration slice is deliberately read-only: no private key is loaded and the service cannot mint, burn, pause, or freeze tokens.

## Data flow

```text
ResearchEuroEMT + local chain
        │ JSON-RPC, every 1-10 seconds
        ▼
AlloyTokenReader → ChainPollingService
        │ TokenObservation
        ▼
SnapshotCache → InMemorySnapshotCache (rolling 24 hours)
        │ latest cached observation only
        ▼
CachedTokenQueryService → GET /api/v1/token
                        └→ GET /api/v1/token/stream (SSE)
```

The HTTP request path never calls JSON-RPC. Polling and cache writes happen in a background task. Both `/health` and `/api/v1/token` return HTTP `503 Service Unavailable` until the first successful poll, immediately after a polling error, or when the last success is older than three polling intervals. A later successful poll restores availability.

`GET /api/v1/token/stream` publishes every successful observation as a `token` SSE event and sends keep-alive comments every 15 seconds. SSE is a notification channel, while the HTTP endpoint remains the authoritative bootstrap and recovery read from cache.

## Project structure

```text
src/
├── api/                 HTTP routing and JSON responses
├── application/         use cases and infrastructure-independent ports
├── domain/              normalized data returned by the service
├── infrastructure/      Alloy/Ethereum implementation of the port
├── config.rs            environment configuration parsing
├── lib.rs               library module exports
└── main.rs              composition root and process startup
```

The dependency direction is intentional. The domain does not know about Axum, Alloy, Ethereum RPC, or environment variables. The application layer depends on the `TokenReader` interface. Alloy is confined to infrastructure.

## Dependencies

- `tokio`: asynchronous runtime, background polling timer, and cache locks;
- `axum`: HTTP routing, state extraction, and JSON responses;
- `alloy`: typed Ethereum addresses, providers, ABI generation, and `eth_call`;
- `serde`: JSON serialization of domain data;
- `async-trait`: asynchronous `TokenReader` as a trait object;
- `thiserror`: typed configuration and RPC errors;
- `tracing` and `tracing-subscriber`: structured application logs.

Exact resolved versions are stored in `Cargo.lock` after a successful build.

## Configuration

The service reads environment variables directly; it does not load `.env` automatically.

| Variable | Required | Default | Meaning |
| --- | --- | --- | --- |
| `TOKEN_ADDRESS` | yes | none | deployed `ResearchEuroEMT` address |
| `RPC_URL` | no | `http://127.0.0.1:8545` | Ethereum-compatible JSON-RPC endpoint |
| `HTTP_ADDRESS` | no | `127.0.0.1:3000` | backend listen address |
| `POLL_INTERVAL_SECONDS` | no | `10` | polling interval; accepted range is 1-10 seconds |
| `RUST_LOG` | no | `info` | tracing filter |

## Local setup tutorial

### Prerequisites

- Rust and Cargo installed through rustup;
- the persistent Hardhat node running at `http://127.0.0.1:8545`;
- `ResearchEuroEMT` deployed to that node;
- the deployed contract address copied from Hardhat Ignition output.

Verify the Rust toolchain and project first:

```powershell
rustc --version
cargo --version
cd backend
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

### Start the complete local flow

In terminal 1, start a persistent Hardhat node:

```powershell
cd ..\assets\EMT\euro-emt-20
npx hardhat node
```

In terminal 2, deploy the token:

```powershell
npx hardhat ignition deploy ignition/modules/ResearchEuroEMT.ts --network localhost
```

Copy the deployed address. In terminal 3, start the backend from the repository root:

```powershell
cd backend
$env:TOKEN_ADDRESS="0x..."
cargo run
```

Optional configuration can be set in the same terminal before `cargo run`:

```powershell
$env:RPC_URL="http://127.0.0.1:8545"
$env:HTTP_ADDRESS="127.0.0.1:3000"
$env:POLL_INTERVAL_SECONDS="10"
$env:RUST_LOG="info"
```

Environment variables set with `$env:` apply only to the current PowerShell process. The backend reads them directly and does not automatically load `.env`.

Read the API:

```powershell
Invoke-RestMethod http://127.0.0.1:3000/health
Invoke-RestMethod http://127.0.0.1:3000/api/v1/token
curl.exe -N http://127.0.0.1:3000/api/v1/token/stream
```

Expected behavior:

- `/health` initially returns `503` until the first successful poll, then returns `200`;
- `/api/v1/token` returns the latest cached observation;
- `/api/v1/token/stream` emits a `token` event after every successful poll;
- stopping Hardhat causes polling errors and makes cached HTTP reads unavailable;
- `Ctrl+C` stops the backend and clears its in-memory cache.

Example token response before any mint:

```json
{
  "observedAtUnixMs": 1787270400000,
  "snapshot": {
    "chainId": 31337,
    "blockNumber": 1,
    "contractAddress": "0x...",
    "name": "Research Euro EMT",
    "symbol": "rEUR",
    "decimals": 6,
    "totalSupplyRaw": "0"
  }
}
```

`observedAtUnixMs` is the backend observation time. `blockNumber` identifies the chain tip observed during that poll. `totalSupplyRaw` is a decimal string in the token's smallest unit. With six decimals, `1000000` means `1 rEUR`. A string is used because an Ethereum `uint256` can exceed JavaScript's safe integer range.

The cache is deliberately process-local for the demo. It can hold approximately 8,640 ten-second observations for one day, has no external service dependency, and is cleared when the backend restarts. The `SnapshotCache` port isolates this choice, so Redis or a database can replace it without changing polling or HTTP logic.

## Development feedback loop

```powershell
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Tests cover the rolling retention window and the complete in-process boundary from a stubbed blockchain reader through polling and cache to the cached query service.
