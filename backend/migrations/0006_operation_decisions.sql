CREATE TABLE IF NOT EXISTS issuer_operation_decisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id TEXT NOT NULL,
    operation_kind TEXT NOT NULL CHECK(operation_kind IN ('issuance','redemption')),
    asset_state TEXT NOT NULL,
    outcome TEXT NOT NULL CHECK(outcome IN ('allowed','rejected')),
    reason TEXT NOT NULL,
    evidence_at_unix_ms INTEGER,
    policy_version TEXT NOT NULL,
    decided_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_issuer_operation_decisions_operation
    ON issuer_operation_decisions(operation_id, operation_kind, decided_at_unix_ms);
