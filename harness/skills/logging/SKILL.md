---
id: "logging"
name: "Logging"
description: "HOW to implement structured logging. Use Wide Event pattern: one unit = one structured JSON event upon completion."
scope: "Logging implementation"
activation: "When implementing logs"
tags: [logging, json, observability]
allowed-tools: [Read, Write]
---

# Logging Skill

Wide Event pattern: accumulate metadata in memory → flush once at completion.

## Pattern

```python
from assets.logger import wide_event

with wide_event("pipeline_name", "production", "my_pipeline") as log:
    log.set_metric("rows_processed", count)
    log.set_context("environment", "prod")
    
    with log.step("extract"):
        # extraction logic
        log.set_metric("source_rows", 100)
    
    with log.step("transform"):
        # transformation logic
        log.set_metric("output_rows", 50)
```

## Output

```json
{
  "timestamp": "2026-01-15T10:30:00Z",
  "event": "pipeline_name",
  "status": "success",
  "total_duration_ms": 150.2,
  "context": {
    "pipeline": "my_pipeline",
    "run_id": "abc-123",
    "environment": "prod"
  },
  "metrics": {
    "rows_processed": 50,
    "source_rows": 100,
    "output_rows": 50
  },
  "steps": {
    "extract": {"duration_ms": 50.1, "source_rows": 100},
    "transform": {"duration_ms": 100.1, "output_rows": 50}
  }
}
```

## Required Fields

- `timestamp`: ISO 8601
- `event`: name
- `status`: success | failed
- `total_duration_ms`: execution time
- `context`: pipeline, run_id, environment
- `metrics`: key-value pairs
- `steps`: step_name → {duration_ms, ...}