# ADR-015: M6 verification hardening (v0.1)

**Date:** 2026-11-16
**Status:** Accepted for v0.1 — revisit at M7 (sandbox/ACL) and M8
**Context:** M6 (independent Verify & Debug) adds deterministic code/document
checks, risk-based mandatory validation, and bounded repair. This ADR records
the decisions taken.

## Decisions

1. **Extended deterministic criteria.** `VerificationEngine` supports, in
   addition to `artifact_exists`, `file_type`, `citations_resolve`,
   `hash_matches`, `source_count_at_least`:
   - `code_tests_pass:<command>` — runs `sh -c <command>` in the task
     workspace, 120s timeout, bounded output capture, verdict from exit code;
   - `json_valid:<path>` — artifact parses as JSON;
   - `no_placeholders:<path>` — rejects TODO/TBD/FIXME/Lorem ipsum/placeholder
     text in generated documents.
2. **Per-task workspace.** `file.write` materializes artifacts under
   `<storage>/workspaces/<task_id>/<path>` (traversal guarded) so code checks
   operate on real files; the verifier creates the directory if missing.
3. **Latest-artifact semantics.** Artifact `version` now increments per
   `(task, path)`; `citations_resolve` checks only the latest version of each
   path, so a repaired deliverable is not poisoned by older broken drafts.
4. **Verification events.** Each `verify` intent records
   `verification.recorded` events (criterion, verdict, target, diagnostics) in
   the ledger and in `TaskState.verifications`; the Wiki compiler turns them
   into `verification` nodes plus `verified_by` edges on pass.
5. **Risk-based mandatory validation.** A successful `external_write` /
   `high_impact` tool run must be followed by a `verification.recorded` event
   before the task may complete. Otherwise the kernel records a
   `MANDATORY_VERIFICATION_MISSING` open issue and lets the model repair
   (bounded); repeated failure ends in `blocked`, never `completed`.
6. **Bounded repair.** Acceptance failures and mandatory-validation failures
   are retried at most twice (three attempts), then the task becomes `blocked`
   with visible open issues. Tool errors are bounded at three consecutive
   attempts (existing M1 behavior).

## Deviations / limitations

- **No OS-level sandbox.** Code checks run as a local subprocess in the task
  workspace; there is no container/seccomp isolation yet (M7 item).
- **Process-group kill.** On timeout only the direct child (`sh`) is killed;
  grandchildren may survive. Follow-up: spawn a process group and kill it.
- **Large-output deadlock risk.** stdout/stderr are captured after exit; a
  command that fills the pipe buffer before exiting could block. Output is
  bounded after reading; follow-up: stream with size caps.
- **Price table is global.** Verification itself is counted (runs), but code
  test execution time is not separately priced.

## Consequences

- Follow-up issues: OS sandboxing, process-group timeouts, streaming output;
  ACL filtering for workspaces (M7); verification cost accounting.
- Mandatory debug phase applies; the M6 debug report lives in
  `docs/debug-reports/`.
