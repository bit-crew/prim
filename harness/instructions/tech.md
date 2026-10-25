---
inclusion: always
name: technology-stack
description: Frameworks, libraries, and development tools used in this workspace.
---

# Technology Stack

## Primary Tools

- **Polars** — Fast DataFrame library for Python
- **Apache Spark** — Distributed processing (via PySpark)
- **DuckDB** — SQL on Parquet files

## Python 3.12+

- ruff (linting)
- mypy (type checking)
- pytest (testing)
- pre-commit (hooks)

## Tool Patterns

### Polars
```python
# Schema en read
pl.read_csv("file.csv", schema={"a": pl.Int64})

# Conditional
pl.when(cond).then(val).otherwise(null)

# Lazy execution
df.lazy().filter(...).select(...).collect()
```

### Spark
```python
# Use DataFrame API only
from pyspark.sql.functions import broadcast
result = big_df.join(broadcast(small_df), "key")
```

### DuckDB
```python
# Reusable logic
CREATE VIEW name AS ...

# Export
COPY table TO 'output.parquet' (FORMAT PARQUET)
```

## Quality Commands

```bash
ruff check .
ruff format .
mypy .
```

## No Services

Prefer packages over running services unless strictly required.