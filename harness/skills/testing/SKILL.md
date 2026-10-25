---
id: "testing"
name: "Testing"
description: "HOW to implement tests using the Engine Pattern. Use when writing, structuring, or refactoring tests."
scope: "Test implementation"
activation: "When implementing or structuring tests"
tags: [testing, pytest, parametrize, data-driven]
allowed-tools: [Read, Write, Glob, Grep]
---

# Testing Skill

Data-Driven Engine Pattern: One driver function, infinite scenarios.

## Core Rule

Logic != Data. Tests contain ZERO logic — all variations live in `TEST_SCENARIOS`.

## Components

### 1. TARGET_FUNCS
List of implementations to test against the same scenarios.

```python
TARGET_FUNCS = [("polars", read_polars), ("orjson", read_orjson)]
```

### 2. TEST_SCENARIOS
Global dictionary. Each key = scenario name, each value = input data + validators per implementation.

```python
TEST_SCENARIOS = {
    "empty": {
        "data": [],
        "validators": {
            "polars": lambda r: r.height == 0,
            "orjson": lambda r: r == [],
        },
    },
    "valid_single": {
        "data": [{"id": 1, "name": "test"}],
        "validators": {
            "polars": lambda r: r.height == 1,
            "orjson": lambda r: len(r) == 1,
        },
    },
    "error_corrupt": {
        "data": "not_json",
        "validators": {
            "polars": lambda r: isinstance(r, Exception),
            "orjson": lambda r: isinstance(r, Exception),
        },
        "expect_error": True,
    },
}
```

### 3. Factory Fixture
ONE generic fixture that generates test files from scenario data.

```python
@pytest.fixture
def json_factory(tmp_path):
    def _create(data):
        path = tmp_path / "test.json"
        path.write_text(orjson.dumps(data).decode() if not isinstance(data, str) else data)
        return path
    return _create
```

### 4. Engine (The ONLY test function)

```python
@pytest.mark.parametrize("func_id, func_impl", TARGET_FUNCS)
@pytest.mark.parametrize("scenario", TEST_SCENARIOS.keys())
def test_engine(json_factory, func_id, func_impl, scenario):
    cfg = TEST_SCENARIOS[scenario]
    path = json_factory(cfg["data"])

    if cfg.get("expect_error"):
        with pytest.raises(Exception) as exc:
            func_impl(path)
        result = exc.value
    else:
        result = func_impl(path)

    assert cfg["validators"][func_id](result), f"{func_id}/{scenario} failed"
```

## Workflow

1. Identify production functions to test → `TARGET_FUNCS`
2. Define all scenarios (happy, edge, error) → `TEST_SCENARIOS`
3. Build ONE factory fixture for input generation
4. Write ONE `test_engine` with `@pytest.mark.parametrize`
5. Run: `pytest -v` shows matrix of func × scenario

## Adding a New Scenario

Only touch `TEST_SCENARIOS` — add a key with data + validators. Zero code changes elsewhere.

## Adding a New Implementation

Only touch `TARGET_FUNCS` — add tuple. Add validator key to each scenario.

## Anti-Patterns

- NEVER: `test_read_empty()`, `test_read_valid()`, `test_read_corrupt()` as separate functions.
- NEVER: `if/else` inside a test function.
- NEVER: hardcoded JSON/CSV inside test body.
- NEVER: one fixture per specific file (`fixture_valid`, `fixture_invalid`).
- NEVER: manual `try-except` — use `pytest.raises` or validators dict.
