# Prim: Data Fabric & Persistence

Prim uses a **Bifurcated Persistence** model to ensure data sovereignty, high performance, and total traceability.

## 1. Functional Domains (SoT)

| Domain | Role | Implementation |
| :--- | :--- | :--- |
| **TDS** | Transactional Data Store | Current OLTP state and logical catalog. |
| **IDS** | Immutable Data Store | Append-only storage: OLAP, blobs, and RAG vectors. |
| **GOP** | Git Operations Platform | Versioned persistence for code, DaC, and IaC. |

## 1b. Central Model: Events (Activity Schema)

The core data model is a **single append-only `events` table** (Activity Schema
pattern): one row per fact. Financial + operational facts live together — no
separate ledger table, **no `money` view** (financial facts are read by filtering
`kind`, made efficient by clustering). This covers the whole IPO map (finance,
productivity, compliance, continuity) from one SoT.

**Decided design:**
- **One table.** Everything is a row (payment, workday, ticket, meeting, decision).
- **Double-entry accounting.** A money transaction is **2+ rows sharing a `tx_id`**
  whose amounts **sum to zero per `unit`** (one leg −, the other +). Operational
  facts are single rows with no account. This gives books that balance, with
  origin/destination, without a second table.
- **Money as integer minor units.** `amount` is **INTEGER in centavos** (never
  float — floats break the zero-sum invariant via rounding).
- **Standardized units.** `unit` generalizes "currency" to any measurable
  magnitude: `COP`/`USD` (money), `sec` (the single time unit), `count`. The
  zero-sum rule applies **per unit** and only to currency units; `sec`/`count`
  never balance.
- **Accounts.** Financial rows reference a small chart-of-accounts dimension
  (`accounts`: code, name, type ∈ asset/liability/equity/income/expense). Balances
  and P&L derive from signed postings (debit +, credit −; beancount convention).
- **Immutability.** Corrections = **reversing entries** (a new `tx_id` with
  sign-flipped postings), never UPDATE/DELETE; `void_of` points to the original.
  Turso CDC is the audit trail.
- **OLAP (history):** Parquet in the **IDS** (R2), **partitioned by date, clustered
  by `kind`**, queried with **DuckDB**. Reports/IPO metrics aggregate here — the
  **OLTP store is never hit**.
- **OLTP (current state):** **Turso (TDS)** holds recent state, stays light. The
  zero-sum invariant is enforced at the Turso write boundary (app-level per `tx_id`).

Dimensions: `entities` (who/what), `catalog` (allowed activity/kind vocabulary),
`accounts` (chart of accounts). Canonical schema:
**[`src/schema/ddl.sql`](../src/schema/ddl.sql)**.


## 2. IDS Structure (Blob Storage)

```text
[project-name]-ids/
├── auth/                       # ACCESS & ENCRYPTION
│   ├── identity.kdbx           # Master KeePass file (Human-managed)
│   └── secrets.enc.yaml        # SOPS-encrypted secrets (Agent-managed)
├── RAG/                        # SEMANTIC BRAIN (OAS only — NOT coding-agent memory)
│   └── brain_index/            # Vectorized context (FAISS/Index) over business docs+DB+history
├── olap/                       # TELEMETRY & HISTORICAL
│   ├── logs/                   # Wide Logs (Parquet) (L_YYYY-MM-DD.parquet)
│   └── dumps/                  # States (Parquet) (S_YYYY-MM-DD_TYPE.parquet)
└── blob/                       # BINARY REPOSITORY
    └── [ID]_[YYYY-MM-DD]_[NAME].[EXT]
```

> **Two memories, do not conflate.** `RAG/brain_index/` above is the **OAS semantic brain** — a vector index over business docs/DB/history, correctly stored in R2 (its natural, cheap home for vector artifacts over Parquet). The **coding agent's (CAG) session memory** (per-repo decisions, bugfixes, conventions) is a separate layer and does **NOT** live in R2: it lives in **Turso via tether**. See [stack-selection.md](./stack-selection.md#cag-memory-coding-agent-session-memory-decided-tether-over-turso).

## 3. Persistence Guarantees

The architecture ensures data integrity via:
`Process → Local WAL → Batch → Blob Storage`

- **At-Least-Once:** Idempotency keys (`process_id` + `timestamp`) + read-time deduplication.
- **Crash Recovery:** Read pending local WAL on restart; delete only after S3/GCS confirmation.
- **Checkpointing:** Save state every N events to define recovery points.

## 4. Observability: Wide Events (Canonical Logging)

Processes emit ONE structured log event per unit of work upon completion.

| Field | Type | Notes |
| :--- | :--- | :--- |
| `timestamp` | ISO8601 UTC (µs) | Required |
| `trace_id` | String (UUID v4) | Required |
| `span_id` | String | Required |
| `level` | `INFO\|WARN\|ERROR` | Required |
| `event` | String (snake_case) | e.g. `etl_pipeline_success` |
| `duration_ms` | Float | Required for closing events |
| `context.*` | Mixed | Metadata (user_id, version, etc.) |

**Aggregation Pipeline:**
`[Span emitted] → Local WAL (Avro/RocksDB) → [Aggregation] → Object Storage (Parquet)`
