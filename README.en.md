# rUSD issuer

[Wersja polska](README.md)

This repository contains a demonstration issuer of the USD-referenced rUSD electronic-money token. It presents issuance and redemption workflows, supply and reserve monitoring, token lifecycle controls, address restrictions, and public token disclosures. It is a research project, not a production implementation or a claim of MiCA compliance.

## Components

- `asset/` — the `ResearchUsdEMT` contract, Hardhat deployment and simulation scripts;
- `backend/` — the Rust issuer API, blockchain observer, SQLite and the separate mockBank process;
- `frontend/` — the issuer administration panel and public information document;
- `docs/` — detailed Polish and English documentation;
- `compose.yaml` — a self-contained local issuer deployment.

The CASP is a separate system. It communicates through HTTP and the rUSD contract and has no access to the issuer database.

## Quick start

With Docker Desktop running, execute from this repository:

```powershell
docker compose up --build --detach --wait
```

Available endpoints:

- issuer panel: `http://127.0.0.1:5173`;
- information document: `http://127.0.0.1:5173/white-paper`;
- issuer API: `http://127.0.0.1:3000`;
- mockBank: `http://127.0.0.1:3100`;
- Ethereum JSON-RPC: `http://127.0.0.1:8545`.

Stop the deployment or remove all demo data with:

```powershell
docker compose down
docker compose down --volumes
```

The Compose file contains public deterministic Hardhat accounts only. Never use these keys on a public network or fund them with real assets.

## Development and verification

Component-specific commands are documented below. The basic verification set consists of Rust tests, Hardhat contract tests, and frontend tests plus a production build. GitHub Actions are intentionally omitted because the repository is prepared as a thesis attachment.

## Further documentation

- [issuer documentation index](docs/en/README.md);
- [contract model and lifecycle](docs/en/token-contract.md);
- [contract and blockchain scripts](docs/en/asset.md);
- [backend, mockBank and API](docs/en/backend.md);
- [issuer frontend](docs/en/frontend.md).

