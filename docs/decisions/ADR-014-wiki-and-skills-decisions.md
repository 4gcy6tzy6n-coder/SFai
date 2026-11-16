# ADR-014: M4 Wiki compiler and M5 Skill packages (v0.1)

**Date:** 2026-11-16
**Status:** Accepted for v0.1 — revisit when M4 async scheduling / M7 multi-user land
**Context:** M4 (incremental Wiki and provenance) and M5 (dynamic capabilities
and Skills) were delivered on top of the M0–M3 core. This ADR records the
concrete choices so later milestones don't silently re-litigate them.

## M4 — In-memory increment, derived-only Wiki

1. **Derived-only writes.** `context/wiki.rs` writes solely to `wiki_nodes`,
   `wiki_edges`, `wiki_state`, `wiki_compile_log`. It never writes `tasks` or
   `events`; a failed compile therefore cannot corrupt task authority (tested).
2. **Material-change classifier.** Only these event types compile:
   `task.created`, `task.status_changed`, `tool.committed`,
   `task.error_opened`, `task.error_resolved`, `task.issue_recorded`.
   Intents/checkpoints/output events are skipped (tested).
3. **Incremental cursor.** `wiki_state.last_compiled_seq` bounds each run; a
   failed compile logs the failure and does **not** advance the cursor, so the
   next run retries the same events.
4. **Idempotent upserts.** Node identity is `(task_id, kind, node_key)`;
   content hash `sha256(title|summary)` compares before writing, so repeated
   ingestion never duplicates facts. Edges are `UNIQUE(task_id, from, to, type)`
   and `INSERT OR IGNORE`.
5. **Node/edge vocabulary.** Kinds: task, evidence, artifact, failure,
   verification (plus reserved constraint/decision). Edge types follow plan
   §5.2: `supported_by`, `produced_by`, `contradicts`, `verified_by`,
   `relevant_to`, `depends_on`, `requires`, `supersedes`.
6. **Contradictions.** Two evidence nodes with the same normalized title but
   different content hashes keep both nodes, gain `contradicts` edges in both
   directions, and are marked `disputed`.
7. **Stale refs.** Unresolvable refs become `stale` evidence nodes and are
   counted in the report; they are never silently dropped.
8. **Write amplification.** `CompileReport` reports events/nodes/edges and
   `write_amplification = (created+updated+edges) / events`; each compile is
   logged with its JSON report.
9. **Scheduling (v0).** Compilation runs at task completion and on demand
   (`sfai wiki compile`). Asynchronous/material-triggered compilation is
   deferred; the classifier and cursor already support it.

## M5 — Skill packages with progressive disclosure

1. **Package format.** `<skills_dir>/<name>/SKILL.md` with `---` frontmatter
   (`name`, `description`, `version`) followed by the instructions body.
   `skills_dir` defaults to `<storage>/skills`; CLI `--skills <dir>` overrides.
2. **Discovery returns metadata only.** `SkillManifest` has no instruction
   field, so instructions cannot leak through discovery (type-level guarantee
   plus tests).
3. **Activation is per-run.** `DecisionKernel.activated_skills` is reset per
   run; activation loads the instructions into that run's `ContextEnvelope`
   (`active_skills`, bounded to 4000 chars). A later run starts clean.
4. **Activation confers no permission.** Activated instruction text is data;
   tool execution still passes through `PolicyEngine`/`ToolBroker` exactly as
   before.
5. **Version pinning and trust.** Activation records the package version and a
   `skill://<name>@1` ref; `SkillTrust` distinguishes builtin/installed
   packages (an install-approval flow is not built yet).
6. **Multi-domain switching.** One run may activate multiple skills; the
   envelope carries all activated instructions so a task can move from
   research to writing mid-run (tested).

## Consequences

- Follow-up issues: async Wiki compilation; third-party Skill install approval;
  per-tenant ACL filtering of `wiki`/`search` reads (M7); Wiki token/cost
  accounting beyond write amplification; real-model end-to-end acceptance.
- Mandatory debug phase applies to every increment; the v0.1 M4/M5 debug report
  is in `docs/debug-reports/`.
