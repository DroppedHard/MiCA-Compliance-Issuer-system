CREATE TABLE IF NOT EXISTS redemption_orders (
    operation_id TEXT PRIMARY KEY,
    holder_address TEXT NOT NULL,
    token_amount_raw INTEGER NOT NULL CHECK(token_amount_raw > 0),
    payout_usd_minor INTEGER NOT NULL CHECK(payout_usd_minor > 0),
    status TEXT NOT NULL CHECK(status IN ('created','burned','completed','failed')),
    burn_transaction_hash TEXT,
    payout_reference TEXT NOT NULL UNIQUE,
    last_error TEXT,
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL
);
