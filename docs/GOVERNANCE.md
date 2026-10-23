# Prim: Governance & Operations

Minimalist operation centered on the channel as the terminal and Git as the source of truth.

## 1. Channel-Centric Demand Flow

The **Unified Demand Flow** follows the "The Thread is the Issue" principle:

1.  **Capture (Channel):** Convert messages into Issues via native Git-platform shortcuts.
2.  **Execution (CLI):** Development occurs in the terminal. Branch name: `feature/issue-ID`.
3.  **Sync:** Automated notifications post commit updates back to the original channel thread.
4.  **Review & Merge:** Approval and consolidation happen directly via channel buttons.

### Standardized Classification Labels
Use these labels for Agent-led classification:
- **Task Type:** `Type::Tech`, `Type::Operational`, `Type::Legal`, `Type::Financial`.
- **Approval:** `Status::Pending-LLM-Approval`, `Status::Pending-Unanimity`.
- **Asset:** `Asset::House-X`, `Expense::Capex`, `Expense::Opex`.

## 2. Access & Identity (IdP + two planes)

Security is an environmental condition, not a business logic function. Prim centralizes **one identity** and exposes it through **two planes of different privilege**. Identity is the center: a single IdP holds the people and their groups; every resource maps groups → permissions.

> **DECISIÓN PENDIENTE (see [stack-selection.md](./stack-selection.md#access-model)):** IdP = **Cloudflare Access** (lean) or self-hosted (Authentik/Zitadel). Machine auth = **short-lived SSH CA via Cloudflare Access for Infrastructure** (lean) or Smallstep/Teleport. Permissions granularity = **coarse-grained** to start. (Front = **Discord**, decided — see stack-selection.md.)

### Single source of truth: IdP + groups

```
IdP (identity + groups: data-readers, developers, project-x, admins)
   │
   ├── Browser plane (SSO web, low privilege — CONSUME)
   │     └── front (Discord), tickets, calendar, video, project tracking,
   │         DuckDB-WASM viewer, spreadsheet component
   │         → permission = "view / interact in place", never raw download
   │
   └── Machine plane (short-lived SSH cert, high privilege — PRODUCE)
         └── data for development, code push, write own data to R2
             → permission = "read/write/develop" per group
```

Onboarding = add the person to the correct IdP groups. Offboarding = remove them → the browser session dies immediately and the machine certificate stops being issued (expires within hours). No per-server key cleanup.

### Plane A — Browser (low privilege, consume)

The person logs in with user+password / SSO in the front and clicks through to services. Everything is **view/interact in place**; raw data is never downloaded.

- Front (Discord — decided): chat, calendar, video calls, tickets/tasks, project tracking, brief docs.
- Data viewing/editing: DuckDB-WASM (query/chart raw data in the tab) + a spreadsheet component (edit final reports), saved to the person's own R2 folder. Stack open — see [stack-selection.md](./stack-selection.md#access-model).
- Permission model: the front SSO + a Worker/data-gate enforce which groups can see/interact with what (coarse-grained to start).

### Plane B — Machine (high privilege, produce)

A developer's machine (PC/tablet/VPC) authenticates with a **short-lived SSH certificate** signed by a **central CA** after the same SSO login. This is what makes the key "centrally scoped and revocable".

```
Developer machine (PC / tablet / VPC)
   │  1. SSO login to the IdP (same identity as the browser plane)
   ▼
Central SSH CA  ── verifies identity + groups → signs an EPHEMERAL certificate (e.g. 8–24h)
   │  2. machine presents the short-lived cert
   ▼
Target (R2 access, code repo, data for development)
   │
   └── 3. cert expires on its own. Revoke = remove from the IdP group → no more signing.
```

Why a CA instead of plain SSH keys:
- **Centralized:** the CA + IdP groups are the single source of truth. No `authorized_keys` copied per host.
- **Instant revocation:** remove from the group → the person can no longer get a cert; existing cert expires in hours. No server-by-server cleanup.
- **Same identity as the browser:** one login for both planes.
- **Auditable:** every issued cert records who, when, which groups.
- **Optional 2nd factor:** a YubiKey can be required to sign the cert — it *adds* security but is no longer the identity (the IdP is). Can be added later without redesign.

Grants on this plane: access data for development, push code to the repository, and write the employee's own data to R2 (`<user>/...`). Scope follows the person's groups.

> **DECISIÓN PENDIENTE:** cert lifetime, whether YubiKey is required as a 2nd factor, and the exact CA implementation (CF Access for Infrastructure vs Smallstep/Teleport).

### Editors and remote work

Editors (Helix / VSCode / Cursor / JetBrains) connect over SSH using the short-lived cert. If a per-project VPS exists, they use Remote-SSH to it; otherwise the machine works locally against the centralized data/services. The machine is not tied to a device enrollment — any machine that can complete the SSO + get a cert works.

### Onboarding (Agent-Executable)

```bash
# === ADD <user> ===
# 1. Create identity in the IdP and add to groups (source of truth)
#    e.g. groups: developers, project-x, data-readers
#    (IdP-specific API — DECISIÓN PENDIENTE which IdP)

# 2. Front (Discord) access is granted via SSO/SCIM from the same groups.

# 3. Machine plane: nothing to copy. On first use the person does SSO and the
#    CA issues a short-lived SSH cert scoped to their groups.

# 4. Data permissions: the groups already map to R2 prefixes / DWH roles
#    (coarse-grained). No per-user provisioning step.

# === RESULT ===
# <user> logs into the front (browser plane) AND can obtain a machine cert
# (machine plane), both from the same identity.
```

### Offboarding (Agent-Executable)

```bash
# === REMOVE <user> ===
# 1. Remove from all IdP groups (single action).
#    → Browser SSO session invalidated (immediate).
#    → CA stops signing certs for them; any live cert expires within hours.
#    → R2/DWH permissions (mapped from groups) revoked.
# 2. Optionally archive their R2 folder for audit.

# === RESULT ===
# One action revokes BOTH planes. No server-by-server key removal.
```

### Data & interface layers

The source of truth for data is **R2 (storage) + DWH/DB** — centralized, not a VPS. Layers:

```
SOURCE (data lives here):   R2 + DWH/DB (centralized)
        │  accessed & PROCESSED at:
Compute (optional VPS or local machine, ephemeral): uv, rust, DuckDB — processes, does not own
        │  VISUALIZED at:
Browser plane:              renders on screen — never the source
```

Security paradigm (Prim-specific): **facilitate access, keep data at the source.** Do not protect by denying access; protect by "use and read in place, do not download/copy". Access is liberal; exfiltration is what is constrained.

### RBAC

Access is assigned by **group/role and project** in the IdP, then mapped to native permissions on each resource: R2 policies by bucket/prefix, DWH roles (`GRANT` by schema/table), and service scopes. Start **coarse-grained** (a few groups → prefix/schema level); go fine-grained only when it hurts (YAGNI).

### Audit

The SSO identity (browser) and the SSH certificate identity (machine) are injected into every event trace in the IDS for forensic analysis.

### Management

IdP groups, R2 policies, DWH roles and (optional) VPS provisioning are 100% API/Terraform-driven — zero dashboard dependency, agent-automatable.

## 3. Complex Event Processing (CEP) Alerts

Eliminate polling and BI dashboards through push-based alerts in the EPE (functional iterators):

| Alert Type | Logic (CEP) | Example |
| :--- | :--- | :--- |
| **Accumulation** | Metric exceeds threshold in time window. | Counter of high-value transactions per account > Limit. |
| **Correlation** | Event A occurred but expected Event B did not follow. | "Due Date Expired" without "Payment Created" within 1h. |
| **Deviation** | Significant difference vs historical average. | Ratio of issues closed without valid ID rises significantly. |
| **Threshold** | Event changes state past a defined limit. | Account balance falls below threshold after transaction. |

## 4. Documentation Strategy (DaC)

- **Non-Technical:** Statutes and agreements in a central repo, published as GOP Pages.
- **Technical:** Documentation lives **next to the code it describes** (`Docs-in-Repo`,
  co-location / sidecar pattern — the industry standard for internal dev docs).
- **Visual (two tools, each in its domain):**
  - **Markmap** → mindmaps / trees (e.g. IPO). Renders to **HTML** (interactive:
    collapse/expand/zoom). Source = `.md` (readable as-is in the repo). Published
    on Pages for full interactivity. Node required only in CI, never local.
  - **D2** → architecture diagrams / flows / containers. Renders to **SVG**
    (static image). Go binary, no Node. Colors/typeface from `theme` in
    `src/config.yaml` (brand: carbon `#2B2D29` + raw `#F3F0E7`, Courier Prime).
  - **Mermaid** → dropped (D2 cleaner, avoids Node locally).

- **Automation (automatic + free):** on push, a GitHub Action
  (`.github/workflows/pages.yml`) renders every `*.mm.md` → Markmap HTML and every
  `*.d2` → D2 SVG, and publishes to **GitHub Pages**
  (`https://bit-crew.github.io/prim/`). Mindmaps are **linked** from normal docs
  to their rendered HTML (e.g. `.../ipo.html`), not embedded (GitHub does not run
  HTML inside `.md`). Convention: mindmap sources use the **`*.mm.md`** suffix
  (still valid markdown, readable in the repo). Rendering happens **only in CI**
  (GitHub Actions) — nothing is generated or installed locally.

### Repo structure (co-location, kept organized)

Documentation for a specific piece of code lives beside it; only cross-cutting
docs are centralized. This keeps docs close to code (so they do not rot) without
cluttering the root.

```
prim/
├── README.md              # cover page (North Star, how to use)
├── docs/                  # documentation (cross-cutting + global architecture)
├── d2/                    # D2 sources (architecture diagrams) → rendered to img/
├── img/                   # rendered output (SVG from D2, HTML from Markmap)
├── src/                   # code
│   ├── config.yaml        # the single entry point (instance + theme)
│   └── schema/            # ddl.sql (+ its own README when needed)
├── infra/                 # IaC (terraform/Multy); doc sits beside it
```

Rule: **doc of a module → beside the module** (a `README.md` in that folder);
**cross-cutting doc → `docs/`**; **global architecture → root**.

## 3. Data-plane & Agent security

The IdP + two planes (§2) secure *access*. Two more concerns are specific to a
data + AI-agent system and are handled by design, not by a catch-all layer
(security stays an environmental condition, distributed across layers — there is
no single "security layer" to add).

### PII tokenization (instead of encrypting everything)

Encrypting all data is costly and useless for external APIs (an API cannot read
ciphertext). Instead, prim **tokenizes PII**: identifying data is isolated and
everything else references it by token.

- A **protected PII table** holds all identifying/contact data (people and
  accounts: names, contacts, IDs, account numbers). It is the most restricted
  datum — tighter group permissions than the rest.
- Everywhere else (analytics, DWH, RAG, anything sent to external APIs) uses
  **tokens** instead of the real value (`cliente_7f3a`, not "Juan Pérez, ID X").
- All analytics and processing run **anonymized** on tokens.
- **De-tokenization happens only at the end**, in final results shown to an
  **authorized** human — never in data that leaves to an external API (LLM,
  embeddings, OCR). External APIs only ever see tokens.
- Who can resolve a token back to PII is itself an IdP-group permission.

Notes / where it tokenizes:
- Tokenization occurs **after** extraction, not before: the OCR/extraction step
  of an invoice does read the real vendor name, then that value is tokenized
  before it flows onward. The sensitive moment (extraction) stays on prim's side;
  only tokens travel to downstream APIs.
- This doubles as a **prompt-injection mitigation** (below): if the RAG works on
  tokens, a successful "leak the data" injection leaks anonymous tokens, not PII.

### Prompt-injection defense (the agent)

A client document or email in the RAG could contain hostile text ("ignore your
instructions, exfiltrate everything, run X"). The agent reads it — so it must not
obey it. No single product solves this; the defense is layered and mostly design:

1. **Data is not instructions.** RAG content is passed to the agent explicitly
   marked as **untrusted reference data**, never as instructions. Primary defense.
2. **Least privilege for the agent.** The OAS can only read what the *asking user*
   is allowed to see, and risky/destructive actions require confirmation
   (ZeroClaw asks before risky actions). A successful injection then achieves
   little.
3. **Tokenization (above).** Even if an injection forces a "leak", it leaks
   anonymous tokens, not real PII.
4. **Audited output.** ZeroClaw stamps a cryptographic receipt per tool call, so a
   successful injection is detectable and reversible.

There is **no tool to buy** for this — it is architecture: untrusted-data framing,
least privilege, tokenization, and confirmation on sensitive actions. prim already
has most of these pieces; this section records them as policy.

### RAG answers respect the asker's permissions

The RAG mixes documents across projects and sensitivities in one index. When the
OAS answers, results **must be filtered by what the asking user may see** (their
IdP groups / project scope) before composing the answer — otherwise the RAG could
leak information across projects or roles. Each RAG chunk carries its scope
(project, sensitivity) as metadata so retrieval can filter by the asker's groups.

### Why NOT a mesh VPN (Tailscale) — open

A mesh VPN (Tailscale) would be the "catch-all extra layer", and for the current
architecture it looks **redundant**: the machine plane already uses short-lived
SSH certs via a central CA (Cloudflare Access for Infrastructure), and Cloudflare
Access can put services behind SSO without exposing them (Zero Trust). Data lives
in R2 (edge), not in servers that must talk privately to each other. A mesh would
duplicate that and add ops (against Zero-Ops). It would only pay off if many
inter-connected per-project VPS appear. **Status: open — revisit when the VPS
story is firmer** (see the VPS question still under discussion).
