# Crypto-asset backend

Minimal Rust service that reads the deployed `ResearchEuroEMT` contract through Ethereum JSON-RPC and exposes a frontend-oriented HTTP API.

This first integration slice is deliberately read-only: no private key is loaded and the service cannot mint, burn, pause, or freeze tokens.

## Data flow

```text
ResearchEuroEMT contract
        │ eth_call through JSON-RPC
        ▼
AlloyTokenReader (infrastructure)
        │ TokenReader port
        ▼
TokenQueryService (application)
        │ TokenSnapshot domain model
        ▼
GET /api/v1/token (HTTP API)
```

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

- `tokio`: asynchronous runtime used by the HTTP server and RPC client;
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
| `RUST_LOG` | no | `info` | tracing filter |

## Run the complete local flow

Start a persistent Hardhat node in the token project:

```powershell
cd ..\assets\EMT\euro-emt-20
npx hardhat node
```

In a second terminal, deploy the token to that node:

```powershell
npx hardhat ignition deploy ignition/modules/ResearchEuroEMT.ts --network localhost
```

Copy the deployed address and start the backend in a third terminal:

```powershell
cd ..\..\..\backend
$env:TOKEN_ADDRESS="0x..."
cargo run
```

Read the API:

```powershell
Invoke-RestMethod http://127.0.0.1:3000/health
Invoke-RestMethod http://127.0.0.1:3000/api/v1/token
```

Example response before any mint:

```json
{
  "contractAddress": "0x...",
  "name": "Research Euro EMT",
  "symbol": "rEUR",
  "decimals": 6,
  "totalSupplyRaw": "0"
}
```

`totalSupplyRaw` is a decimal string in the token's smallest unit. With six decimals, `1000000` means `1 rEUR`. A string is used because an Ethereum `uint256` can exceed JavaScript's safe integer range.

## Development feedback loop

```powershell
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

The unit test replaces blockchain infrastructure with a `StubTokenReader`. This demonstrates that application logic can be tested without running Hardhat.
