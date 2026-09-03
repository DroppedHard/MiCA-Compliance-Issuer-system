CREATE TABLE IF NOT EXISTS address_restrictions (
    normalized_address TEXT PRIMARY KEY,
    address TEXT NOT NULL,
    reason TEXT NOT NULL,
    active INTEGER NOT NULL CHECK (active IN (0, 1)),
    transaction_hash TEXT,
    updated_at_unix_ms INTEGER NOT NULL
);
