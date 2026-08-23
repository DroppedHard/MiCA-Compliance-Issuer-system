# Hardhat + viem project

This project is the on-chain asset owned by the rUSD issuer boundary at `issuer/asset/`.

## Project layout

```
contracts/        Solidity source files (*.sol) and unit tests (*.t.sol)
test/             TypeScript integration tests and Solidity unit tests (*.sol)
ignition/         Hardhat Ignition deployment modules
scripts/          Standalone scripts run with `hardhat run`
hardhat.config.ts
```

## Working in this project

When a **`hardhat`** skill is available, use it for tests, `hardhat.config.ts` or TypeScript network interaction. If the current agent environment does not provide that skill, inspect the installed Hardhat version and use the official Hardhat 3 documentation linked below. Always run the real Solidity/TypeScript tests and type checker after changes.

## Docs

- Hardhat 3 — https://hardhat.org/llms.txt
- viem — https://viem.sh/llms.txt
