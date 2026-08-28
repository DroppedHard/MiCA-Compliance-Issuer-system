CREATE TABLE IF NOT EXISTS casp_daily_transaction_reports (
    date_utc TEXT PRIMARY KEY,
    asset_symbol TEXT NOT NULL,
    currency_area TEXT NOT NULL,
    total_operation_count INTEGER NOT NULL,
    total_value_raw INTEGER NOT NULL,
    total_value_usd_minor INTEGER NOT NULL,
    means_of_exchange_count INTEGER NOT NULL,
    means_of_exchange_value_raw INTEGER NOT NULL,
    means_of_exchange_value_usd_minor INTEGER NOT NULL,
    means_of_exchange_value_eur_minor INTEGER NOT NULL,
    excluded_operation_count INTEGER NOT NULL,
    known_onchain_overlap_count INTEGER NOT NULL,
    known_onchain_overlap_value_raw INTEGER NOT NULL,
    classifications_json TEXT NOT NULL,
    methodology_version TEXT NOT NULL,
    conversion_methodology TEXT NOT NULL,
    imported_at_unix_ms INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS casp_report_imports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_date_utc TEXT NOT NULL,
    to_date_utc TEXT NOT NULL,
    imported_at_unix_ms INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS casp_quarterly_assessments (
    year INTEGER NOT NULL, quarter INTEGER NOT NULL, evidence_json TEXT NOT NULL,
    complete_source_range INTEGER NOT NULL, threshold_breached INTEGER NOT NULL,
    threshold_enforceable INTEGER NOT NULL, assessed_at_unix_ms INTEGER NOT NULL,
    PRIMARY KEY(year, quarter)
);
CREATE TABLE IF NOT EXISTS activity_issuance_gate (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1), blocked INTEGER NOT NULL,
    reason TEXT, evidence_json TEXT, updated_at_unix_ms INTEGER NOT NULL
);
INSERT OR IGNORE INTO activity_issuance_gate VALUES(1,0,NULL,NULL,0);
