CREATE TABLE IF NOT EXISTS observer_state (
    chain_id INTEGER NOT NULL,
    contract_address TEXT NOT NULL,
    last_processed_block INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    PRIMARY KEY (chain_id, contract_address)
);

CREATE TABLE IF NOT EXISTS token_daily_activity (
    date_utc TEXT NOT NULL,
    chain_id INTEGER NOT NULL,
    contract_address TEXT NOT NULL,
    transaction_count INTEGER NOT NULL,
    first_block INTEGER NOT NULL,
    last_block INTEGER NOT NULL,
    finalized_at_unix_ms INTEGER,
    PRIMARY KEY (date_utc, chain_id, contract_address)
);

CREATE TABLE IF NOT EXISTS esg_daily_estimates (
    date_utc TEXT NOT NULL,
    chain_id INTEGER NOT NULL,
    contract_address TEXT NOT NULL,
    transaction_count INTEGER NOT NULL,
    energy_milliwh INTEGER NOT NULL,
    energy_lower_milliwh INTEGER NOT NULL DEFAULT 0,
    energy_upper_milliwh INTEGER NOT NULL DEFAULT 0,
    emissions_milligram_co2e INTEGER NOT NULL,
    renewable_energy_milliwh INTEGER NOT NULL,
    nuclear_energy_milliwh INTEGER NOT NULL,
    fossil_energy_milliwh INTEGER NOT NULL,
    methodology_version TEXT NOT NULL,
    calculated_at_unix_ms INTEGER NOT NULL,
    data_origin TEXT NOT NULL DEFAULT 'observed',
    PRIMARY KEY (date_utc, chain_id, contract_address)
);
