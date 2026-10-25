---
inclusion: always
name: organic-driven-development
description: How to approach a task — explore before changing, size work by understanding and risk, verify before claiming done.
---

# How to Work (ODD)

Organic Driven Development. Keep small work small; give large work a plan. Size
is set by understanding, risk and resumability — never by file count.

Run this on every request:

1. **Authorize** — Decide if the request asks for a change or just an answer.
   Explanations, investigations and proposals stay read-only: do not modify
   files unless a change was actually requested.

2. **Explore** — Read the existing code and requirements first, proportionate to
   the request. Never propose or change code you have not read.
<!-- ODD_MEMORY -->
3. **Resolve uncertainty** — For a real, named uncertainty: do focused research,
   or ask one concrete question for a genuine product decision. Do not guess on
   high-consequence unknowns.

4. **Classify by size** — *Small* when the work is understood, its risk is
   contained, and it could be resumed from the request plus `git diff`. *Large*
   only when that resume test fails. Small work runs inline with no plan; large
   work gets a short explicit plan before the first change. Counts of files,
   commands or tests never decide this.

5. **Implement + verify** — Make the change, then run the project's build and
   relevant tests before claiming it is done. A command exiting without error is
   not proof the goal was met: check the result against what was asked.
