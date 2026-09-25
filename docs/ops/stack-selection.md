# Agent Stack Selection

Roles are stable; the stack is swappable. Selection pending.

## Roles

| Code | Role | Function |
|------|------|----------|
| OAS | Operational Assistant | Conversational assistant over RAG. Answers in-channel, queries docs + DB + history, runs tasks via API. Not a coding agent. |
| CAG | Coding Agent | Writes and executes code. Covered by Kiro. |

## OAS Candidates

Criteria (Kestrel axioms): library/binary > service, Zero-Ops, minimalism, self-hosted, token-efficient.

| Candidate | Lang | Footprint | Notes |
|-----------|------|-----------|-------|
| ZeroClaw | Rust | 3.4 MB, <10 ms | Single binary, no deps, 22+ LLM providers. Max Zero-Ops. Basic memory. |
| nanobot | Python | pip, no Docker | ~4k readable lines, MCP, fits Python stack. |
| PicoClaw | Go | <10 MB, <1 s | Single binary, runs on tiny hardware. Pre-1.0. |

Preliminary: ZeroClaw for max Zero-Ops; nanobot if auditing/modifying in Python. Verify repos before committing.

## GOP: Git / Code Host (PENDING)

Decision pending. The GOP (Git Operations Platform) is the SoT for code, DaC and IaC. It is **code hosting only** — it is NOT the identity provider (person authentication is handled on the VPS via YubiKey/SSH, see [Access Model](#access-model)). Choosing one affects the bootstrap runbook (see BOOTSTRAP.md) and the `onboard`/`offboard` flows for technical staff's code access.

Criteria (Kestrel axioms, revised): **not self-hosted** (rejected: cost + ops overhead), data-portable (plain git), scriptable 100% via API/Terraform, RBAC for code by group. The former "free OIDC provider" criterion no longer applies since the GOP is not the identity layer.

| Candidate | Hosting | API | Notes |
|-----------|---------|-----|-------|
| GitHub | SaaS | Full REST API + mature Terraform provider | Max visibility/discovery; best tooling. Canonical for the public framework. |
| GitLab.com | SaaS | Full REST API | Cleaner nested-group RBAC if many nested projects appear. |
| Codeberg | SaaS (Forgejo) | REST API | Viable only as a **visibility mirror**, not canonical. |

Lean: **GitHub canonical + Codeberg mirror** for the public framework (kestrel/dev-setup/agent) — GitHub for reach, Codeberg mirror for axiom-alignment and as an escape hatch. Private instance repos: host not fixed, self-hosting rejected. See [Access Model](#access-model) for why the identity role was removed from the GOP.

---

## Access Model

How people reach internal company resources. Decided (2026-09-17).

### Principle: two independent layers, never conflated

- **Layer A — Transport:** Cloudflare Tunnel exposes each VPS with no public IP, no inbound ports. Cloudflare is **only the cable**; it does not authenticate people.
- **Layer B — Person authentication:** the VPS `sshd` authenticates the person directly with a **FIDO2 hardware-backed SSH key (YubiKey)**. No email, no IdP, no third party.

Internal resources (TDS/OLTP DB, private services, private R2) are reachable **only from the VPS** and do not authenticate people — being inside the VPS is the trust boundary.

### Rejected alternatives (chronological, all evaluated)

| Option | Why rejected |
|--------|--------------|
| Self-hosted git as IdP (Forgejo) | Self-hosting = ops overhead + cost. Hard "no self-hosting" constraint. |
| Codeberg as Cloudflare IdP | Forgejo OAuth-as-OIDC for third parties unproven; no native Cloudflare connector; and self-host still required for control. |
| Social identity (Google/Microsoft) | Rejected by owner — do not want employees using external personal accounts. |
| Okta | Extra external dependency; still email/username-centric. |
| Email OTP (Cloudflare Access) | Requires giving each person an email/alias. Owner does not want to issue email to employees. |
| Service token only | Identifies a credential, not a person; transferable, weak identity. |
| Device-bound enrollment | Not portable — breaks "use any machine / VSCode from another computer". |

Chain of elimination: with **email removed AND all IdPs removed**, Cloudflare Access has no way to identify a *person*. Conclusion: **do not use Cloudflare for person auth at all.** Move authentication to the VPS.

### Decided model

```
Transport:    Cloudflare Tunnel per VPS (no public IP, outbound-only)
Auth:         VPS sshd + FIDO2 SSH key (ed25519-sk) → YubiKey touch required
Portability:  identity = the person's YubiKey, not the device → any machine works
Editors:      VSCode / Cursor / JetBrains / Helix via Remote-SSH
              (ProxyCommand = cloudflared access ssh)
Single session: MaxSessions 1 + MaxStartups 1 in sshd_config
Internal res: localhost/private only, reachable solely from the VPS, no extra 2FA
```

### Why this satisfies the axioms

- **Minimalism / Library > Service:** no IdP service, no Access policies for people, no Okta. One binary (`cloudflared`) for transport + native OpenSSH FIDO2 for auth.
- **No self-hosting:** nothing self-hosted; Cloudflare + Hetzner + hardware key.
- **Portability:** the YubiKey is the identity; any machine + the key = access. Solves multi-device and remote editors.
- **Sovereignty / privacy:** no third party sees or brokers the person's identity; auth is local to the VPS.

### Trade-off accepted

Identity is "whoever holds the FIDO2 SSH key + the physical YubiKey," not a biometric/central identity of a person. The YubiKey (non-copyable, phishing-resistant, requires physical touch) mitigates credential theft. True per-person central identity would require an IdP or email — both explicitly rejected. This trade-off is accepted deliberately.

### Data & interface layers (corrected model)

The source of truth for data is **Cloudflare R2 (storage) + DB** — NOT the VPS. Layers:

```
SOURCE (data lives here):   Cloudflare R2 + DB
        │  accessed & PROCESSED at:
VPS (compute, ephemeral):   uv, rust, DuckDB, etc. — processes, does not store
        │  VISUALIZED at:
Employee device:            renders on screen — never the source
```

Security paradigm (Kestrel-specific): **facilitate access, keep data at the source.** Do not protect by denying access; protect by "use and read in place, do not download/copy". Access is liberal; exfiltration is what is constrained.

### Interface needs by role (defined; stack NOT yet chosen)

**All employees — the channel (common interface):**
- Channel (Slack / Mattermost — candidate, not decided): communication with coworkers, calendar, project docs, talk to agents, schedule/consult meetings, create/manage tickets. Centralized. This is the primary shared interface.

**Technical employees — VPS + own device:**
- VPS with full stack (uv, rust, DuckDB, etc.). Connect from their device over **SSH** with their editor (Helix / VSCode / Cursor). Solved, not rigid — plain SSH to the VPS. See Access Model above.

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
| Spreadsheet component for **final reports** ("like Excel") — OPEN | **Univer** (Apache 2.0, most complete, heaviest) · **Jspreadsheet/jExcel** (light, GPL/Pro) · **Glide Data Grid** (MIT, high-perf grid) · **RevoGrid** (MIT, light) · **Handsontable** (commercial for business) · **o-spreadsheet** (Odoo, OSS) · Syncfusion/Apryse (commercial, S3 integration built-in) | Non-technical users edit **final reports** (small tables), NOT raw data. This removes the hardest piece (rewriting Parquet by hand). Trade-off: Univer = most Excel-like but heaviest; Jspreadsheet/RevoGrid/Glide = lighter, more minimalist (closer to Kestrel spirit). UNDECIDED. |

### Open decisions that define the scope

1. **Two-tier data model (clarified):** non-technical users edit **final reports** (small tables), NOT raw data. Raw data (Parquet in R2) is view/query/chart only via DuckDB-WASM; final reports are hand-editable via a spreadsheet component and saved to the user's R2 folder. This removes the hard "rewrite Parquet by hand" piece. Open border to define: what is hand-edited vs. what is regenerated from data (Parquet→report automatically by agent/DuckDB).
2. **Spreadsheet component:** Univer vs. Jspreadsheet vs. Glide Data Grid vs. RevoGrid vs. Handsontable vs. o-spreadsheet vs. commercial (Syncfusion/Apryse). Lean minimalist (lighter grids) vs. most Excel-like (Univer). UNDECIDED.
3. **Report file format:** UNDECIDED. Note the components render cells regardless of format; the format is a storage choice.
   - **CSV:** most open + lightest for small reports, but no types/format/formulas. (Correction: for small files CSV is *lighter* than XLSX, not heavier — earlier claim reversed.)
   - **JSON:** open, preserves basic types, more verbose than CSV.
   - **XLSX (OOXML):** open standard (ECMA-376 / ISO 29500, not Microsoft-locked) but complex/heavy in implementation; only justified if reports need formatting/formulas/multiple sheets.
   - **ODS:** truly-free OpenDocument alt to XLSX, less JS-component support.
   - Guidance: start CSV for plain tabular reports; escalate to XLSX only if formatting/formulas are actually needed (YAGNI). Trade-off triangle: open + light + rich-format → pick two.
4. **Data-in-place rigor:** *pragmatic* (data reaches the browser tab, no download button — product-level protection) OR *strict* (not a single byte to the client → process on the VPS, send only pixels/results). Pragmatic fits "view as spreadsheet + charts"; strict would rule out DuckDB-WASM. UNDECIDED.
5. **Data format for raw analytical data:** Parquet (columnar, optimal for DuckDB read/query). Avro reserved for the WAL/streaming per FABRIC.md, NOT for the view/edit layer.
6. **Channel:** Slack vs. Mattermost (self-hosted data sovereignty) vs. other. UNDECIDED.

### Data-exfiltration caveat (AI editors)

VSCode/Cursor with AI features (Copilot, Cursor AI) may send code context to the provider even though the code is processed on the VPS. For sovereignty-critical data, restrict which AI editors are allowed or use local models only. Tracked as an open policy decision.


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
