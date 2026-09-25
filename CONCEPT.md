# Kestrel: Concept & Axioms

Kestrel is an opinionated framework for building high-performance, low-maintenance systems through architectural minimalism and data sovereignty.

## 1. Framework Axioms

Non-negotiable principles that guide every technical and business decision:

| Principle | Statement | Rejection Criteria | Implementation |
| :--- | :--- | :--- | :--- |
| **Minimalism** | One tool per need; Library > Service. | Solutions adding complexity or tech debt. | Functional stream (orjson + generators) + FAISS indexer + in-DB CDC log as queue. |
| **Observability** | State is governed by logs and alerts. | Manual intervention or BI dashboards. | LLM Agent reporting + CEP alerts. |
| **Single Source of Truth** | Bifurcated Persistence + Backend-First. | Data silos or multiple repositories. | Backend logic only; TDS + IDS domains. |

## 2. Technology Summary (The Stack)

| Component | Tool | Rationale |
| :--- | :--- | :--- |
| **TDS (OLTP)** | **Embedded SQL DB** | Row-Store for current state and logical catalog. Concrete engine in [docs/ops/stack-selection.md](./docs/ops/stack-selection.md#transactional-db--queue-decided-turso). |
| **IDS (OLAP)** | **S3/GCS + Iceberg** | Parquet + Metadata for ACID on the data lake. (Iceberg vs flat Parquet to revisit at scale.) |
| **GOP** | **Git (Any platform)** | SoT for code, DaC, and IaC (code hosting only — not the identity provider; see stack-selection.md). |
| **IaC** | **Multy.dev (Terraform)** | Cloud-agnostic abstraction layer. |
| **EPE** | **orjson + iterators** | Functional stream processing, memory-efficient. |
| **Queue/Events** | **In-DB CDC / event log** | Change log as queue ("the log is the queue"): no broker, no dual-write. Concrete engine in [stack-selection.md](./docs/ops/stack-selection.md#transactional-db--queue-decided-turso). |
| **AQE (Ad-hoc)** | **DuckDB** | Fast, in-process SQL queries on OLAP files. |
| **AQE (Batch)** | **Spark / Trino** | Large-scale batch processing on demand. |
| **OAS (Assistant)** | **TBD (see docs/ops/stack-selection.md)** | Operational Assistant: conversational interface layer over RAG. |
| **CAG (Coding Agent)** | **TBD (see docs/ops/stack-selection.md)** | Coding Agent: writes and executes code (e.g. Kiro). |

## 3. The "Zero-New-Ops" Rule

Strategic hierarchy to minimize Operational Technical Debt (Priority 1/2 is the goal).

| Priority | Strategy | Core Logic | Impact |
| :--- | :--- | :--- | :--- |
| **Level 1** | **File/Library-based** | Ephemeral compute + Object Storage (S3/GCS). | **Zero Ops:** Pure storage cost. |
| **Level 2** | **Serverless** | Managed services that scale to zero. | **Low Ops:** Config only. |
| **Level 3** | **Dedicated Infra** | Dedicated VMs or persistent clusters. | **High Ops:** Patches, security. |

## 4. Opinionated Environment

Recommended tools for a terminal-centric workflow:
- **Python:** Manage packages exclusively with **`uv`**.
- **Editor:** **Helix** (modal). **Git:** **Lazygit** (TUI). **Layout:** **Zellij**.
- **Connection:** **Mosh** (stable connections for mobile).
