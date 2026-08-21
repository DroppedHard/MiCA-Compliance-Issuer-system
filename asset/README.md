# ResearchEuroEMT

A local demonstration of the technical core of a euro-denominated electronic money token. The project replaces the sample `Counter` contract and is inspired by the public [Circle EURC contract architecture](https://github.com/circlefin/stablecoin-evm).

## Important limitation

`ResearchEuroEMT` is a research token only. It does not represent real euros, hold reserves, create a redemption right, or claim MiCA compliance. Off-chain operations such as receiving funds or paying out euros are represented in the demo only by calls to `mint` and `burn` made by an authorized operator.

## Implemented core

- ERC-20 token named `Research Euro EMT` with symbol `rEUR`;
- six decimal places, following the EURC convention;
- controlled issuance and redemption-side burning;
- separate administrator, minter, burner, pauser, and freezer roles;
- a global pause covering transfers, issuance, and burning;
- address freezing that prevents sending, receiving, minting to, and burning from an address;
- standard ERC-20 events plus `AddressFrozen` and `AddressUnfrozen`.

The EURC inspiration concerns the operational model: controlled supply, separated roles, the ability to stop token movement, and address blocking. This implementation is intentionally smaller: it has no proxy, contract upgrades, EIP-3009 authorizations, permits, minter allowances, or cross-chain integration.

## Run locally

```shell
npm install
npm test
npm run typecheck
npm run demo
```

Deploy with Hardhat Ignition:

```shell
npx hardhat ignition deploy ignition/modules/ResearchEuroEMT.ts
```

## Demonstration flow

The `scripts/emt-demo.ts` script performs the following operations locally:

1. deploy the contract;
2. issue 100 rEUR to a holder;
3. transfer 25 rEUR from the holder to a merchant;
4. burn 5 rEUR from the merchant balance as a simplified redemption step.

The Solidity tests verify contract rules and authorization. The TypeScript/viem tests verify the complete flow from the perspective of a blockchain client.
