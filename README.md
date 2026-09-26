# Prim

The central layer for **data, services and identity**. Prim is not a VPS framework — it is the shared core that projects plug into: centralized storage and databases, processing and editing services, communication and AI, all behind a single identity with role-based permissions. Per-project compute (VPS/VPC) is optional and decided by each project, not by Prim.

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

- **[CONCEPT.md](./CONCEPT.md):** Axioms, the Zero-Ops rule, the two-plane access model, and the opinionated stack.
- **[FABRIC.md](./FABRIC.md):** Data domains (TDS/IDS), persistence guarantees, and observability.
- **[GOVERNANCE.md](./GOVERNANCE.md):** Channel-centric demand flows, GitOps, and the identity/permissions model (IdP + two planes).
- **[IPO.md](./IPO.md):** Symptom-diagnosis map based on the Input-Process-Output framework.

## Runbook & Backlog

- **[BOOTSTRAP.md](./BOOTSTRAP.md):** From absolute zero to a running core (R2 → IdP → SSO services → group-to-permission mapping). Per-project VPS is optional.
- **[TODO.md](./TODO.md):** Scope, architecture, agent (OAS) design, and pending decisions.

## Reference Docs (`docs/`)

- **[docs/ops/stack-selection.md](./docs/ops/stack-selection.md):** Stack decisions — access model (IdP + two planes), data layers, DB/queue, OAS candidates, code host.
- **[docs/ops/principios-operativos.md](./docs/ops/principios-operativos.md):** Operating principles (minimalism, technological governance, ROE, debt, quality).
- **[docs/governance/votaciones-documentos.md](./docs/governance/votaciones-documentos.md):** Voting and legal documents pattern (review pending).
- **[docs/fiscal/guia-tributaria-sas.md](./docs/fiscal/guia-tributaria-sas.md):** Tax guide for a Colombian SAS (DIAN, UVT, reporting).

---
"Keep it Simple. Stay Sovereign."
