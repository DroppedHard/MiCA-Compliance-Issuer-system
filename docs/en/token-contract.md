# rUSD contract and lifecycle

`ResearchUsdEMT` is a six-decimal demonstration ERC-20 with controlled minting and burning and separated operator roles. It is inspired by the public Circle EURC architecture but does not reproduce a complete production system.

`mintForOperation` executes one issuance per operation identifier, `burn` reduces supply during redemption, `freeze` restricts a selected address, and `pause` stops every balance update. `blockIssuance` applies an activity-evidence restriction, while `setReserveState` mirrors the reversible reserve assessment. `enterWindDown` is irreversible: it stops minting and ordinary transfers while preserving authorized redemption burns.

The effective lifecycle is `Active`, `Warning`, `IssuanceBlocked`, or terminal `WindDown`. Activity and reserve restrictions are independent; recovery on one axis cannot clear the other.

The local Hardhat account 0 receives every role. Production keys and roles would require segregation and secure custody. The contract neither holds fiat nor confirms bank payments; those operations belong to the issuer backend.

