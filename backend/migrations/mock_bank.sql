CREATE TABLE IF NOT EXISTS reserve_accounts (
    account_id TEXT PRIMARY KEY,
    currency TEXT NOT NULL,
    balance_minor INTEGER NOT NULL CHECK(balance_minor >= 0),
    version INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS reserve_transactions (
    idempotency_key TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    operation_type TEXT NOT NULL CHECK(operation_type IN ('deposit','withdrawal')),
    amount_minor INTEGER NOT NULL CHECK(amount_minor > 0),
    balance_after_minor INTEGER NOT NULL,
    reference TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    FOREIGN KEY(account_id) REFERENCES reserve_accounts(account_id)
);
CREATE TABLE IF NOT EXISTS reserve_initializations (
    initialization_id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL,
    previous_balance_minor INTEGER NOT NULL,
    target_balance_minor INTEGER NOT NULL,
    reference TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    FOREIGN KEY(account_id) REFERENCES reserve_accounts(account_id)
);
