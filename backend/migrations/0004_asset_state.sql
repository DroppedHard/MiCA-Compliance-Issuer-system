CREATE TABLE IF NOT EXISTS asset_state (
    singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
    state TEXT NOT NULL,
    reason TEXT NOT NULL,
    reserve_coverage_percent REAL,
    evidence_at_unix_ms INTEGER,
    policy_version TEXT NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL
);

INSERT OR IGNORE INTO asset_state VALUES (
    1, 'data_unavailable', 'Reserve coverage has not been observed yet', NULL, NULL,
    'reserve-coverage-v1', CAST(unixepoch('subsec') * 1000 AS INTEGER)
);
