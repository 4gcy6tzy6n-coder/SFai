# ADR-023: Three-layer model memory — Index / Wiki / Way

**Date:** 2026-11-16
**Status:** Accepted (project-owner design)
**Context:** Memory must not be Wiki-only. Anthropic's Claude Code memory
subsystem (CLAUDE.md project memory + MEMORY.md/topic auto-memory + session
summaries + progressive disclosure; see https://code.claude.com/docs/en/memory
and source analyses) showed the multi-layer shape. The owner defined sfai's
three layers: **Index / Wiki / Way**, with the Wiki bridging the other two.

## Architecture

| Layer | Role | Implementation |
|---|---|---|
| **Index** | addressing & retrieval: everything searchable, refs resolve | FTS5 `search_docs/search_fts`, `context.search/read/resolve`, root index |
| **Wiki** | derived structured knowledge; **承接 index 与 way 的内容** | M4 compiler (`wiki_nodes/wiki_edges`), plus `way` nodes bridged from Way notes |
| **Way** | the agent's *method memory*: how each task was done, wins, and — **emphasized — mistakes**, self-written by the agent | `memory_notes(kind='way')`, `way.write` op, auto-extraction at run end |

Retrieval path: **index → wiki → way** — search locates, wiki structures,
way teaches how to work.

## Way layer semantics (core)

1. **Every terminal run leaves a Way entry** (completed / blocked / failed) —
   mistakes live in failed runs and must not be lost.
2. **Mistakes are the emphasized section** (`## mistakes (what went wrong)`),
   auto-extracted from tool errors (`error_code`s), acceptance issues, and
   run outcomes.
3. **The agent writes it itself**: the kernel intercepts `way.write`
   (method / wins / mistakes / lesson) as a first-class memory op — model
   reflections are never overwritten by auto-extraction (distinct identity).
4. **Revision semantics**: same (task, kind, topic, title) revises instead of
   duplicating (MEMORY.md-style maintenance).
5. **Envelope exposure**: Way notes ride into the model-visible envelope first
   (bounded, progressive disclosure) so lessons shape the next decide step.
6. **Wiki bridge**: each Way note becomes a `way` wiki node with a
   `relevant_to` edge to the task node — the Wiki 承接 index 和 way 的内容.

## Prior layers (kept)

- `project` notes: CLAUDE.md-style persistent instructions.
- `topic` notes: cross-task knowledge (MEMORY.md index materializes all notes).
- `task_summary` + `log`: auto episodic memory.

## Consequences

- E8/routing telemetry feeds Way: loop-guard trips and misroutes are mistakes.
- Future: staleness/decay of way notes, per-user ACL, and model-driven way
  *retrieval* (`way.lookup`) to close the index → wiki → way loop end-to-end.
