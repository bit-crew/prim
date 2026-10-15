# Agent Stack Selection

Roles are stable; the stack is swappable. Selection pending.

## Roles

| Code | Role | Function |
|------|------|----------|
| OAS | Operational Assistant | Conversational assistant over RAG. Answers in-channel, queries docs + DB + history, runs tasks via API. Not a coding agent. |
| CAG | Coding Agent | Writes and executes code. Covered by Kiro. |

## OAS Candidates

Criteria (Prim axioms): library/binary > service, Zero-Ops, minimalism, self-hosted, token-efficient.

| Candidate | Lang | Footprint | Notes |
|-----------|------|-----------|-------|
| ZeroClaw | Rust | 3.4 MB, <10 ms | Single binary, no deps, 22+ LLM providers. Max Zero-Ops. Basic memory. |
| nanobot | Python | pip, no Docker | ~4k readable lines, MCP, fits Python stack. |
| PicoClaw | Go | <10 MB, <1 s | Single binary, runs on tiny hardware. Pre-1.0. |

Preliminary: ZeroClaw for max Zero-Ops; nanobot if auditing/modifying in Python. Verify repos before committing.

## CAG Memory: Coding-Agent Session Memory (DECIDED: tether over Turso)

**This is the memory of the coding agent (CAG), not the OAS semantic brain.** They are two distinct layers — do not conflate them:

| Layer | What it stores | Who uses it | Where it lives |
|-------|----------------|-------------|----------------|
| **CAG memory** (this section) | Per-repo decisions, bugfixes, conventions, gotchas; cross-session continuity for the coding agent | A developer while coding (Kiro and any MCP agent) | **Turso** (via tether), see below |
| **OAS RAG brain** (`RAG/brain_index/` in FABRIC.md) | Vectorized context over business docs + DB + history | The OAS answering in-channel | **R2** (vector index over Parquet) — stays in R2 |

Criteria (Prim axioms): library > service, Zero-Ops, local-first, shared across a team, on the already-decided stack (Turso), token-efficient.

**Decision: [tether](https://github.com/sidyellur/tether)** — an MCP server over a local SQLite file that becomes an **embedded libSQL/Turso replica** when pointed at a hosted primary.

Why tether over the alternatives:

| Candidate | Lang | Centralization | Fit with Prim | Verdict |
|-----------|------|----------------|---------------|---------|
| **tether** | Python (uv) | Embedded replica on **Turso** — single-primary; writes push immediately, reads pull debounced | Reuses Turso (already decided); no new running service — the MCP process is ephemeral, launched by the agent over stdio; **Nivel 1/2** | **Chosen** |
| Engram + Git Sync | Go binary | Batch, manual (`engram sync` → chunks in a git repo → peer pulls) | Async + human-in-the-loop; uses the code GOP as a data bus (mixes domains); merge-conflict risk | Rejected: not near-real-time; domain mixing |
| Engram Cloud | Go binary | Self-hosted container (Go + **Postgres**) | A running service + Postgres (ajeno al stack) → **Nivel 3**, contradicts Zero-Ops | Rejected: new service + new DB engine |

**How it runs (no extra service):** tether is an MCP server over **stdio** — the agent launches it as a subprocess and it dies with the session. Local-only by default (`~/.local/share/tether/memory.db`, Nivel 1). For shared team memory, install the sync extra and set `TETHER_SYNC_URL` (`libsql://<db>.turso.io`) + `TETHER_SYNC_TOKEN`: the local file becomes a Turso embedded replica. The central piece is Turso (managed), never a process Prim operates.

**Shared-team model:** all developers use the same project name and the same Turso primary; `scope: project` memories (saved in the team lingua franca — FTS5 is monolingual) are visible to every teammate's agent near-real-time.

**Analytics (viable, YAGNI until needed):** DuckDB can query the Turso/SQLite store directly for memory analytics (which decisions/conventions, how often recalled) with no data movement. Only if large-scale historical analytics are ever needed, export periodically to Parquet in R2 (`olap/`) and query alongside the rest. R2 is **not** the primary store for CAG memory.

**Open / to validate (spike before committing in production):**
1. tether is **pre-1.0 (v0.6.1)** — less production mileage; validate the 3-writer shared flow against a real Turso primary.
2. Documented sync gap: writes made **while the backend is unreachable** land only in that device's local file and are not reconciled later ([tether#99](https://github.com/sidyellur/tether/issues/99)). Minor for stable connections; not zero.

## GOP: Git / Code Host (PENDING)

Decision pending. The GOP (Git Operations Platform) is the SoT for code, DaC and IaC. It is **code hosting only** — it is NOT the identity provider (person authentication is handled by the central IdP; code push uses the machine plane's short-lived SSH cert, see [Access Model](#access-model)). Choosing one affects the bootstrap runbook (see BOOTSTRAP.md) and the `onboard`/`offboard` flows for technical staff's code access.

Criteria (Prim axioms, revised): **not self-hosted** (rejected: cost + ops overhead), data-portable (plain git), scriptable 100% via API/Terraform, RBAC for code by group. The former "free OIDC provider" criterion no longer applies since the GOP is not the identity layer.

| Candidate | Hosting | API | Notes |
|-----------|---------|-----|-------|
| GitHub | SaaS | Full REST API + mature Terraform provider | Most mature automation (RBAC/repos via Terraform); max visibility. |
| GitLab.com | SaaS | Full REST API | Cleaner nested-group RBAC if many nested projects appear. |
| Codeberg | SaaS (Forgejo) | REST API | Most axiom-aligned; automation/API less mature than GitHub. |

Lean: **DECISIÓN PENDIENTE — open for now.** The GOP will be **one fixed, supported platform**, not free-per-project: the automation (bootstrap/IaC creating repos, applying group RBAC via Terraform, `onboard`/`offboard`) cannot support every git host at once. The choice is among the candidates above and stays open. (Where the Prim framework repo itself is parked is a trivial operational fact, not a stack decision — not documented here.) See [Access Model](#access-model) for why the identity role was removed from the GOP.

---

## Access Model

How people reach centralized company resources. Model revised (2026-09-26): **one identity (IdP) + two access planes**. Identity is the center; per-project VPS is optional.

### Principle: one identity, two planes

A single IdP holds people + groups. Every resource maps groups → permissions. People reach the core through two planes of different privilege:

- **Browser plane (low privilege, consume):** SSO login in the front → view data (DuckDB-WASM, spreadsheet), chat, calendar, video, tickets, project tracking. Interact in place, no raw download.
- **Machine plane (high privilege, produce):** the developer's machine authenticates with a **short-lived SSH certificate** signed by a **central CA** after the same SSO. Grants data-for-development, code push, and writing own data to R2. Centrally scoped; revoke = remove from the IdP group → cert stops issuing and expires in hours.

### DECISIÓN PENDIENTE (leans)

| Piece | Lean (start simple) | Alternatives |
|-------|--------------------|--------------|
| **IdP (identity)** | **Cloudflare Access** — CF already hosts R2/edge; managed, Zero-Ops | Authentik / Zitadel (self-hosted, more control, Level 3) |
| **Machine auth (CA)** | **Cloudflare Access for Infrastructure** — managed SSH-CA, same SSO | Smallstep / Teleport (self-hosted CA) |
| **Cert lifetime / 2FA** | 8–24h cert; YubiKey optional as a signing 2nd factor (adds security, not the identity) | Longer/shorter TTL; mandatory hardware factor |
| **Front (browser plane)** | **Discord** (decided; disposable adapter) | Self-hosted (Mattermost/Stoat) rejected: adds a running service for no gain since business data lives in R2/Turso/Git, not the chat |
| **Permissions granularity** | **Coarse-grained** (group → R2 prefix / DWH schema) | Fine-grained (table/column/row) when it hurts |

### Why a short-lived SSH CA (vs plain SSH keys)

- **Centralized + revocable:** the CA + IdP groups are the single source of truth. No `authorized_keys` copied per host; no server-by-server cleanup on offboarding.
- **Ephemeral:** certs expire on their own; a removed person loses access within hours.
- **Same identity as the browser:** one login for both planes.
- **Auditable:** each cert records who/when/which groups.

### Historical context: why identity was previously moved off-IdP (now superseded)

An earlier design (2026-09-17) removed email and all IdPs and pushed person auth to **YubiKey+SSH on each VPS**, with Cloudflare Tunnel as transport-only. Rationale then: avoid issuing email/IdP accounts and avoid self-hosting an IdP. That model is **superseded** by the centralized IdP + two-plane model above, because:

- The new model keeps identity central and revocable without per-VPS key management.
- It supports a browser plane (SSO) that YubiKey-per-VPS never covered.
- VPS becomes optional per project rather than the mandatory unit of access.

The prior rejections remain useful as context (they still shape the leans):

| Previously rejected | Note under the new model |
|--------|--------------|
| Self-hosted git as IdP (Forgejo) | Still avoid self-hosting the IdP to start; managed Cloudflare Access is the lean. |
| Social identity (Google/Microsoft) | Still not the primary identity; the IdP is Prim's own tenant. |
| Okta | Extra external dependency; a lighter managed IdP (CF Access) is preferred to start. |
| Email OTP | Email is no longer forbidden as an identity signal; the IdP decides the factor. |
| Service token only | Still weak — the IdP identifies a person, tokens identify credentials. |
| Device-bound enrollment | Still avoided — the machine plane uses per-session certs, portable across machines. |

YubiKey is **not discarded**: it can be required as a second factor to sign the SSH cert, adding hardware-backed protection without being the identity itself.

### Data & interface layers (corrected model)

The source of truth for data is **Cloudflare R2 (storage) + DWH/DB** — centralized, NOT a VPS. Layers:

```
SOURCE (data lives here):   Cloudflare R2 + DWH/DB (centralized)
        │  accessed & PROCESSED at:
Compute (optional VPS or local machine, ephemeral): uv, rust, DuckDB — processes, does not own
        │  VISUALIZED at:
Employee device (browser plane): renders on screen — never the source
```

Security paradigm (Prim-specific): **facilitate access, keep data at the source.** Do not protect by denying access; protect by "use and read in place, do not download/copy". Access is liberal; exfiltration is what is constrained.

### Interface needs by role (defined; stack NOT yet chosen)

**All employees — the channel (common interface):**
- Channel (**Discord**, decided): communication with coworkers, calendar, project docs, talk to agents, schedule/consult meetings, create/manage tickets. The primary shared interface — a thin, swappable adapter; business data lives in R2/Turso/Git, not the chat.

**Technical employees — machine plane (own device, optional VPS):**
- Full stack (uv, rust, DuckDB, etc.) on their own machine or an optional per-project VPS. Connect over **SSH using a short-lived certificate** (central CA after SSO) with their editor (Helix / VSCode / Cursor). VPS is optional per project — the centralized data/services are reachable directly. See Access Model above (machine plane).

**Everyone (technical + non-technical) — the hard part, still open:**
Needs that neither the channel nor SSH-editor cover:
1. View R2 data "like a spreadsheet" (tabular).
2. Run queries on tables.
3. See tables as-is, make charts.
4. Reachable from the channel: a technical user drops a report/dataset link in the channel → a non-technical user clicks → a tab opens to view/edit + chart.
5. On save → writes to **their own R2 folder**.
6. Browse their R2 folder as a **file tree**.

### Candidate approaches for the data-viewing/editing layer (OPEN — do not commit yet)

All serverless / scale-to-zero, aligned with Zero-New-Ops L1/L2. To be decided later.

| Piece | Candidate option(s) | Notes |
|-------|--------------------|-------|
| Compute for queries/charts (raw data) | **DuckDB-WASM** (runs in the browser tab) | Reads Parquet/CSV directly from R2 via HTTP; runs real SQL client-side; same DuckDB already in the stack. `duck-ui` (OSS) already does SQL editor + notebooks + charts, no backend. For **viewing/querying raw data**, not hand-editing. |
| Static app hosting | **Cloudflare Pages** | Serves the HTML/JS mini-app at the edge, no server. |
| Read from R2 | **Worker generating presigned URLs** | Worker validates per-user permission, returns temporary signed URL; the tab reads from it. |
| Write to R2 (save) | **Worker with R2 binding** | Runs only on request, scales to zero (the "lambda that scales and saves" intuition, Cloudflare-native). Writes to `<user>/...`. |
| File tree of user's R2 folder | **Worker `list` by prefix `<user>/`** | Returns JSON, tab renders as tree. |
| Data RBAC | **The Worker as the single data-access gate** | Backend-first; who can read/write which R2 prefix. |
| Spreadsheet component for **final reports** ("like Excel") — OPEN | **Univer** (Apache 2.0, most complete, heaviest) · **Jspreadsheet/jExcel** (light, GPL/Pro) · **Glide Data Grid** (MIT, high-perf grid) · **RevoGrid** (MIT, light) · **Handsontable** (commercial for business) · **o-spreadsheet** (Odoo, OSS) · Syncfusion/Apryse (commercial, S3 integration built-in) | Non-technical users edit **final reports** (small tables), NOT raw data. This removes the hardest piece (rewriting Parquet by hand). Trade-off: Univer = most Excel-like but heaviest; Jspreadsheet/RevoGrid/Glide = lighter, more minimalist (closer to Prim spirit). UNDECIDED. |

### Open decisions that define the scope

1. **Two-tier data model (clarified):** non-technical users edit **final reports** (small tables), NOT raw data. Raw data (Parquet in R2) is view/query/chart only via DuckDB-WASM; final reports are hand-editable via a spreadsheet component and saved to the user's R2 folder. This removes the hard "rewrite Parquet by hand" piece. Open border to define: what is hand-edited vs. what is regenerated from data (Parquet→report automatically by agent/DuckDB).
2. **Spreadsheet component:** Univer vs. Jspreadsheet vs. Glide Data Grid vs. RevoGrid vs. Handsontable vs. o-spreadsheet vs. commercial (Syncfusion/Apryse). Lean minimalist (lighter grids) vs. most Excel-like (Univer). UNDECIDED.
3. **Report file format:** UNDECIDED. Note the components render cells regardless of format; the format is a storage choice.
   - **CSV:** most open + lightest for small reports, but no types/format/formulas. (Correction: for small files CSV is *lighter* than XLSX, not heavier — earlier claim reversed.)
   - **JSON:** open, preserves basic types, more verbose than CSV.
   - **XLSX (OOXML):** open standard (ECMA-376 / ISO 29500, not Microsoft-locked) but complex/heavy in implementation; only justified if reports need formatting/formulas/multiple sheets.
   - **ODS:** truly-free OpenDocument alt to XLSX, less JS-component support.
   - Guidance: start CSV for plain tabular reports; escalate to XLSX only if formatting/formulas are actually needed (YAGNI). Trade-off triangle: open + light + rich-format → pick two.
4. **Data-in-place rigor:** *pragmatic* (data reaches the browser tab, no download button — product-level protection) OR *strict* (not a single byte to the client → process on compute, send only pixels/results). Pragmatic fits "view as spreadsheet + charts"; strict would rule out DuckDB-WASM. UNDECIDED.
5. **Data format for raw analytical data:** Parquet (columnar, optimal for DuckDB read/query). Avro reserved for the WAL/streaming per FABRIC.md, NOT for the view/edit layer.
6. **Channel / front (browser plane): DECIDED — Discord.** Zero-Ops (SaaS, nothing to self-host), mature interactive buttons→webhook (the floating-button→Worker pattern needs this), project structure (servers/channels/roles/subgroups, pin, markdown). Self-hosted options (Mattermost/Stoat/Matrix) rejected: self-hosting a chat only to "own conversations" is technical debt for no gain — business data already lives in R2/Turso/Git. The chat is a thin, swappable adapter.

### Data-exfiltration caveat (AI editors)

VSCode/Cursor with AI features (Copilot, Cursor AI) may send code context to the provider even though the code is processed on the machine/VPS. For sovereignty-critical data, restrict which AI editors are allowed or use local models only. Tracked as an open policy decision.


---

## Data Layers Summary

State of the data stack (2026-09-17). Some layers decided, two explicitly deferred.

### Decided

| Layer | Choice | Rationale |
|-------|--------|-----------|
| **Serialization — edges** (ingest, APIs, channel payloads, email→R2) | **JSON** (via `orjson`) | Readable, flexible, schema-less. For boundaries, not internal streaming. |
| **Serialization — WAL / internal streaming** | **Avro** | Binary, compact, schema + schema-evolution. Per FABRIC.md WAL. |
| **Batch / OLAP storage (IDS)** | **Parquet** on R2 | Columnar, optimal for DuckDB read/query. Egress-free on R2. |
| **Ad-hoc query engine** | **DuckDB** (incl. DuckDB-WASM in browser) | In-process, reads Parquet directly. Same tool client + server. |
| **Object storage** | **Cloudflare R2** | S3-compatible, free egress. |
| **Final reports** | **CSV / XLSX** (format still open, see above) | Human-edited small tables; raw data stays Parquet. |

**JSON vs Avro border (defined):** JSON at the edges (ingest/APIs/channel/email), Avro for the WAL and schema'd internal streaming. They are NOT interchangeable — each has its place. (Optional future: Arrow IPC for zero-copy inter-process data if high-volume queues ever appear.)

### Transactional DB + Queue (decided: Turso)

| Layer | Choice | Rationale |
|-------|--------|-----------|
| **Transactional DB (TDS / OLTP)** | **Turso (libSQL/SQLite rewrite)** | "Library > Service" literal — SQLite *is* a library, no running DB service (Zero-Ops). The Turso rewrite (2025) adds **concurrent writes** + **native CDC** + vector search, closing SQLite's gaps for this use case. |
| **Queue / Events (CDC)** | **Turso native CDC** as the event log | CDC records every insert/update/delete into a **local table queryable like any other**. "The log is the queue": a consumer reads new rows by cursor/offset — no broker, no separate queue service, no dual-write. More minimalist than Postgres + pgmq (which needs an extension). |

**Queue tiers under Turso:**
- **Tier 0 (minimum, default):** native CDC table + consumer cursor (offset). Zero new dependencies.
- **Tier 1 (only if ack/retries/pub-sub needed — YAGNI):** embedded SQLite libs, no broker — `honker` (SQLite extension: Postgres-style NOTIFY/LISTEN + durable task queue, no daemon/polling) or `litequeue` (Python, FIFO, JSON messages).

**Maturity caveat (honest):** Turso CDC + concurrent writes are recent (2025, part of the SQLite rewrite). Sound in concept but less production mileage than Postgres/pgmq (years). Appropriate for personal/small-company scale. **Fallback:** if the OLTP ever becomes high-criticality financial at high write concurrency, revisit Postgres + pgmq. Not expected at current scale.

### Table-format note (to revisit, not blocking)

CONCEPT.md lists **S3/GCS + Iceberg** for the IDS. Honest caveat: Iceberg (catalog + metadata) may be over-engineering at small/medium scale. Consideration for later: **flat partitioned Parquet on R2 + DuckDB** is more minimalist and sufficient until ACID/time-travel/concurrent writes are genuinely needed (YAGNI). Iceberg/Delta only when scale demands it. Not a blocker now.

---

## Diagrams-as-Code (DECIDED: Markmap + D2)

Two tools, each in its domain. Mermaid dropped.

| Tool | Domain | Output | Runtime | Notes |
|------|--------|--------|---------|-------|
| **Markmap** | Mindmaps / trees (e.g. IPO) | **HTML** (interactive: collapse/expand/zoom) | Node/JS — **CI only**, never installed locally | Source = `.md` (readable in the repo as-is). Published on Pages for interactivity. |
| **D2** | Architecture diagrams / flows / containers | **SVG** (static image) | **Go binary** (no Node) | Custom fonts + theme from `src/config.yaml`. |
| ~~Mermaid~~ | — | — | Node/JS | Dropped — D2 cleaner (ELK layout), avoids Node locally. |

### Rationale for the split

D2 does not produce interactive HTML (only SVG/PNG/PDF/PPTX). Markmap produces
an interactive HTML that is navigable (collapse/expand/zoom) and its source is
plain `.md` — readable in the repo without rendering. Each tool covers what the
other cannot:
- **Markmap** = trees/mindmaps → interactive HTML (IPO, planning).
- **D2** = boxes/arrows/containers → SVG (architecture, data flows).

### Constraints

- **No Node locally** — Markmap runs only in CI (GitHub Action / equivalent);
  developers never install it. D2 is a Go binary (local, no Node).
- Renderable as static image (SVG) to post in the channel (D2). Markmap HTML is
  for Pages / browser consumption, not the channel.

### Open decisions

1. **CI workflow:** Action that renders `*.md` → Markmap HTML + `*.d2` → D2 SVG
   and publishes to Pages. PENDING (depends on where the repo is hosted).
2. **Verify D2 is in nixpkgs** for declarative install in `dev-setup`. PENDING.
3. **Font asset:** add `CourierPrime-Regular.ttf` to `assets/fonts/` so D2 embeds
   it in the SVG (else fallback monospace). PENDING.
4. **Channel render integration:** who renders diagrams for the channel (agent vs
   Worker). Ties to the front decision. UNDECIDED.
