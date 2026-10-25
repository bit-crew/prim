---
name: skill-design
description: Create and configure Claude Code skills using the Anthropic Agent Skills Open Standard. Use when user wants to build, structure, or troubleshoot skills with proper metadata, progressive disclosure, or allowed-tools.
allowed-tools: Read, Grep, Glob, Write, Edit, Bash
---

# Skill Design Skill

Follow the Anthropic Agent Skills Open Standard when creating skills.

## Required Metadata Fields

- **name** — Lowercase, numbers, hyphens. Max 64 chars. Matches directory name.
- **description** — What the skill does AND when to use it. Claude uses this to match.

## Optional Metadata Fields

- **allowed-tools** — Restricts tools when skill is active. Useful for read-only or security-sensitive workflows.
  > Note: `model` field is provider-specific (omit or set accordingly in your opencode.json).

## Writing Effective Descriptions

A good description answers:
1. What does the skill do?
2. When should Claude use it?

Include keywords matching how users phrase requests.

## Progressive Disclosure

Keep SKILL.md under 500 lines. Detailed content goes in:
- `references/` — Additional documentation
- `scripts/` — Executable code (runs without loading into context)
- `assets/` — Templates, images, config files

Link supporting files with clear instructions on when to load them.

## Directory Structure

```
skill-name/
├── SKILL.md
├── references/
│   └── detailed-guide.md
└── scripts/
    └── validate.sh
```

## Frontmatter Template

```yaml
---
name: my-skill
description: One sentence: what it does + when to use it.
allowed-tools: Read, Grep, Glob, Bash
---
```