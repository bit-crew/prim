# Kestrel

The Opinionated Architecture Framework. Kestrel defines a philosophy for building and managing high-performance systems with zero operational overhead and data sovereignty.

## Architectural Pillars

- **[CONCEPT.md](./CONCEPT.md):** Axioms, the Zero-Ops rule, and the opinionated system stack.
- **[FABRIC.md](./FABRIC.md):** Data domains (TDS/IDS), persistence guarantees, and observability.
- **[GOVERNANCE.md](./GOVERNANCE.md):** Channel-centric demand flows, GitOps, and Zero-Trust security (YubiKey/SSH access model).
- **[IPO.md](./IPO.md):** Symptom-diagnosis map based on the Input-Process-Output framework.

## Runbook & Backlog

- **[BOOTSTRAP.md](./BOOTSTRAP.md):** From absolute zero to a running instance (root identity → Cloudflare → code host → compute → automation).
- **[TODO.md](./TODO.md):** Scope, architecture, agent (OAS) design, and pending decisions.

## Reference Docs (`docs/`)

- **[docs/ops/stack-selection.md](./docs/ops/stack-selection.md):** Stack decisions — access model, data layers, DB/queue (Turso + CDC), OAS candidates, code host.
- **[docs/ops/principios-operativos.md](./docs/ops/principios-operativos.md):** Operating principles (minimalism, technological governance, ROE, debt, quality).
- **[docs/governance/votaciones-documentos.md](./docs/governance/votaciones-documentos.md):** Voting and legal documents pattern (review pending).
- **[docs/fiscal/guia-tributaria-sas.md](./docs/fiscal/guia-tributaria-sas.md):** Tax guide for a Colombian SAS (DIAN, UVT, reporting).

---
"Keep it Simple. Stay Sovereign."
