# Prim

The central layer for **data, services and identity**. Prim is not a VPS framework — it is the shared core that projects plug into: centralized storage and databases, processing and editing services, communication and AI, all behind a single identity with role-based permissions. Per-project compute (VPS/VPC) is optional and decided by each project, not by Prim.

## North Star (the binding vision)

> This is the horizon. Every decision and change must serve it. If a change contradicts this, the change is wrong.

**Prim is a one-command, self-service bootstrap for your own operating core.** Someone arrives at the repo, finds **everything they need documented**, gathers their prerequisites, runs **a single command**, and ends up with a fully provisioned core **in their own accounts** — not a SaaS, not someone else's tenant.

**What the user brings (prerequisites, documented in the repo):**
- Turso account + API key (transactional DB)
- Cloudflare account + API key (R2 storage + edge)
- Their main repository (GOP)
- Company name
- Brand colors + typeface
- LLM API key (optional)

**How it runs:**
1. The user copies **one command** from the documentation (a `curl ... | sh`-style installer).
2. It either **asks for each value interactively**, one by one, **or** loads a ready **`config.yaml`** (unattended mode).
3. It **creates everything**: storage, DB, services, identity, theme — provisioned into the user's own accounts.

**Non-negotiable principles derived from this:**
- **Self-service, self-hosted-by-the-user:** the core lives in the user's accounts (their Turso, their Cloudflare, their repo). Prim provisions; it does not host for them.
- **Supported platforms are a fixed, finite set:** the installer automates against concrete platforms (Cloudflare, Turso, a supported GOP, a supported LLM provider). It cannot and will not support "any platform" — infinite support is explicitly out of scope.
- **`config.yaml` is the single entry point:** instance identity, accounts, GOP, theme (colors/typeface) and optional LLM all live there. Interactive mode just fills the same file.
- **One command, zero manual wiring:** if a step needs hand-holding outside the installer, that is a gap to close, not the intended experience.

## What Prim provides (centralized)

- **Data:** Cloudflare R2 (object storage) + DWH/DB, with roles/permissions.
- **Services:** communication (chat/channel), calendar, video calls, tickets/tasks, project tracking, code repository, text/data editing.
- **AI:** the operational assistant (OAS) over a shared RAG.
- **Identity & permissions:** one identity provider (IdP), one set of groups → permissions propagated to every resource.

## Two access planes (one identity)

Prim exposes everything through **two planes with different privilege**, both backed by the **same IdP**:

- **Browser plane (low privilege, consume):** log in with user + password / SSO in the front (Mattermost — candidate) and click through to services: view data (DuckDB-WASM, spreadsheet component), video calls, tickets, calendar, project tracking. Interact in place; do not download raw data.
- **Machine plane (high privilege, produce):** a developer's machine (PC/tablet/VPC) authenticates with a **short-lived SSH certificate** signed by a central CA after the same SSO login. Grants access to data for development, pushing code, and writing the employee's own data to R2. Centrally scoped and revocable (remove from the IdP group → the certificate stops being issued and expires).

## Architectural Pillars

- **[CONCEPT.md](./docs/CONCEPT.md):** Axioms, the Zero-Ops rule, the two-plane access model, and the opinionated stack.
- **[FABRIC.md](./docs/FABRIC.md):** Data domains (TDS/IDS), persistence guarantees, and observability.
- **[GOVERNANCE.md](./docs/GOVERNANCE.md):** Channel-centric demand flows, GitOps, and the identity/permissions model (IdP + two planes).
- **[IPO (interactive mindmap)](https://bit-crew.github.io/prim/ipo.html):** Symptom-diagnosis map based on the Input-Process-Output framework. Source: `docs/ipo.mm.md`.

## Runbook & Backlog

- **[BOOTSTRAP.md](./docs/BOOTSTRAP.md):** From absolute zero to a running core (R2 → IdP → SSO services → group-to-permission mapping). Per-project VPS is optional.
- **[TODO.md](./TODO.md):** Scope, architecture, agent (OAS) design, and pending decisions.

## Reference Docs (`docs/`)

- **[docs/stack-selection.md](./docs/stack-selection.md):** Stack decisions — access model (IdP + two planes), data layers, DB/queue, OAS candidates, code host.
- **[docs/principios-operativos.md](./docs/principios-operativos.md):** Operating principles (minimalism, technological governance, ROE, debt, quality).
- **[docs/votaciones-documentos.md](./docs/votaciones-documentos.md):** Voting and legal documents pattern (review pending).
- **[docs/guia-tributaria-sas.md](./docs/guia-tributaria-sas.md):** Tax guide for a Colombian SAS (DIAN, UVT, reporting).

---
"Keep it Simple. Stay Sovereign."
