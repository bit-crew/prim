---
name: knowledge-base
description: "HOW to use CLI-based persistent memory and RAG. Use when user wants local knowledge base or query answering."
allowed-tools: [Read, Grep, Glob, Write, Edit, Bash]
---

# Knowledge Base Skill

CLI-based persistent memory and RAG.

## Core Principles

- **Query/Add:** ALWAYS execute `scripts/query_engine.py`
- **Styles:** See [steering/visual-style.md](../steering/visual-style.md)
- **Token Efficiency:** Write compact JSON, retain only final results

## Architecture

```
knowledge-base/
├── SKILL.md              # This file
├── scripts/
│   └── query_engine.py   # CLI query/add engine
└── assets/
    ├── presentation.html
    └── chart.html
```

## Commands

### Add Knowledge
```bash
python scripts/query_engine.py --action add --payload '{"text": "your content", "tags": ["topic"]}'
```

### Query Knowledge
```bash
python scripts/query_engine.py --action query --payload '{"query": "your question"}'
```

## Workflow

1. User requests info → query
2. User provides new info → add
3. User wants slides/charts → Use steering/visual-style.md

## Limits

- Payloads under 2KB JSON
- SKILL.md under 500 lines