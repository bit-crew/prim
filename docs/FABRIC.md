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
├── olap/                       # TELEMETRY & HISTORICAL
│   ├── logs/                   # Wide Logs (Parquet) (L_YYYY-MM-DD.parquet)
│   └── dumps/                  # States (Parquet) (S_YYYY-MM-DD_TYPE.parquet)
├── docs-intake/                # INCOMING DOCUMENTS catalog (Parquet; see §5)
│   └── dt=YYYY-MM-DD/project=<name>/...parquet
├── docs-md/                    # Editable Markdown produced from intake (R2)
│   └── <project>/<id>.md
└── blob/                       # BINARY REPOSITORY (immutable originals / evidence)
    └── [ID]_[YYYY-MM-DD]_[NAME].[EXT]
```

> **RAG vectors live in Turso, not R2.** The OAS semantic brain (vector index
> over docs/DB/history) is stored with **Turso native vector search** — no
> separate vector DB, no extension: embeddings are a column next to the data and
> are queried with SQL distance functions, so semantic search composes with
> relational filters (project, role, date) in one query. This supersedes the
> earlier "FAISS/Index in R2" idea: now that Turso (already the TDS) has native
> vectors, consolidating the RAG there is simpler (library > service) and keeps a
> single query surface. R2 still holds the **source** documents (blob) and the
> **editable Markdown** + **intake catalog** (Parquet); Turso holds the vectors.
>
> **Two memories, do not conflate.** The RAG above is the **OAS semantic brain**.
> The **coding agent's (CAG) session memory** (per-repo decisions, bugfixes,
> conventions) is a separate layer in **Turso via tether**. See
> [stack-selection.md](./stack-selection.md#cag-memory-coding-agent-session-memory-decided-tether-over-turso).

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

## 5. Data Classification (single source of truth)

The data defines where it lives, not the reverse. Every datum is classified by:
mutability, whether it needs version history, who reads it, how it is accessed,
and scope (company-wide vs per-project). From that, its **owning store** follows.

| # | Data type | Mutable | Valid version | Readers | Access | Scope | Owning store |
|---|-----------|---------|---------------|---------|--------|-------|--------------|
| 1 | Strategic docs (vision, principles, standards, guidelines) | yes | last **approved** | human + OAS | read + RAG | company | **Git** (versions+approval) → RAG (last approved) |
| 2 | Project docs (specs, design, decisions) | yes | last approved | human + OAS + CAG | read + RAG | project | **Git** (per project) → RAG |
| 3 | Incoming documents (what a client sends: PDF, Excel, meeting minutes) | no (immutable once received) | all (evidence) | human + OAS | blob + RAG | project | **R2** original (evidence) + Markdown → RAG. Catalog in Parquet (§6) |
| 4 | Transactional state (events: tickets, tasks, money, project state) | yes | current | human + OAS + system | SQL | project | **Turso (TDS)** — the events table (§1b) |
| 5 | Logs / telemetry (wide events) | no (append-only) | full history | system + analysis | analytic query | company + project | **R2** (Parquet, OLAP) + DuckDB |
| 6 | Code / DaC / IaC | yes | last (+ git history) | human + CAG | diff/versions | project | **Git (GOP)** |
| 7 | CAG session memory | yes (cumulative) | current | CAG | query | project | **Turso** via tether |
| 8 | Skills / Prompts (agent config) | yes | last (+ history) | OAS + CAG | read | company | **Git** (prim-skills) |
| 9 | RAG vectors (OAS semantic brain) | yes (re-indexed) | derived from source | OAS (via MCP) | vector search | company + project | **Turso** native vector search |
| 10 | Secrets | yes | current | human + system | encrypted | company + project | **R2** (SOPS) / KeePass |

### Patterns that emerge

- **A — Living approvable document** (1, 2): mutable, versioned, truth = last
  approved. **Git is the source of truth**; the **RAG is a derived projection**
  that indexes only the last approved version. If Git and RAG differ, Git wins;
  the RAG is regenerated.
- **B — Immutable evidence** (3, 5): arrives, never modified, kept forever. **R2
  is the source of truth** (the original blob). RAG/DuckDB are read projections.
- **C — Live state** (4, 7): changes constantly, queried live. **Turso is the
  source of truth** (CDC provides the audit history).
- **D — Versioned text** (6, 8): **Git**.

### The golden rule

> **Each datum has ONE owning store (source of truth). Everything else is a
> derived, regenerable projection.**

The **RAG is never a source of truth** — it is a derived index over Git (strategic
/ project docs) or R2 (incoming documents). It can be rebuilt from the source at
any time. This is what keeps coherence: there are never two "official" copies of
the same datum.

### Multi-project scope

One company, many projects, each with its own state. Scope is a **partitioning**
axis, not a new system:
- **Company (global):** strategic docs, skills, aggregated logs — transversal.
- **Project:** its docs, its state (Turso events partitioned by project), its code
  (Git), its RAG slice (filtered by project).

The RAG is queried with a scope filter ("search company docs + project X docs").
One model, partitioned by project — not a separate stack per project.

## 6. Incoming Documents Pipeline

Employees drop files in the channel (Discord/Telegram). The system normalizes
every document into prim's own format and keeps a clean, queryable catalog — no
PDFs/binaries to maintain as the working copy.

### Flow

```text
File dropped in the channel (PDF, Excel, meeting minutes, ...)
   │
   ▼  if not Markdown → AnyDoc converts it to clean Markdown
   ▼  original is stored in R2 blob/ as immutable evidence (url_original)
   ▼  editable Markdown is written to R2 docs-md/<project>/<id>.md (url_markdown)
   ▼  a row is appended to the intake catalog (Parquet, docs-intake/)
   ▼  content is embedded (via API) and indexed in Turso (RAG)
   ▼  the file is removed from Discord/Telegram (the channel is transport, not storage)
```

- **AnyDoc** ([firecrawl/anydoc](https://github.com/firecrawl/anydoc), Rust):
  converts Word/PowerPoint/Excel/ODF/RTF/EPUB/CSV/PDF → clean GitHub-Flavored
  Markdown. The ingest/parsing step.
- **Editable Markdown is the source of truth of the content.** It lives in R2
  (`docs-md/`) and is edited in-browser via the data-viewing/editing layer
  (Cloudflare Worker + R2 binding; see stack-selection.md). R2 stores the file;
  the Worker reads/writes it with permission control — R2 is fully compatible
  with online Markdown editing through that Worker.
- The **original** (PDF, etc.) is kept in R2 `blob/` as **immutable evidence**;
  `url_original` never changes.

### Intake catalog (Parquet in R2, `docs-intake/`)

Partitioned by **date**, clustered by **project**. One row per document.

| Column | Type | Notes |
|--------|------|-------|
| `project` | string | partition/cluster |
| `fecha` | date | partition |
| `remitente` | string | who sent/uploaded it |
| `titulo` | string | document title |
| `contenido` | string | the Markdown text (see note) |
| `url_markdown` | string | editable `.md` in R2 (`docs-md/`) — content source of truth |
| `url_original` | string | immutable original in R2 `blob/` (evidence) |

**Content in the column — decision:** the Markdown text is stored directly in the
`contenido` column (simpler, self-contained, fine at prim's scale — a company's
documents, not millions of huge files). Partitioning by date+project keeps each
Parquet file small, so DuckDB only reads the needed partition. **Parquet is a
regenerable projection**: the editable `.md` in R2 is the content source of truth;
if the `.md` is edited, the Parquet row is re-written from it. If content ever
grows too large, switch `contenido` to url-only (easy, non-breaking).

**RAG indexing — decision:** the RAG indexes the **full content** (chunked), not
a summary. A separate `resumen` column was considered and dropped (YAGNI): with
full content indexed it has no RAG role, and no other consumer needs it yet. It
can be added later as a catalog preview if a need appears.
