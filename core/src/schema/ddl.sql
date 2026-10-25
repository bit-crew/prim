-- Prim — Central data model (canonical DDL)
-- Pattern: single append-only Activity table with double-entry accounting.
-- OLTP engine: Turso (libSQL / SQLite dialect) — current/recent state.
-- OLAP: history as Parquet in the IDS (R2), partitioned by date, clustered by
--        `kind`, queried with DuckDB. Analytics never hit the OLTP store.
-- See FABRIC.md (Central Model: Events) for the rationale.
--
-- KEY DECISIONS:
--  * ONE table (events). Financial + operational facts together. No money view.
--  * DOUBLE-ENTRY: a money transaction = 2+ rows sharing tx_id whose amounts
--    SUM TO ZERO per unit (debit +, credit -). Operational facts = single row,
--    account NULL, not subject to the zero-sum rule.
--  * MONEY = INTEGER minor units (centavos). NEVER float (float breaks zero-sum).
--  * UNITS standardized: COP/USD (money), sec (the one time unit), count.
--  * Corrections = reversing entries (new tx_id, sign-flipped), never UPDATE/DELETE.

-- ---------------------------------------------------------------------------
-- Dimensions
-- ---------------------------------------------------------------------------

-- Who/what a fact is about: worker, supplier, client, project, team.
CREATE TABLE IF NOT EXISTS entities (
    entity_id    TEXT PRIMARY KEY,           -- stable id (e.g. worker_01)
    kind         TEXT NOT NULL,              -- worker | supplier | client | project | team
    name         TEXT NOT NULL,
    feature_json TEXT,                       -- JSON: contract, fund, bank, tax id, ...
    created_ts   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- Controlled vocabulary of activity/kind codes (no free strings in events.kind).
CREATE TABLE IF NOT EXISTS catalog (
    code         TEXT PRIMARY KEY,           -- e.g. salary, expense, meeting, ticket
    class        TEXT NOT NULL,              -- financial | productivity | compliance | ...
    is_financial INTEGER NOT NULL DEFAULT 0, -- 1 if rows of this kind carry money + account
    label        TEXT NOT NULL
);

-- Chart of accounts (only used by financial postings).
CREATE TABLE IF NOT EXISTS accounts (
    code  TEXT PRIMARY KEY,                  -- e.g. 1010_cash, 5010_payroll_expense
    name  TEXT NOT NULL,
    type  TEXT NOT NULL CHECK (type IN ('asset','liability','equity','income','expense'))
);

-- ---------------------------------------------------------------------------
-- Central fact table: events
-- Operational fact  = single row (account NULL, unit sec/count).
-- Financial tx      = 2+ rows sharing tx_id, amounts sum to zero per unit.
-- Append-only. Corrections = reversing entries (new tx_id). Never UPDATE/DELETE.
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS events (
    event_id    TEXT PRIMARY KEY,            -- UUID, idempotency key (one posting/row)
    tx_id       TEXT NOT NULL,               -- groups postings of one transaction
    ts          TEXT NOT NULL,               -- ISO8601 UTC
    entity      TEXT NOT NULL REFERENCES entities(entity_id),
    kind        TEXT NOT NULL REFERENCES catalog(code),  -- the verb/type
    account     TEXT REFERENCES accounts(code),          -- financial postings only; else NULL
    amount      INTEGER NOT NULL,            -- signed; money = centavos; sec; count
    unit        TEXT NOT NULL,               -- COP | USD | sec | count
    feature_json TEXT,                       -- per-kind variable attrs (participants, assignee, status, method, counterparty...)
    ref_doc     TEXT,                        -- receipt/invoice/PILA/DIAN -> IDS blob
    project_id  TEXT NOT NULL,
    source      TEXT NOT NULL,               -- cli | ai_tool | automation | import
    void_of     TEXT                         -- original tx_id this entry reverses (header-level intent)
);

CREATE INDEX IF NOT EXISTS idx_events_kind_ts    ON events(kind, ts);
CREATE INDEX IF NOT EXISTS idx_events_tx         ON events(tx_id);
CREATE INDEX IF NOT EXISTS idx_events_project_ts ON events(project_id, ts);
CREATE INDEX IF NOT EXISTS idx_events_entity_ts  ON events(entity, ts);
CREATE INDEX IF NOT EXISTS idx_events_account    ON events(account);

-- INTEGRITY (enforced at the Turso write boundary, per transaction):
--   For every financial tx_id, for each currency unit:  SUM(amount) = 0.
--   sec/count units are operational and never balance.
--   This is an application-level check at insert time (all postings inserted
--   together). Parquet in R2 has no constraints — integrity lives in Turso.

-- READING MONEY (no view, by decision): filter kind over the clustered column, e.g.
--   SELECT account, SUM(amount) FROM events
--   WHERE unit='COP' AND kind IN (SELECT code FROM catalog WHERE is_financial=1)
--   GROUP BY account;   -- balances / P&L from signed postings
-- OLAP (DuckDB over Parquet in R2):
--   SELECT * FROM read_parquet('s3://<instance>-ids/olap/events/date=*/**.parquet')
