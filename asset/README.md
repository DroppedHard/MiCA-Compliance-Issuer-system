# ResearchUsdEMT

A local demonstration of the technical core of a USD-denominated electronic money token. The project replaces the sample `Counter` contract and is inspired by the public [Circle EURC contract architecture](https://github.com/circlefin/stablecoin-evm).

## Important limitation

`ResearchUsdEMT` is a research token only. It does not represent real US dollars, hold reserves, create a redemption right, or claim MiCA compliance. Off-chain operations such as receiving funds or paying out US dollars are represented in the demo only by calls to `mint` and `burn` made by an authorized operator.

The contract exposes a terminal `enterWindDown()` operation restricted by `WIND_DOWN_ROLE`. Wind-down is distinct from the emergency `pause`: it blocks mint and ordinary transfers but intentionally allows authorised burns required for redemption. There is no wind-down exit function. A fresh local deployment is required after changing this contract ABI.

## Implemented core

- ERC-20 token named `Research USD EMT` with symbol `rUSD`;
- six decimal places, following the EURC convention;
- controlled issuance and redemption-side burning;
- operation-correlated `mintForOperation`, which permanently rejects reuse of the same issuance identifier;
- separate administrator, minter, burner, pauser, and freezer roles;
- a global pause covering transfers, issuance, and burning;
- address freezing that prevents sending, receiving, minting to, and burning from an address;
- standard ERC-20 events plus `MintOperationExecuted`, `AddressFrozen` and `AddressUnfrozen`.

The legacy role-protected `mint` remains available to local simulation scripts. The issuer backend uses `mintForOperation` because its idempotency identifier crosses the bank, SQLite and blockchain boundary.

The EURC inspiration concerns the operational model: controlled supply, separated roles, the ability to stop token movement, and address blocking. This implementation is intentionally smaller: it has no proxy, contract upgrades, EIP-3009 authorizations, permits, minter allowances, or cross-chain integration.

## Local setup tutorial

### Prerequisites

- Node.js and npm;
- an available local port `8545` for the persistent Hardhat node.

Install dependencies once:

```powershell
npm install
```

Verify the isolated contract project:

```powershell
npm run build
npm test
npm run typecheck
```

Run the self-contained disposable demonstration:

```powershell
npm run demo
```

`npm run demo` starts an ephemeral Hardhat environment, executes the scenario, and exits. Use a persistent node when connecting the Rust backend.

### Persistent node for backend integration

In terminal 1, start the node and leave it running:

```powershell
npx hardhat node
```

In terminal 2, from this same directory, deploy to that node:

```powershell
npx hardhat ignition deploy ignition/modules/ResearchUsdEMT.ts --network localhost
```

Copy the deployed address printed under `ResearchUsdEMTModule#ResearchUsdEMT`. Export that value as `TOKEN_ADDRESS` when starting the backend. Stop the node with `Ctrl+C`; its blockchain state is intentionally disposable.

## Generate continuous transfer traffic

Keep the Hardhat node running and use the same deployed address in another terminal:

```powershell
cd issuer\asset
$env:TOKEN_ADDRESS="0x...paste-the-deployed-address..."
npm run traffic
```

The simulator prepares four local users with up to `1000 rUSD` each and then transfers `1-5 rUSD` between them every three seconds. Existing balances are checked before minting, so restarting the command does not blindly repeat the full initial issuance. Stop it with `Ctrl+C`.

Optional settings:

```powershell
$env:TRAFFIC_INTERVAL_MS="1000"     # one transfer per second
$env:TRAFFIC_MAX_TRANSFERS="20"     # stop automatically after 20 transfers
npm run traffic
```

Every transfer creates a new block on the default automining Hardhat node. The backend therefore observes changing block numbers even though ERC-20 transfers do not change `totalSupply`.

## Demonstrate minting and burning

To make the supply chart visibly rise and fall, run a finite mint/burn cycle against the deployed token:

```powershell
$env:TOKEN_ADDRESS="0x...paste-the-deployed-address..."
npm.cmd run supply-cycle
```

By default the script mints `500 rUSD`, waits 12 seconds (long enough for the backend's polling cycle), and burns `200 rUSD`. The final net supply change is therefore `+300 rUSD`. Override the values when needed:

```powershell
$env:SUPPLY_MINT_AMOUNT="750.5"
$env:SUPPLY_BURN_AMOUNT="250.25"
$env:SUPPLY_DELAY_MS="15000"
npm.cmd run supply-cycle
```

The burn amount cannot exceed the amount minted by the same invocation. Both operations emit ERC-20 `Transfer` events, so they also add two transactions to the current ESG activity count.

## Demonstration flow

The `scripts/emt-demo.ts` script performs the following operations locally:

1. deploy the contract;
2. issue 100 rUSD to a holder;
3. transfer 25 rUSD from the holder to a merchant;
4. burn 5 rUSD from the merchant balance as a simplified redemption step.

The Solidity tests verify contract rules and authorization. The TypeScript/viem tests verify the complete flow from the perspective of a blockchain client.
