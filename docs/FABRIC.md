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
| 3 | Incoming items (file or email: PDF, Excel, receipt, bank email, minutes) | no (original immutable once received) | all (evidence) | human + OAS | classified → routed | project | **Classified into A/B/C** (§6): A→R2 Markdown+RAG; B→`events`(Turso)/project Parquet; C→discarded. Original kept as evidence in R2 `blob/` |
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

Employees drop files (or forward emails) in the channel (Discord/Telegram). Not
everything that arrives is a "document to read" — the same inbound item can be a
document, a piece of structured data, or noise. So ingest starts with a
**classifier** that routes each item to one of three destinations.

### Step 0 — Classifier (what is this?)

Done by the **agent (OAS/ZeroClaw) with the LLM**, with a few hard rules for
obvious cases (e.g. an email from a known bank → transaction). The LLM is used
because routing needs context, not a fixed format. It decides the destination:

```text
Something arrives in the channel (file or email)
   │
   ▼  CLASSIFIER (agent/LLM + hard rules)
   ├── A. Document to understand  → text whose value is reading/querying it
   ├── B. Data to process         → structured facts (two sub-kinds, below)
   └── C. Noise / ephemeral        → discarded, or turned into a reminder/task
```

### The three destinations

**A. Document to understand** (contract, proposal, minutes, report):
→ if not Markdown, **AnyDoc** converts it → editable Markdown in R2 → intake
catalog (Parquet) → content embedded and indexed in **Turso (RAG)**. Original
kept as immutable evidence in R2 `blob/`.

**B. Data to process** — structured facts. **Two sub-kinds, different homes:**
  - **B1 — Company event:** a fact that belongs to the company ledger/operations
    (an invoice, a bank transaction, a payment). → extracted into the **`events`
    table (Turso, §1b)** as one or more rows (money facts as double-entry
    `tx_id`), then flows to the OLAP/DWH (Parquet) for accounting, reports, IPO
    metrics. The file (PDF/photo) is kept as evidence (blob, `url`).
  - **B2 — Project input data:** tabular data that feeds a specific project but is
    **not** a company event (a dataset, a quotation sheet, a listing). → stored as
    **project data** (Parquet in the project's IDS / DWH), consumed by that
    project's processes. Not an event; no ledger posting.

**C. Noise / ephemeral** (spam, trivial confirmations, chit-chat): discarded. If
it implies an action, it becomes a **reminder/task**, not a stored document. Most
inbound email falls here.

### Why both OCR and the classifier are needed

- **The classifier** is needed because the destination is **not** determined by
  file type: a `.xlsx` can be a document (A), a company event (B1), or project
  input (B2); an email can be anything including noise (C). Only the content +
  context decides, so an LLM-based classifier routes it.
- **OCR** is needed because **AnyDoc requires a real text layer** — it cannot read
  scanned PDFs, photos of receipts, or images. A company that takes cash payments
  will receive photographed/scanned receipts constantly. Those need OCR to turn
  pixels into text/fields before anything else. Digital documents with signatures
  have a text layer and skip OCR; scans/photos do not.

### Cases that become events

Some inbound items are not documents at all — their value is a few fields that
must land in the **`events` table**, then accounting/DWH (destination B1):

- A **bank-transaction email**: only amount, transaction number, date matter. The
  body is not an official document — extract the fields → one event → the rest is
  dropped (C). 
- A **cash-payment receipt** (photo): OCR → amount/date/vendor → event (B1) +
  photo as evidence.
- An **invoice** (PDF/Excel): extract line-items → double-entry event(s) → DWH;
  the file is evidence.

### Extraction engine (OCR + field extraction) — decision

Two needs overlap: reading pixels (OCR) and pulling structured fields. Decision by
**criticality**, not one tool for all:

- **Financial/accounting data (B1 — critical):** use a **modern multimodal LLM**
  (e.g. Gemini-class, cheap: <$0.50 / 1000 pages) for OCR+extraction, **guarded by
  the accounting invariant**: extracted money facts must satisfy the double-entry
  **zero-sum per `tx_id`** (§1b). A hallucinated amount breaks the invariant and is
  rejected/flagged for review — the invariant protects against LLM hallucination,
  which is the known failure mode of general LLMs on structured financial docs. A
  dedicated invoice-parsing API stays as the fallback if volume or error rate
  demands it.
- **Documents to understand (A — non-critical):** a multimodal LLM is more than
  enough; perfect extraction is not required (it feeds semantic search, not the
  ledger).

> Benchmark note (2026): on clean/medium documents the accuracy gap between modern
> multimodal LLMs and specialized OCR has largely closed, and LLMs are far cheaper;
> but general LLMs can hallucinate on structured financial fields (fabricated
> numbers, skipped rows). Hence: LLM + the zero-sum invariant for money, specialized
> API only as a fallback.

### Flow (destination A — document to understand)

```text
File classified as A
   │
   ▼  has text layer? no → OCR (LLM) to get text ; yes → use directly
   ▼  AnyDoc converts it to clean Markdown
   ▼  original stored in R2 blob/ as immutable evidence (url_original)
   ▼  editable Markdown written to R2 docs-md/<project>/<id>.md (url_markdown)
   ▼  a row appended to the intake catalog (Parquet, docs-intake/)
   ▼  content embedded (via API) and indexed in Turso (RAG)
   ▼  the file removed from Discord/Telegram (the channel is transport, not storage)
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
