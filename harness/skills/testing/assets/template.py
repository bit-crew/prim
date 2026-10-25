"""
Testing Engine Pattern Template.
Copy and adapt TARGET_FUNCS, TEST_SCENARIOS, and factory fixture to your project.
"""

import pytest
import orjson

# === 1. TARGET_FUNCS: implementations to test ===
# Replace with your actual production functions
TARGET_FUNCS = [
    ("impl_a", lambda path: ...),
    ("impl_b", lambda path: ...),
]

# === 2. TEST_SCENARIOS: data + validators ===
TEST_SCENARIOS = {
    "happy_path": {
        "data": [{"id": 1, "value": 100}],
        "validators": {
            "impl_a": lambda r: r.height == 1,
            "impl_b": lambda r: len(r) == 1,
        },
    },
    "empty": {
        "data": [],
        "validators": {
            "impl_a": lambda r: r.height == 0,
            "impl_b": lambda r: r == [],
        },
    },
    "error_case": {
        "data": "invalid_input",
        "validators": {
            "impl_a": lambda r: isinstance(r, Exception),
            "impl_b": lambda r: isinstance(r, Exception),
        },
        "expect_error": True,
    },
}


# === 3. Factory Fixture ===
@pytest.fixture
def json_factory(tmp_path):
    def _create(data):
        path = tmp_path / "test.json"
        content = orjson.dumps(data).decode() if not isinstance(data, str) else data
        path.write_text(content)
        return path

    return _create


# === 4. Engine: the ONLY test function ===
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
