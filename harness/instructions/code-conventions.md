---
inclusion: always
---

# Engineering Principles

## Project Structure

### Brevity
- Short is better. Fewer lines = lower maintenance.
- Duplicate trivial code (<10 lines) rather than fragmenting.

### Classes as Containers
- Use classes exclusively as containers for vectorized/functional methods.
- No internal state complexity.
- No deep inheritance.

### Iterators
- Use Iterators/Generators and List/Dict Comprehensions.
- Avoid `for` loops for data transformations.

### Anti-SOLID
- No abstractions of abstractions.
- No interfaces, factories, or deep inheritance.
- No premature abstractions.

## 1. Vectorized > Functional
- Always prefer vectorized/columnar operations over iteration.
- Use engine-native expressions (Polars, Spark SQL, DuckDB).
- Only use functional when vectorized is not available.

```python
# DO: Engine-native
df.select(pl.col("x") * 2)

# DON'T: UDF
df.select(udf_multiply("x"))
```

## 2. Minimalism
- Short is better. Fewer lines = lower maintenance.
- YAGNI: Do not implement features until strictly required.
- KISS: Avoid cleverness.
- DRY (3+ Rule): Do not abstract until logic is repeated 3 times.

```python
# DO: Duplicate <10 lines
df.select([...])  # repeated 2x is fine

# DON'T: Premature abstraction
class MyProcessor:  # create wrapper before 3 uses
```

## 3. Always Optimize Resources
- Memory and speed are always priorities.
- Prefer packages/libraries over running services.
- Reduce technical debt whenever possible.

## 4. Wide Logs
- Use [Logging Skill](../skills/logging/SKILL.md).
- One unit = One structured JSON event upon completion.
- Accumulate metadata in memory and flush once.

## 5. Cross-Cutting Optimizations
- Lazy Evaluation: Prefer lazy execution for query optimization.
- Predicate/Filter Pushdown: Filter -> Project -> Join (late).
- Filter Before Cast: Always apply filters before type casts.
- Cast Filter to Column Type: Adjust filter value to match column type, not the reverse.
- Shuffle Reduction: Limit or eliminate operations that trigger data shuffling across the network.
- Broadcast Joins: When one side fits in memory.
- Caching: Surgical use only when recomputation > storage cost.
- Anti-UDF: UDFs are a failure of design.

```python
# DO: Filter -> Project -> Join
df.lazy().filter(col("status") == "active").select("id", "name").join(other, on="id")

# DON'T: Collect early
df.collect().filter(...)
```

## 6. Error Handling

**Paradigm Integrity — NEVER mix paradigms in error handling.**

- NEVER use `try-except` to fallback from vectorized to iterative code.
- NEVER catch errors to retry with a slower method.
- Handle errors inside the expression tree (`when/then/otherwise`, `fill_null`, `coalesce`).
- Streaming/Functional: Railway Oriented Programming — functions return `(Success, Failure)` tuple or Result object, never raise.
- DLQ Pattern: Create error columns via `pl.when().then().otherwise()` to keep execution vectorized.

```python
# FORBIDDEN: Paradigm switching
try:
    result = df.with_columns((pl.col("val") / pl.col("factor")).alias("res"))
except ComputeError:
    result = df.map_rows(lambda row: safe_div(row[0], row[1]))

# CORRECT: Handle inside expression tree
result = df.with_columns(
    pl.when(pl.col("factor") != 0)
    .then(pl.col("val") / pl.col("factor"))
    .otherwise(None)
    .alias("res")
)
```

## 7. Module Organization

- Function used in ONE module → lives in THAT module.
- Function used in 2 modules → evaluate if truly the same logic.
- Function used in 3+ modules → move to `common/`.
- NEVER create `utils.py`, `helpers.rs`, `common.go` without 3+ real consumers.
- Limits: <500 lines per module, <50 lines per function.
- Private helpers at beginning or end of file.
- Flat is better than nested.

```
# BAD: Unnecessary separation
src/
├── users.py
├── user_validators.py      # Only used by users.py
└── user_transformers.py    # Only used by users.py

# GOOD: Cohesive
src/
└── users.py                # Contains validators + transformers inline
```

## 8. Reuse Criteria

Create shared utility ONLY if ALL conditions are true:
1. Repeated 3+ times in DIFFERENT modules.
2. Has >5 lines of complexity.
3. Improves real readability.
4. Makes sense independent of context.
5. Is not just to comply with DRY or SOLID.

If ANY condition is NO → keep inline. Copy-paste is better than wrong abstraction.

## 9. Testing

- One Logic = One Test Function (Engine Pattern).
- Zero Logic in Tests — no `if/else` inside test functions.
- Single Source of Truth: all variations in `TEST_SCENARIOS` dict.
- NEVER: `test_case_a`, `test_case_b`, `test_case_c` for the same function.
- NEVER: hardcoded data inside test functions.
- NEVER: manual `try-except` in tests (use `pytest.raises`).
- See [Testing Skill](../skills/testing/SKILL.md) for implementation.

## 10. Commits

- Format: `<type>(<scope>): <description>`
- Types: `feat`, `fix`, `perf`, `refactor`, `test`, `docs`, `chore`
- NEVER emojis in commits, code, or responses.

## 11. Agent Interaction

- NEVER apply changes before confirming with user.

## 12. Python Stack

| Use Case | Tool |
|----------|------|
| Excel files | `calamine` |
| JSON serialization | `orjson` |
| Schema validation (streaming) | `msgspec` |
| DataFrames | `polars` |
| Business rules on DataFrames | `pandera` (vectorized) |
| Logging | `structlog` |

**Tiered Validation (Performance First):**
1. Native schema in `pl.read_*()` — handles 90% at Rust level.
2. Pandera — only for complex business rules (ranges, regex, cross-column).
3. NEVER Pydantic or row-based loops to validate DataFrames.
