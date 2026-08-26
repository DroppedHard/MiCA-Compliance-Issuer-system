CREATE TABLE IF NOT EXISTS issuance_coverage_decisions (
    decision_id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation_id TEXT NOT NULL,
    decision TEXT NOT NULL CHECK(decision IN ('accepted','rejected','data_unavailable')),
    reason TEXT NOT NULL,
    current_reserve_minor TEXT,
    pre_operation_reserve_minor TEXT,
    confirmed_incoming_minor TEXT NOT NULL,
    current_supply_raw TEXT,
    proposed_mint_raw TEXT NOT NULL,
    current_coverage_bps TEXT,
    projected_coverage_bps TEXT,
    evidence_block_number INTEGER,
    bank_as_of_unix_ms INTEGER,
    policy_version TEXT NOT NULL,
    evaluated_at_unix_ms INTEGER NOT NULL,
    FOREIGN KEY(operation_id) REFERENCES issuance_orders(operation_id)
);
CREATE INDEX IF NOT EXISTS idx_issuance_coverage_operation
    ON issuance_coverage_decisions(operation_id, decision_id DESC);
