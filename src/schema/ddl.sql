-- Prim — Central data model (canonical DDL)
-- Pattern: Activity Schema (one append-only fact table; "money" is a filtered view).
-- OLTP engine: Turso (libSQL / SQLite dialect) — current/recent transactional state.
-- OLAP: history lives as Parquet in the IDS (R2), partitioned by date, clustered by
--        `activity`, queried with DuckDB. Analytics never hit the OLTP store.
-- See FABRIC.md (Central Model: Events) for the architectural rationale.

-- ---------------------------------------------------------------------------
-- Dimensions (small, not facts)
-- ---------------------------------------------------------------------------

-- Who/what a fact is about: worker, supplier, client, project, team.
CREATE TABLE IF NOT EXISTS entities (
    entity_id   TEXT PRIMARY KEY,            -- stable id (e.g. worker_01)
    kind        TEXT NOT NULL,               -- worker | supplier | client | project | team | entity
    name        TEXT NOT NULL,
    feature_json TEXT,                        -- JSON: contract, fund, bank, tax id, ...
    created_ts  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- Controlled vocabulary: allowed activity types + financial-kind taxonomy.
-- No free strings in events.activity / events.feature_json 'kind'.
CREATE TABLE IF NOT EXISTS catalog (
    code        TEXT PRIMARY KEY,            -- e.g. salary, expense, meeting_scheduled
    class       TEXT NOT NULL,               -- financial | productivity | compliance | continuity | ...
    is_financial INTEGER NOT NULL DEFAULT 0, -- 1 if this activity carries money
    label       TEXT NOT NULL
);

-- ---------------------------------------------------------------------------
-- Central fact table: events (Activity Schema)
-- One row per fact: `entity` did `activity` at `ts`.
-- Append-only. Corrections = new rows (void + re-entry), never UPDATE/DELETE.
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS events (
    event_id    TEXT PRIMARY KEY,            -- UUID, idempotency key
    ts          TEXT NOT NULL,               -- ISO8601 UTC
    entity      TEXT NOT NULL REFERENCES entities(entity_id),
    activity    TEXT NOT NULL REFERENCES catalog(code),
    amount      REAL,                        -- signed; NULL for non-financial activities
    currency    TEXT,                        -- ISO 4217 (COP default); NULL if non-financial
    feature_json TEXT,                        -- JSON: method, counterparty, category, status, ...
    ref_doc     TEXT,                        -- receipt/invoice/PILA/DIAN/postmortem -> IDS blob
    project_id  TEXT NOT NULL,               -- which instance/project
    source      TEXT NOT NULL,               -- cli | ai_tool | automation | import
    void_of     TEXT REFERENCES events(event_id) -- set when this row voids a prior one
);

CREATE INDEX IF NOT EXISTS idx_events_activity_ts ON events(activity, ts);
CREATE INDEX IF NOT EXISTS idx_events_project_ts  ON events(project_id, ts);
CREATE INDEX IF NOT EXISTS idx_events_entity_ts   ON events(entity, ts);

-- ---------------------------------------------------------------------------
-- money: NOT a table. A filtered view over events (financial activities only).
-- Clustering by `activity` makes this scan only the financial clusters.
-- ---------------------------------------------------------------------------
CREATE VIEW IF NOT EXISTS money AS
SELECT
    event_id, ts, entity, activity AS kind, amount, currency,
    feature_json, ref_doc, project_id, source, void_of
FROM events
WHERE activity IN (
    'income', 'expense', 'salary', 'dividend',
    'tax', 'loan', 'transfer', 'investment'
);

-- OLAP note (DuckDB over Parquet in R2 — not created here, documented for reference):
--   SELECT * FROM read_parquet('s3://<instance>-ids/olap/events/date=*/**.parquet')
--   P&L / cash flow / runway / cost-per-month = GROUP BY kind, date over the view.
