CREATE TABLE IF NOT EXISTS issuance_orders (
    operation_id TEXT PRIMARY KEY,
    recipient_address TEXT NOT NULL,
    amount_usd_minor INTEGER NOT NULL CHECK(amount_usd_minor > 0),
    token_amount_raw INTEGER NOT NULL CHECK(token_amount_raw > 0),
    bank_idempotency_key TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL CHECK(status IN ('awaiting_fiat','minting','completed','failed')),
    transaction_hash TEXT,
    last_error TEXT,
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL
);
