# ADR-012: Implementation language switches from Python to Rust

**Date:** 2026-11-16
**Status:** Accepted (user directive) — supersedes the Python baseline in `docs/implementation-plan.md` §2
**Decision makers:** project owner (direct instruction), recorded by dev session
**Resolution of open question:** option (b) chosen — the Python scaffold is
DELETED and the `system-for-future-ai/` repository is wiped clean (`.git` and
the GitHub remote preserved) for the Rust rewrite. This copy of the ADR lives
outside the repo so the decision survives the wipe.

## Context

The engineering development plan v0.5 (§2 Technology Baseline) picked
**Python 3.11+ with Pydantic v2 / SQLite / FastAPI** as an engineering
default, explicitly noting these were "engineering defaults, not a
restriction on the research hypothesis" and that "all critical contracts
remain language-agnostic".

During implementation of SF-001–SF-014 a Python scaffold was built under
`system-for-future-ai/` (contracts, decision kernel, policy broker, event
ledger, FTS5 context index, CLI, fixture suite).

On 2026-11-16 the project owner directed: **all of this project is written in
Rust, not Python.** Development was paused at that point.

## Decision

1. **The implementation language for sfai is Rust.** The Python baseline in
   the plan is superseded. On resumption, the plan §2 technology table must
   be revised to Rust equivalents (e.g. `serde`/`schemars` for contracts,
   `rusqlite` or `sqlx` for the event ledger, `axum` for the API, `clap` for
   the CLI, `cargo` + `cargo-nextest` for the test harness).
2. **The Python scaffold is frozen, not deleted** (pending owner decision).
   It remains valuable as an executable reference of the intended semantics:
   the action-intent envelope, policy risk tiers, idempotent tool dispatch,
   event-ledger replay, bounded context envelope, and the fixture suite's
   expected behaviors. It must not be developed further.
3. **Contracts stay language-agnostic** (plan §2 compatibility note): the
   JSON shapes defined in `src/sfai/contracts.py` are the portable spec;
   the Rust rewrite should implement the same schemas and pass the same
   fixture scenarios (direct answer / tool use / failed tool / approval /
   context retrieval) as acceptance tests.

## Consequences

- SF-001–SF-014 must be re-implemented in Rust; the issue tracker entries
  carry over unchanged because they are contract-level, not language-level.
- The mandatory debug phase (`docs/development-debug-process.md`) applies to
  the Rust implementation exactly as written.
- Milestone exit gates (M0–M8) are unchanged.
- ~~Open question for the owner: delete the Python scaffold or keep it under
  `reference/python/` as a semantics oracle.~~ **Resolved 2026-11-16: deleted
  (option b).** The expected behavior spec survives in
  `DEVELOPMENT_DEBUG_PROCESS.md` (fixture list, debug gates) and in the plan's
  acceptance suite (§12 scenarios A–L).

## Evidence

- User instruction, 2026-11-16: "我们的所有由rust编写而不是py" (all of ours is
  written in Rust, not Python), followed by "暂停" (pause).
- Plan v0.5 §2: "These choices are engineering defaults, not a restriction
  on the research hypothesis. All critical contracts remain language-agnostic."
