# Prim: Concept & Axioms

Prim is the **central layer for data, services and identity** — an opinionated core for building high-performance, low-maintenance systems through architectural minimalism and data sovereignty. Projects consume the core; per-project compute (VPS/VPC) is optional, not mandatory.

## 1. Framework Axioms

Non-negotiable principles that guide every technical and business decision:

| Principle | Statement | Rejection Criteria | Implementation |
| :--- | :--- | :--- | :--- |
| **Minimalism** | One tool per need; Library > Service. | Solutions adding complexity or tech debt. | Functional stream (orjson + generators) + FAISS indexer + in-DB CDC log as queue. |
| **Observability** | State is governed by logs and alerts. | Manual intervention or BI dashboards. | LLM Agent reporting + CEP alerts. |
| **Single Source of Truth** | Bifurcated Persistence + Backend-First. | Data silos or multiple repositories. | Backend logic only; TDS + IDS domains. |
| **Identity is the center** | One identity, one set of groups; permissions propagate to every resource. | Per-resource ad-hoc credentials; identity tied to a device or a VPS. | Single IdP → groups → R2/DWH/services. |
| **Access, don't hoard** | Facilitate access; keep data at the source. | "Download-and-copy" workflows. | View/query in place (DuckDB-WASM); constrain exfiltration, not access. |

## 2. Two Access Planes (one identity)

Prim is reached through **two planes of different privilege, backed by the same IdP**. This is the core of the access model (details in [GOVERNANCE.md](./GOVERNANCE.md#2-access--identity-idp--two-planes)).

| Plane | Privilege | Login | Grants | Revocation |
| :--- | :--- | :--- | :--- | :--- |
| **Browser** | Low (consume) | User+password / SSO in the front | View data, chat, calendar, video, tickets, project tracking, edit final reports | Instant (SSO session/group) |
| **Machine** | High (produce) | Same SSO → short-lived SSH certificate (central CA) | Data for development, push code, write own data to R2 | Central: remove from group → cert stops issuing, expires in hours |

> **DECISIÓN PENDIENTE (IdP):** lean = **Cloudflare Access** as the single IdP (CF already hosts R2/edge; "start simple"). Alternatives: Authentik/Zitadel (self-hosted). See [stack-selection.md](./docs/ops/stack-selection.md#access-model).
> **DECISIÓN PENDIENTE (machine auth):** lean = **SSH with a short-lived CA** via **Cloudflare Access for Infrastructure** (managed). Alternative: Smallstep/Teleport (self-hosted). Optional 2nd factor: YubiKey to sign the cert (adds security, is not the identity). See [stack-selection.md](./docs/ops/stack-selection.md#access-model).
> **DECISIÓN PENDIENTE (front):** lean = **Mattermost** (SSO). Slack requires a paid plan for SSO. See [stack-selection.md](./docs/ops/stack-selection.md#access-model).
> **DECISIÓN PENDIENTE (permissions granularity):** start **coarse-grained** (by group/prefix/schema); refine to fine-grained only when it hurts (YAGNI).

## 3. Technology Summary (The Stack)

| Component | Tool | Rationale |
| :--- | :--- | :--- |
| **Identity (IdP)** | **Single IdP (DECISIÓN PENDIENTE: Cloudflare Access lean)** | One identity + groups for both planes; permissions propagate to every resource. |
| **Machine auth** | **Short-lived SSH CA (DECISIÓN PENDIENTE: CF Access for Infrastructure lean)** | Central, revocable, ephemeral certs. No permanent keys copied per host. |
| **TDS (OLTP)** | **Embedded SQL DB** | Row-Store for current state and logical catalog. Concrete engine in [docs/ops/stack-selection.md](./docs/ops/stack-selection.md#transactional-db--queue-decided-turso). |
| **IDS (OLAP)** | **R2 + Parquet** | Columnar on object storage. (Iceberg vs flat Parquet to revisit at scale.) |
| **GOP** | **Git (Any platform)** | SoT for code, DaC, and IaC (code hosting; see stack-selection.md). |
| **IaC** | **Multy.dev (Terraform)** | Cloud-agnostic abstraction layer. |
| **EPE** | **orjson + iterators** | Functional stream processing, memory-efficient. |
| **Queue/Events** | **In-DB CDC / event log** | Change log as queue ("the log is the queue"): no broker, no dual-write. Concrete engine in [stack-selection.md](./docs/ops/stack-selection.md#transactional-db--queue-decided-turso). |
| **AQE (Ad-hoc)** | **DuckDB (incl. DuckDB-WASM in browser)** | Fast, in-process SQL queries on OLAP files, client- or server-side. |
| **AQE (Batch)** | **Spark / Trino** | Large-scale batch processing on demand. |
| **Front (browser plane)** | **DECISIÓN PENDIENTE (Mattermost lean)** | Communication + calendar + tickets + video; SSO entry point. |
| **OAS (Assistant)** | **TBD (see stack-selection.md)** | Operational Assistant: conversational interface layer over RAG. |
| **CAG (Coding Agent)** | **TBD (see stack-selection.md)** | Coding Agent: writes and executes code (e.g. Kiro). |

## 4. The "Zero-New-Ops" Rule

Strategic hierarchy to minimize Operational Technical Debt (Priority 1/2 is the goal). The centralized core lives at Level 1/2; per-project VPS (Level 3) is opt-in, not default.

| Priority | Strategy | Core Logic | Impact |
| :--- | :--- | :--- | :--- |
| **Level 1** | **File/Library-based** | Ephemeral compute + Object Storage (R2). | **Zero Ops:** Pure storage cost. |
| **Level 2** | **Serverless / Managed** | Managed services (IdP, edge Workers, front) that scale to zero. | **Low Ops:** Config only. |
| **Level 3** | **Dedicated Infra** | Per-project VMs (optional, project's decision). | **High Ops:** Patches, security. |

## 5. Opinionated Environment

Recommended tools for a terminal-centric workflow (developer's own machine, provisioned by `dev-setup`):
- **Python:** Manage packages exclusively with **`uv`**.
- **Editor:** **Helix** (modal). **Git:** **Lazygit** (TUI). **Layout:** **Zellij**.
- **Connection:** **Mosh** (stable connections for mobile).
