# Kestrel: Data Fabric & Persistence

Kestrel uses a **Bifurcated Persistence** model to ensure data sovereignty, high performance, and total traceability.

## 1. Functional Domains (SoT)

| Domain | Role | Implementation |
| :--- | :--- | :--- |
| **TDS** | Transactional Data Store | Current OLTP state and logical catalog. |
| **IDS** | Immutable Data Store | Append-only storage: OLAP, blobs, and RAG vectors. |
| **GOP** | Git Operations Platform | Versioned persistence for code, DaC, and IaC. |

## 2. IDS Structure (Blob Storage)

```text
[project-name]-ids/
├── auth/                       # ACCESS & ENCRYPTION
│   ├── identity.kdbx           # Master KeePass file (Human-managed)
│   └── secrets.enc.yaml        # SOPS-encrypted secrets (Agent-managed)
├── RAG/                        # SEMANTIC BRAIN
│   └── brain_index/            # Vectorized context (FAISS/Index)
├── olap/                       # TELEMETRY & HISTORICAL
│   ├── logs/                   # Wide Logs (Parquet) (L_YYYY-MM-DD.parquet)
│   └── dumps/                  # States (Parquet) (S_YYYY-MM-DD_TYPE.parquet)
└── blob/                       # BINARY REPOSITORY
    └── [ID]_[YYYY-MM-DD]_[NAME].[EXT]
```

## 3. Persistence Guarantees

The architecture ensures data integrity via:
`Process → Local WAL → Batch → Blob Storage`

- **At-Least-Once:** Idempotency keys (`process_id` + `timestamp`) + read-time deduplication.
- **Crash Recovery:** Read pending local WAL on restart; delete only after S3/GCS confirmation.
- **Checkpointing:** Save state every N events to define recovery points.

## 4. Observability: Wide Events (Canonical Logging)

Processes emit ONE structured log event per unit of work upon completion.

| Field | Type | Notes |
| :--- | :--- | :--- |
| `timestamp` | ISO8601 UTC (µs) | Required |
| `trace_id` | String (UUID v4) | Required |
| `span_id` | String | Required |
| `level` | `INFO\|WARN\|ERROR` | Required |
| `event` | String (snake_case) | e.g. `etl_pipeline_success` |
| `duration_ms` | Float | Required for closing events |
| `context.*` | Mixed | Metadata (user_id, version, etc.) |

**Aggregation Pipeline:**
`[Span emitted] → Local WAL (Avro/RocksDB) → [Aggregation] → Object Storage (Parquet)`
