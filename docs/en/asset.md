# Contract and blockchain scripts

Install and verify:

```powershell
npm.cmd install
npm.cmd run typecheck
npm.cmd test
npm.cmd run build
```

Start `npx.cmd hardhat node`, then deploy from another terminal:

```powershell
npx.cmd hardhat ignition deploy ignition/modules/ResearchUsdEMT.ts --network localhost
```

Pass the printed address to the backend as `TOKEN_ADDRESS`. Available demonstration commands include `npm.cmd run traffic`, `supply-cycle`, `compliance-threshold`, and `external-deposit -- alice 100`. Direct-mint scripts intentionally bypass the issuer gateway and are test-only.

