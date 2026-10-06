# Prim — TODO

Generalist operating framework. Manages personal life, projects, and companies.
Automates the tedious: accounting, emails, files, repetitive tasks, tax filings.

---

## Scope

Prim serves for:
- **Personal life** — emails, files, reminders, tedious tasks
- **Own projects** — management once they generate revenue
- **Companies** — reusable operating model (any instance)

It is not only financial. It is the operating system of your productive life.

---

## Completed

- [x] `docs/fiscal/guia-tributaria-sas.md` — Colombia tax rules for any SAS
- [x] `docs/ops/principios-operativos.md` — Minimalism, technological governance, ROE, debt, quality
- [x] `docs/governance/votaciones-documentos.md` — Voting and legal documents pattern (REVIEW PENDING)
- [x] Instance cleanup — references to prim instead of duplicated content
- [ ] Access model — REVISED to IdP + two planes (browser SSO + machine SSH-CA). Leans decided, specifics DECISIÓN PENDIENTE (see GOVERNANCE.md / stack-selection.md)
- [x] Transactional DB + queue decided — Turso + native CDC (see stack-selection.md)

---

## Architecture

### Fundamental separation

```
prim/                          (this repo)
├── Operating model             → Rules (docs: fiscal, ops, governance)
├── Channel integration         → Employee interface (Mattermost + agent)
└── config.yaml                 → Defines the instance (DB, storage, repos)

dev-setup/                        (separate repo, NOT prim)
└── Nix + dotfiles + technical access (direct DB, CLI, cloud infra)
```

The channel is the same for everyone. The dev environment is a separate concern of the developer.

### Employee interface — the channel (decided)

The primary employee interface is the **channel / front** (browser plane, SSO — **Mattermost lean**; Slack needs a paid plan for SSO),
not a web portal. Per GOVERNANCE.md: employees interact in the channel and with the agent; there is **no portal** as the source of truth.

| Capability | Detail | Backend-first |
|---|---|---|
| **Projects** | Separation per project, each with its resources | Yes |
| **Chat** | Per-project messaging with threads | Yes |
| **Video calls** | Integrated or linked from the calendar | Yes |
| **Calendar** | Summons, events, reminders, deadlines | Yes |
| **Tickets** | As simple as possible (title + status + assignee) | Yes |
| **Brief documentation** | Inline editing, no endless docs | Yes |
| **Agent** | Brings you up to date, answers questions, queries the project RAG | Yes |
| **Files** | Synced with R2 (persistent) or Git as appropriate | Yes |
| **Repos/Resources** | Links to repos, Cloudflare, infra — visible by role | Yes |
| **RBAC** | Everything filtered by roles per project | Yes |

> **Data-viewing/editing (non-technical users):** open question, not a portal. Candidate: static app on Cloudflare Pages + DuckDB-WASM + Worker/R2, opened from the channel. See stack-selection.md.

### Storage

```
Persistent (source of truth):
├── Project R2            → project documents and files
├── Collaborator R2       → agent's personal memory
├── Git                   → statutes, code, DaC
└── TDS (OLTP)            → transactional state

Temporary (not persisted):
└── User's local PC       → drafts, working files that need not persist
```

The channel/apps sync with R2/Git. The user may keep temporary local files they do not upload.

### config.yaml (draft)

```yaml
instance:
  name: <project_name>
  repo_code: <GOP>/<org>/<project_name>.git   # code host (SSO via IdP)
  db: <tds-dsn>                                # TDS/OLTP, engine in stack-selection.md
  storage: s3://...                            # R2 compatible (data core)
identity:
  idp: cloudflare-access   # single IdP (DECISIÓN PENDIENTE). Groups map to permissions.
  machine_ca: cf-access-infra   # short-lived SSH cert authority (DECISIÓN PENDIENTE)
  groups: [admins, developers, data-readers, data-writers]   # coarse-grained to start
front:
  type: mattermost   # browser plane entry, SSO (DECISIÓN PENDIENTE)
agent:
  type: oas   # Operational Assistant. Stack: docs/ops/stack-selection.md
  steering: <GOP>/<org>/agent.git
```

No `profile` or `environment` field. Prim defines the instance (data + identity + services), not the developer's environment. Per-project VPS is optional and not part of this config.

### Boundaries between projects

| Project | Responsibility |
|----------|----------------|
| prim/ | Operating model + instance config + channel/agent integration |
| dev-setup/ | Personal technical environment (Nix, dotfiles). NOT part of prim |
| agent/ | Steering for LLMs. Cloned by dev-setup |
| `<project_name>/` | Specific instance: statutes, investment policy, OLTP |

### Open questions

- [ ] R2 (Cloudflare, free egress) or S3 for storage?
- [ ] Data-viewing/editing layer for non-technical users (see stack-selection.md open decisions)
- [ ] Video call: Jitsi integration, native, or external link?
- [ ] Does the agent run in a backend or as a separate service?

---

## Agent architecture (OAS - Operational Assistant)

Local agent on each machine with per-user memory in R2 and a shared RAG.

### Knowledge layers

| Layer | What it contains | Where it lives | Access |
|------|-------------|------------|--------|
| Per-user memory | Individual history, context, preferences | Engram → R2 (by user_id) | User only |
| Shared RAG | Company docs + data + conversations | Centralized | Everyone equally |

### Shared RAG (sources)

```
┌─────────────────────────────────────────┐
│  Shared RAG                             │
│                                         │
│  - prim/ (operating model)           │
│  - <project_name>/ (statutes, rules)    │
│  - TDS/OLTP (financial data)            │
│  - Channel history (conversations)      │
└─────────────────────────────────────────┘
```

The channel history is part of the RAG. The agent needs to read everything said in the project channels to answer with full context.

### Response flow

```
Employee asks in the channel
        │
        ▼
┌───────────────────────────────────┐
│  Agent (OAS)                      │
│                                   │
│  1. Identify user                 │
│  2. Load individual memory (R2)   │
│  3. Search the shared RAG:        │
│     - Docs (prim + <project_name>) │
│     - DB (TDS/OLTP)               │
│     - Channel history (project)   │
│  4. Generate response             │
│  5. Save to the user's memory     │
└───────────────────────────────────┘
```

### Deployment

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Machine 1  │     │  Machine 2  │     │  Machine N  │
│  (VPS/PC)   │     │  (tablet)   │     │  (client)   │
│  OAS agent  │     │  OAS agent  │     │  OAS agent  │
│ (ZeroClaw/  │     │ (ZeroClaw/  │     │ (ZeroClaw/  │
│  nanobot)   │     │  nanobot)   │     │  nanobot)   │
└──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                   │                   │
       └───────────────────┼───────────────────┘
                           ▼
              ┌──────────────────────┐
              │  R2 (Cloudflare)     │
              │                      │
              │  /users/{id}/memory  │  ← Per-user memory
              │  /rag/               │  ← Indexed shared RAG
              └──────────────────────┘
```

### Principles

- No 24/7 centralized service (packages > services, except the channel which is the central interface)
- Each machine has a local agent (no extra latency)
- Per-user memory: isolated individual context
- Shared RAG: same knowledge base for everyone
- Channel history as a RAG source
- Cloudflare R2: free egress, S3-API compatible
- The channel replaces email (zero emails)

### Communication channel

The channel replaces email. It is the single interaction interface (people + agent).

Requirements:
- Channels/topics (separate company from personal, project A from B)
- Threads (do not mix conversations)
- Organized, searchable files (so they are not lost)
- Video and voice calls
- Calendar (events, reminders, deadlines)
- API for a bot (the agent lives there)
- Full history (RAG source)
- Self-hosted or own data (single source of truth)

Agent integration:
- Agent reads the full history of the project channels (RAG)
- Webhook triggers the agent when someone asks
- The agent answers in the same channel
- Hosting: the agent runs as a centralized SSO service (Worker/serverless preferred). A Cloudflare Worker can front it as a public proxy if the compute has no fixed public endpoint.

---

## Input Interfaces (headless core, multiple frontends)

**Payroll, purchases and expenses are core Prim domain**, not an optional add-on.
Per `principios-operativos.md` §1 ("no payroll or expenses not tied to revenue"),
§2 ("Immutable Traceability — every movement in auditable logs; Single Source of
Truth") and TODO Scope ("automates the tedious: accounting, tax filings... the
operating system of your productive life"), every money movement (worker pay,
purchases, expenses, PILA, DIAN) is a first-class object of the reusable operating
model. IPO.md reinforces this via the OUTPUT symptoms "Death by Running Out of
Cash" (burn rate/runway), "Regulatory/Legal Risk Ignored" (PILA/DIAN mapped to
`guia-tributaria-sas.md`) and "Equity risk". **Prim provides the rules (domain +
calculations + legal); the instance provides only the context** (who the worker
is, which accounts, which entities).

The domain is **headless** but not command-only: one domain/backend, multiple
interfaces. CLI and AI call **exactly the same domain services** — no logic
duplicated between them.

### Architecture principle

```
                      DOMAIN (legal/calc source of truth)
                               │
          ┌────────────────────┼────────────────────┐
         CLI (Typer)       AI (typed tools)     AUTOMATION
          │                    │                      │
          └────────────────────┼──────────────────────┘
                               │
                        DOMAIN SERVICES
                               │
          ┌────────────────────┼────────────────────┐
       Turso (OLTP          DIAN API             PILA operator
       + native CDC
       as audit log)
                               │
                           PAYMENTS (Nequi / banking)
```

The LLM is an **orchestrator**, never a direct actor. It never touches Turso,
credentials, API keys, DIAN certificates or banking services directly. Flow:
`User → LLM → tool selection → domain service → Turso/API → tool result → LLM → reply`.

### Interface 1 — Interactive CLI

- Built with **Typer/Rich** (thin layer; zero business logic — invokes domain services only).
- Interactive mode interprets intent, asks only for missing data, executes, shows result, writes audit.
- Explicit commands work headless (no LLM), e.g.:
  - `payroll workday add --date 2026-10-02`
  - `payroll payment create --worker worker_01 --amount 100000 --method nequi`
  - `payroll payroll calculate --month 2026-10`
  - `payroll payroll close --month 2026-10`

### Interface 2 — AI chat

Conversational agent using **tools / function calling** against the domain.
The AI MUST NOT: compute legal values directly, modify the DB directly, invent
percentages, run arbitrary SQL, or decide whether a financial operation is valid.
It only calls typed tools. Keeps enough context to avoid re-asking stored data
(worker, contract, salary, fund, entities, usual amount/method).

### Tool layer (typed domain tools)

`get_worker` · `get_contract` · `register_workday` · `register_absence` ·
`calculate_daily_payroll` · `calculate_monthly_payroll` · `get_payroll_summary` ·
`create_payment` · `get_payment_status` · `generate_payroll_receipt` ·
`close_payroll_period` · `generate_pila` · `validate_pila` · `pay_pila` ·
`get_pila_status` · `download_pila_receipt` · `generate_dian_payroll` ·
`submit_dian_payroll` · `get_dian_status` · `calculate_severance` ·
`create_severance_consignation` · `get_documents` · `archive_document` · `get_audit_log`

### Tool security tiers

| Tier | Examples | AI autonomy |
|------|----------|-------------|
| **READ** | queries, calculations, states, documents | auto-execute, no confirmation |
| **WRITE** | register workday, modify data, generate documents | backend validation required |
| **FINANCIAL** | pay worker, pay PILA, consign severance | **explicit confirmation + backend authorization** before real money moves |
| **EXTERNAL** | transmit DIAN, send PILA, query providers | explicit confirmation + backend authorization |

Confirmation is UX; **authorization is code**. FINANCIAL/EXTERNAL tools sit behind
the backend's authorization layer (IdP groups → permissions), never behind prompt
trust. Smart confirmation: unambiguous READ/low-risk WRITE may auto-run (e.g.
"trabajó ayer" when there is a single worker and the date is unambiguous); any
money movement shows period/worker/amount/operator/plan and waits for "sí".

### No-AI mode (hard requirement)

The whole system runs without an LLM. If the AI provider fails: payroll,
calculations, external APIs and the DB keep working. The AI is an **additional
interface, not a domain dependency**.

### AI provider abstraction

- `AIProvider` interface; initial impl `OpenAIProvider`. The agent receives only
  available tools, schemas and needed context — **never secrets**.
- Prim-core axiom (`library/binary > service`) is preserved by exposing the core
  domain's typed tools as **MCP tools** the shared OAS (ZeroClaw/nanobot
  candidates) consumes. The domain lives in Prim; the OAS is just one more frontend.

### Turso adaptations (vs the spec's PostgreSQL assumption)

- OLTP/TDS is **Turso (libSQL/SQLite)**, not PostgreSQL — "arbitrary SQL from the
  LLM" stays forbidden; writes go through typed domain services only.
- **Audit log = Turso native CDC.** `get_audit_log` reads the CDC table (every
  insert/update/delete recorded, queryable like any other table) — no extra
  queue/broker, consistent with the decided data stack.

### Open decisions (interfaces)

- [ ] AI provider deployment: shared OAS consuming the core domain's MCP tools vs a dedicated `OpenAIProvider` process (both call the same Prim domain services either way).
- [ ] CLI distribution: shared `prim` CLI with subcommands (`prim payroll ...`, `prim expense ...`) vs separate binaries.
- [ ] Backend authorization binding for FINANCIAL/EXTERNAL tools: map to which IdP groups.

---

## Desired capabilities (no order or priority yet)

- Automate income-tax filing and accounting
- Auto-categorize expenses/income
- Answer tedious emails or generate drafts
- Avoid losing files (organization, search)
- Repetitive task management (minimalist, no heavy apps)
- Operating model for companies (reusable)
- Knowledge base so an assistant (OAS) helps employees
- Personal management: reminders, documents, paperwork

---

## Pending decisions

- [ ] What prim does vs what agent/ does (technical steering)
- [ ] What prim does vs what dev-setup/ does (infra/environment)
- [ ] What lives in prim vs what lives in an instance (company, personal life)
- [ ] Final form: Python package, knowledge base, both, something else
- [ ] Review the voting pattern (prim/docs/governance/)
- [ ] Instance OLTP: stays as is (specific to asset protection)
- [ ] Input interfaces: AI provider deployment (shared OAS via MCP tools vs dedicated provider) — payroll/expenses are core domain, not an instance (see Input Interfaces section)
