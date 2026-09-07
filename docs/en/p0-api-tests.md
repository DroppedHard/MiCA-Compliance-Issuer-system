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

## Terminal EM-07 wind-down scenario

As the final check on a fresh, disposable deployment, run:

```powershell
python .\scripts\run-p0-api-tests.py --terminal-wind-down
```

The flag appends EM-07 after the regular suite. It proves that an ordinary
ERC-20 transfer works before wind-down, enters `wind_down` through the issuer
administration API, verifies idempotent replay, and then confirms that:

- a new issuance is rejected with `issuance_blocked` and the fiat deposit is
  refunded to the CASP;
- an ordinary transfer submitted directly through JSON-RPC is rejected by the
  contract;
- redemption can still burn tokens and settle the 1:1 payout.

This test irreversibly changes the token lifecycle. Run it last and recreate
the local demo environment, including its volumes, before another test series.
The JSON report marks this coverage as `live-api-terminal` and records whether
the terminal part was requested and executed.
