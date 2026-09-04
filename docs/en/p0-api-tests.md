# Issuer P0 API scenarios

`scripts/run-p0-api-tests.py` executes representative EM-01--EM-05 flows
against the running issuer and mockBank containers. It verifies issuance
idempotency, fiat-before-mint ordering, concurrent settlement, the published
operation-gate state, and a 1:1 redemption with a concurrent retry.

Run it after starting the issuer Compose project:

```powershell
python .\scripts\run-p0-api-tests.py
```

This is a mutating local-demo test. It uses unique operation identifiers,
issues a small amount to a deterministic Hardhat account, records fiat entries,
and burns part of the issued balance. Results are written to
`test-results/api-p0-issuer-*.json`. The complete EM-04 state matrix remains in
the isolated Rust integration tests so a shared demo is not deliberately left
in a blocked state.
