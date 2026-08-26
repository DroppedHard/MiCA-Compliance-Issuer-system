# Crypto-asset backend

## mockBank and reserve coverage

mockBank is a separate process with its own `data/mock-bank-usd.sqlite` database and immutable deposit/withdrawal ledger. It creates the reserve account with `0.00 USD`; it does not invent issuer assets by itself.

On every issuer-backend startup, the issuer reads the current on-chain rUSD supply and initializes its mockBank account to exactly 110% of that liability. The initialization atomically clears the previous balance and sets the target while preserving an audit row containing the previous balance, target and timestamp. This is an explicit demo bootstrap policy, not continuous 110% maintenance: later issuance deposits remain 1:1 and can dilute the initial buffer.

```powershell
cargo run --bin mock-bank
```

Run the main backend in another terminal as before. It polls `http://127.0.0.1:3100` every polling cycle; override that location with `MOCK_BANK_URL`. The facade exposes `GET /api/v1/reserves` and `/api/v1/reserves/stream`.

The same polling result drives the persisted issuer-owned asset assessment. Read it with `GET /api/v1/asset-state` or subscribe to `/api/v1/asset-state/stream` (SSE event `asset-state`). Policy `reserve-coverage-v1` maps coverage of at least 105% to `active`, 100–105% to `warning`, below 100% to `mint_blocked`, and missing evidence to `data_unavailable`. The frontend does not reproduce these rules.

The demo-only administrative command `POST /api/v1/admin/asset-state/wind-down` accepts JSON such as `{"operationId":"wind-down-demo-1","reason":"supervisory wind-down simulation"}`. The backend submits the terminal on-chain command, waits for confirmation, persists `wind_down` and appends the operation ID, reason and transaction hash to `wind_down_audit`. The contract then blocks mint and ordinary transfers while preserving authorised redemption burns. Redeploy the local contract after pulling this contract revision.

Simulate a USD deposit:

```powershell
$body = @{ amountMinor="100000"; reference="demo deposit"; idempotencyKey="deposit-001" } | ConvertTo-Json
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:3100/api/v1/reserve-accounts/reserve-rusd/deposits -ContentType "application/json" -Body $body
```

Simulate a withdrawal:

```powershell
$body = @{ amountMinor="200000"; reference="coverage stress"; idempotencyKey="withdrawal-001" } | ConvertTo-Json
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:3100/api/v1/reserve-accounts/reserve-rusd/withdrawals -ContentType "application/json" -Body $body
```

Amounts use US cents and idempotency keys prevent repeated requests from being applied twice. Reusing a key with different operation data is rejected. mockBank rejects overdrafts. The backend compares cents and the token's six-decimal units using integer arithmetic; `ratioPercent` is only a presentation value. A detected shortfall is informational and does not pause the token.

## Issuer purchase and issuance endpoint

The issuer exposes a durable, idempotent purchase workflow. Creating an order does not mint tokens. Settlement succeeds only after mockBank contains a matching USD deposit, and the contract records the issuance operation identifier so a retry cannot mint twice.

The pre-mint gate deliberately stays simple. The confirmed deposit must match the order amount and reference exactly. The persisted issuer state must be `active` or `warning`; `mint_blocked`, `data_unavailable` and `wind_down` reject settlement before the order enters `minting`. The backend does not calculate projected coverage for a normal purchase because the demo adds USD reserve and rUSD liability at the same 1:1 value. Reserve deterioration is detected by the independent periodic observer.

Issuance and redemption settlement now share the `issuer-operation-gate-v1` application boundary while retaining different policies. Every evaluation is appended to the `issuer_operation_decisions` SQLite table with its evidence and reason. HTTP handlers do not reproduce these rules.

Start by creating the order:

```powershell
$order = @{
  operationId = "casp-purchase-001"
  recipientAddress = "0x...CASP_CORPORATE_WALLET..."
  amountUsdMinor = "25000"
} | ConvertTo-Json
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:3000/api/v1/issuance-orders -ContentType "application/json" -Body $order
```

`25000` means USD 250.00 and becomes `250000000` smallest rUSD units. The response contains `bankIdempotencyKey`. Simulate the matching confirmed bank deposit; its reference must equal the issuance `operationId`:

```powershell
$deposit = @{
  amountMinor = "25000"
  reference = "casp-purchase-001"
  idempotencyKey = "issuance-casp-purchase-001"
} | ConvertTo-Json
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:3100/api/v1/reserve-accounts/reserve-rusd/deposits -ContentType "application/json" -Body $deposit
```

Finally request settlement:

```powershell
Invoke-RestMethod -Method Post http://127.0.0.1:3000/api/v1/issuance-orders/casp-purchase-001/settle
Invoke-RestMethod http://127.0.0.1:3000/api/v1/issuance-orders/casp-purchase-001
```

The relevant states are `awaiting_fiat`, `minting`, `completed` and `failed`. Missing or mismatched fiat returns HTTP 409. Repeating creation with an identical payload returns the same order; repeating it with changed recipient or amount returns HTTP 409. Repeating settlement after completion returns the completed order without minting again.

## ESG daily estimates

The service observes ERC-20 `Transfer` logs on every polling cycle and groups them by transaction hash. Mint and burn are included because ERC-20 represents them with the same event. SQLite stores only a restart checkpoint and daily aggregates; raw blockchain logs are deliberately not copied. The default database is `data/backend-usd.sqlite` and `DATABASE_PATH` can override it.

The hard-coded demonstration methodology lives in `src/config/esg.rs`, including its Cambridge source. It propagates Cambridge's 1.26 / 7.87 / 11.49 GWh annual scenarios into lower, best-guess and upper per-transaction allocations. `/api/v1/esg`, `/api/v1/esg/daily` and `/api/v1/esg/stream` expose estimates, not direct energy measurements or statistical confidence intervals. The current UTC day is provisional; older activity is finalized when the observer reaches a later day.

Seed the optional 17–21 August 2026 demonstration history after setting the same token address used by the backend:

```powershell
$env:TOKEN_ADDRESS="0x5FbDB2315678afecb367f032d93F642f64180aa3"
cargo run --bin seed-esg-demo
```

The command is idempotent and never replaces an existing day. Seeded rows carry the `demoSeed` origin in the API.

Minimal Rust service that polls the deployed `ResearchUsdEMT` contract through Ethereum JSON-RPC, retains one day of observations in memory, and exposes the latest cached observation through a frontend-oriented HTTP API.

The future Embedded Compliance volume gate uses a hardcoded `1 USD = 1 EUR`
conversion (`eur-usd-fixed-parity-demo-v1`). This is an intentionally strong
demo simplification, not a market/ECB rate or production FX methodology. The
constant and exact integer conversion live in `src/config/compliance.rs`.

The monitoring paths remain read-only, but issuance settlement loads an explicitly configured issuer key and can call only the typed issuance adapter implemented by this backend. The configured account must hold `MINTER_ROLE`. Never use a production or funded key in this research demo.

## Data flow

```text
ResearchUsdEMT + local chain
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

Both the issuer binary and mockBank load `.env` from this directory before parsing configuration. Existing process environment variables take precedence. `.env` is gitignored; `.env.example` is the reproducible local-Hardhat template.

| Variable | Required | Default | Meaning |
| --- | --- | --- | --- |
| `TOKEN_ADDRESS` | yes | none | deployed `ResearchUsdEMT` address |
| `RPC_URL` | no | `http://127.0.0.1:8545` | Ethereum-compatible JSON-RPC endpoint |
| `HTTP_ADDRESS` | no | `127.0.0.1:3000` | backend listen address |
| `POLL_INTERVAL_SECONDS` | no | `10` | polling interval; accepted range is 1-10 seconds |
| `RUST_LOG` | no | `info` | tracing filter |
| `DATABASE_PATH` | no | `data/backend-usd.sqlite` | issuer ESG and issuance-operation SQLite database |
| `MOCK_BANK_URL` | no | `http://127.0.0.1:3100` | mockBank base URL |
| `ISSUER_PRIVATE_KEY` | yes | none | local issuer signer holding `MINTER_ROLE`; use only a disposable development key |
| `INITIALIZE_RESERVE_ON_STARTUP` | no | `true` | replace the mockBank balance with 110% of current supply during issuer startup |

mockBank must be running before the issuer backend because reserve initialization is a startup prerequisite. `MOCK_BANK_INITIAL_BALANCE_MINOR` defaults to `0` and only affects creation of a previously absent account; existing balances are replaced by issuer initialization.

For reserve-shortfall and recovery tests, set `INITIALIZE_RESERVE_ON_STARTUP=false` before starting the issuer. This disables only the automatic 110% reset. Reserve polling remains active and reports the actual mockBank value, while issuance still requires a matching confirmed fiat deposit. This makes under-collateralization testable without silently allowing unbacked minting.

## Local setup tutorial

### Prerequisites

- Rust and Cargo installed through rustup;
- the persistent Hardhat node running at `http://127.0.0.1:8545`;
- `ResearchUsdEMT` deployed to that node;
- the deployed contract address copied from Hardhat Ignition output.

Verify the Rust toolchain and project first:

```powershell
rustc --version
cargo --version
cd issuer\backend
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

### Start the complete local flow

In terminal 1, start a persistent Hardhat node:

```powershell
cd issuer\asset
npx hardhat node
```

In terminal 2, deploy the token:

```powershell
npx hardhat ignition deploy ignition/modules/ResearchUsdEMT.ts --network localhost
```

For a fresh default Hardhat node, the ready local `.env` already contains the deterministic contract address and issuer account. In terminal 3, start the backend:

```powershell
cd issuer\backend
cargo run --bin crypto-asset-backend
```

In another terminal start mockBank, which reads the same file:

```powershell
cd issuer\backend
cargo run --bin mock-bank
```

Shell variables can still override `.env` for one process:

```powershell
$env:RPC_URL="http://127.0.0.1:8545"
$env:HTTP_ADDRESS="127.0.0.1:3000"
$env:POLL_INTERVAL_SECONDS="10"
$env:RUST_LOG="info"
$env:ISSUER_PRIVATE_KEY="0x...LOCAL_HARDHAT_PRIVATE_KEY..."
```

Environment variables set with `$env:` apply only to the current PowerShell process and take precedence over values loaded from `.env`.

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
    "symbol": "rUSD",
    "decimals": 6,
    "totalSupplyRaw": "0"
  }
}
```

`observedAtUnixMs` is the backend observation time. `blockNumber` identifies the chain tip observed during that poll. `totalSupplyRaw` is a decimal string in the token's smallest unit. With six decimals, `1000000` means `1 rUSD`. A string is used because an Ethereum `uint256` can exceed JavaScript's safe integer range.

The cache is deliberately process-local for the demo. It can hold approximately 8,640 ten-second observations for one day, has no external service dependency, and is cleared when the backend restarts. The `SnapshotCache` port isolates this choice, so Redis or a database can replace it without changing polling or HTTP logic.

## Issuer redemption API

The CASP uses the following idempotent issuer boundary:

- `POST /api/v1/redemption-orders` creates or returns an order identified by `operationId`;
- `GET /api/v1/redemption-orders/{operationId}` reads its persisted state;
- `POST /api/v1/redemption-orders/{operationId}/settle` burns rUSD from the supplied holder wallet and records the corresponding mockBank USD withdrawal.

The contract correlates every burn with the operation ID and rejects a second burn for the same identifier. The backend persists progress, so a retry after the burn but before the mockBank response resumes the payout without burning again. This is a demo saga across SQLite, Ethereum and mockBank; it is not a distributed ACID transaction.

Reserve coverage does not authorize or reject redemption. Settlement burns `x` rUSD and pays `x` USD, including while the issuer state is `mint_blocked` or `wind_down`. The current experiment assumes sufficient mockBank funds; an overdraft is rejected by mockBank, while the corresponding insolvency process remains outside scope.

Example request for 10 rUSD:

```powershell
$body = @{
  operationId = "issuer-redemption-demo-1"
  holderAddress = "0x...CASP_HOT_ADDRESS..."
  tokenAmountRaw = "10000000"
} | ConvertTo-Json
Invoke-RestMethod -Method Post -ContentType application/json -Body $body http://127.0.0.1:3000/api/v1/redemption-orders
Invoke-RestMethod -Method Post http://127.0.0.1:3000/api/v1/redemption-orders/issuer-redemption-demo-1/settle
```

## Development feedback loop

```powershell
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Tests cover the rolling retention window and the complete in-process boundary from a stubbed blockchain reader through polling and cache to the cached query service.
