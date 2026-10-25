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

- [x] `docs/guia-tributaria-sas.md` — Colombia tax rules for any SAS
- [x] `docs/principios-operativos.md` — Minimalism, technological governance, ROE, debt, quality
- [x] `docs/votaciones-documentos.md` — Voting and legal documents pattern (REVIEW PENDING)
- [x] Instance cleanup — references to prim instead of duplicated content
- [ ] Access model — REVISED to IdP + two planes (browser SSO + machine SSH-CA). Leans decided, specifics DECISIÓN PENDIENTE (see GOVERNANCE.md / stack-selection.md)
- [x] Transactional DB + queue decided — Turso + native CDC (see stack-selection.md)

---

## Architecture

### Fundamental separation

```
prim/                          (this repo)
├── Operating model             → Rules (docs: fiscal, ops, governance)
├── Channel integration         → Employee interface (Discord + agent)
└── config.yaml                 → Defines the instance (DB, storage, repos)

dev-setup/                        (separate repo, NOT prim)
└── Nix + dotfiles + technical access (direct DB, CLI, cloud infra)
```

The channel is the same for everyone. The dev environment is a separate concern of the developer.

### Employee interface — the channel (decided)

The primary employee interface is the **channel / front** (browser plane — **Discord**, decided; see "Web interface" below),
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

### Central data model — Events as the universal fact table

Decided and canonical. Everything (money + non-money) is one append-only fact in a
single **`events`** table. **Double-entry accounting**: a money transaction = 2+
rows sharing `tx_id` that **sum to zero per unit** (debit +, credit −); operational
facts are single rows. **Money = integer centavos** (never float). **Units
standardized**: COP/USD, `sec` (one time unit), `count`. **No `money` view** —
financial facts are read by filtering `kind` (clustered). OLAP = Parquet in IDS
(DuckDB), OLTP = Turso; analytics never hit Turso.

- Schema (SoT): **[`src/schema/ddl.sql`](./src/schema/ddl.sql)** — `events`,
  `entities`, `catalog`, `accounts`. Validated (double-entry balances to zero).
- Rationale: **[FABRIC.md](./docs/FABRIC.md)** → "Central Model: Events".
- **Open (to fix before implementation):** seed `catalog` (activity/kind list) +
  `accounts` (chart of accounts); FX handling when a tx mixes currencies (plug a
  gain/loss account).




### config.yaml (the single entry point)

Real file: **[`src/config.yaml`](./src/config.yaml)** — what the installer fills
(interactive or unattended). Holds the user's prerequisites (instance, Turso +
Cloudflare keys, GOP repo), identity, front, agent, optional LLM, and the brand
`theme` (colors + typeface). Fixed supported platforms: Turso (db), Cloudflare
(storage); GOP/IdP/front are DECISIÓN PENDIENTE (see stack-selection.md).

```yaml
instance:
  name: <company>
  org: <org>
  repo_code: <org>/<name>.git     # host from gop.platform
db:        { provider: turso, api_key: "", url: "" }
storage:   { provider: cloudflare, api_key: "", bucket: "" }
gop:       { platform: "" }       # github | gitlab | codeberg (one, pending)
identity:  { idp: "", machine_ca: "", groups: [admins, developers, data-readers, data-writers] }
front:     { type: discord }     # chat/adapter (decided); disposable, swappable
agent:     { type: oas, steering: <org>/agent.git }
llm:       { provider: "", api_key: "" }   # optional; core runs without it
theme:
  font:   { family: "Courier Prime", regular: "assets/fonts/CourierPrime-Regular.ttf" }
  colors: { ink: "#2B2D29", raw: "#F3F0E7", accent: "#A0A29E" }   # each project overrides
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

## Web interface — floating button + general services

The browser-plane front is a **chat**, and the provider is a **thin, replaceable
adapter**. **DECIDED: Discord.** Rationale: Discord is Zero-Ops (SaaS, nothing to
self-host or operate), has **mature interactive buttons/components** that POST to
an external webhook (the floating-button→Worker pattern depends on this), and
provides project structure out of the box (servers → channels → roles →
subgroups like `a-developers`, pinned messages, markdown).

**Why not self-hosted (Mattermost/Stoat/Matrix):** the business data (events,
reports, docs) already lives in **R2 + Turso + Git remote — in the user's own
accounts.** The chat only carries conversations, not business data. Self-hosting a
chat server just to "own the conversations" is technical debt for no real gain
(it adds a running service against "library > service", and ops burden). Sovereignty
is already satisfied where it matters: the data core. The chat is disposable and
swappable — if Discord ever stops fitting, only the thin adapter is rewritten.

On top of the chat sits a **floating button** (persistent in the UI, not a
per-message button) that opens a menu of **general services**.

**Key principle: the services are general/cross-cutting, not per-project.** A
project is only the *context* from which a service is invoked; the service itself
is the same for everyone. This keeps the services reusable and the chat a thin
layer.

### The pattern (chat-agnostic core)

```
[Floating button in chat]  →  [Cloudflare Worker (serverless)]  →  [R2 / Turso]
       (thin adapter)              (the reusable service core)
                                        │ (only when CF can't cover it)
                                        ▼
                          [external provider: Jitsi video link]
```

**Self-host principle: the ONLY self-hosted piece is the database (Turso).** No
self-hosted chat, no self-hosted calendar server (CalDAV/RustiCal/Radicale all
ruled out). Calendar and tickets are **`events` + a view in Discord**, not a
separate service. R2 is managed storage (Cloudflare), not something operated.

- **Services run serverless on Cloudflare** (Workers + R2 + Turso). This is the
  reusable core, identical for any chat provider.
- **The button is a thin adapter** — it just fires the Worker. Rewritten per chat
  provider (~a few lines); the service behind it never changes. Swap chat → rewrite
  only the adapter, not the services.
- **External providers** only where Cloudflare does not fit and no self-host is
  needed: **video calls = Jitsi Meet link** (generate a room, paste the link in
  Discord — nothing to operate). Calendar is NOT external — it is `events`.

### Floating button menu (general services)

| Service | What it does | Runs on |
|---------|-------------|---------|
| **Calendar** | Schedule in-person/video meetings as **`events`** (`activity='meeting_scheduled'`, `feature_json={start,end,participants[],jitsi_link}`); availability & "who/when to invite" = query over future meeting events; **max-hours-per-profile** limit (per IPO focus-protection) = query. Viewed in Discord. **No CalDAV, no calendar server** — not synced to the phone's native calendar (lives in Discord). | Worker + Turso (events) |
| **Tickets** | Create tickets as **`events`** (`ticket_opened`/`ticket_assigned`/`ticket_closed`); assignee is a field in `feature_json`; a person's **task view** = query of open tickets where `assignee=them`. Viewed in Discord. | Worker + Turso (events) |
| **Agent (OAS)** | Ask the assistant (e.g. request vacation, check payroll, file leave) | OAS binary |
| **Project state (`+`)** | Optional button: project status + indicators (budget spent vs available, a traffic-light/indicator). Computed from `events` (financial `kind`) filtered by `project_id` | Worker + DuckDB over R2 |

**Everything is `events`.** Calendar, tickets, meetings, payments, workdays — one
append-only table, viewed through Discord. The only external piece is the Jitsi
link for video. The only self-hosted piece is Turso (the DB).

### No prefab platforms — services are rows + queries

Decided: do **not** adopt a prefab tickets/tasks/calendar platform (Vikunja,
Taskwarrior, CalDAV servers, etc.). Every such product brings its **own DB + own
server** → a second source of truth and a service to operate, both against the
axioms. Since storage is already ours (the `events` table), a platform has nothing
left to give but UI. The realistic (small) work:
- **Typed write functions** that append a well-shaped row (`createTicket`,
  `createTask`, `createReminder(at:T)`, `createMeeting(rrule?)`).
- **Query views** over `events` folding the log into current state (open tickets,
  today's tasks, upcoming meetings).
- Import exactly **one logic library**: a recurrence engine (**`rrule`**, Rust) for
  calendar repetition — store the rule once, expand occurrences at read time (do
  not materialize every occurrence as rows).
- **Reminders ("do X at time T")** = **Cloudflare Durable Object Alarms** (store
  pending reminders in the DB; one self-rescheduling alarm fires what is due). No
  always-on server; fits "self-host only the DB".
- **Video** = Discord/Telegram native (or Jitsi link). Nothing to build.

### In-chat rich content (opens visual editors)

Clicking content in a message opens a **visual viewer** (data stays in R2;
compute in Cloudflare — only pixels/interaction to the client):
- **Table data link** → visual table viewer with a **pencil** to enter edit mode +
  save (back to R2). (Spreadsheet component OPEN — see stack-selection.md.)
- **Markdown doc** → markdown viewer + edit + save (normally repo/Git docs, may be
  in R2).
- **Markdown pasted into chat** → rendered inline (native in the chat provider).
- **Important message** → pinnable (native).

### What is "connect tools" vs "own development" (honest split)

- **Low effort (connect existing):** chat with channels/subgroups (e.g.
  `a-developers`), pin, markdown render, bots, interactive buttons. Native to
  Discord.
- **Own development (the Prim value):** the services the buttons open — table/
  markdown editors over R2, project-state dashboard, calendar rules, task view.
  Mostly already listed as candidates (Pages + DuckDB-WASM + Worker + R2).
- The chat is ~20% (easy, replaceable); the serverless services are ~80% (the
  reusable core).

### Open decisions (web interface)

- [x] Chat provider — **Discord** (adapter target; disposable/swappable).
- [x] Calendar — **`events` + Discord view**, no CalDAV/calendar server (self-host only the DB). Not synced to native phone calendars.
- [x] Video — **Jitsi Meet link** posted in Discord (nothing to operate).
- [ ] Spreadsheet component + report format (upstream OPEN, see stack-selection.md).

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
- [ ] Review the voting pattern (prim/docs/)
- [ ] Instance OLTP: stays as is (specific to asset protection)
- [ ] Input interfaces: AI provider deployment (shared OAS via MCP tools vs dedicated provider) — payroll/expenses are core domain, not an instance (see Input Interfaces section)
