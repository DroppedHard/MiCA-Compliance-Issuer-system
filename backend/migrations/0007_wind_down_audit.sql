CREATE TABLE IF NOT EXISTS wind_down_audit (
    operation_id TEXT PRIMARY KEY,
    reason TEXT NOT NULL,
    transaction_hash TEXT,
    confirmed_at_unix_ms INTEGER NOT NULL
);
