.PHONY: install test lint format validate clean pre-commit install-all

PYTHON := python
PYTEST := pytest
RUFF := ruff
PRECOMMIT := pre-commit

install:
	$(PYTHON) -m pip install -e ".[dev]"

install-all: install pre-commit

pre-commit:
	$(PRECOMMIT) install && $(PRECOMMIT) install --hook-type commit-msg --hook-type pre-push

test:
	$(PYTEST) -v

lint:
	$(RUFF) check .

format:
	$(RUFF) format .

validate: lint
	$(RUFF) format --check .
	bandit -r skills/ || true

clean:
	rm -rf __pycache__/ .pytest_cache/ .ruff_cache/
	rm -rf *.egg-info build dist

pre-commit-run:
	$(PRECOMMIT) run --all-files

.DEFAULT_GOAL := install