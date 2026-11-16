# ADR-020: Router-block hardening (loop guard + routing telemetry)

**Date:** 2026-11-16
**Status:** Accepted — the router is the project core; this records its runtime guards
**Context:** The project's core is the router block: the model self-routes to the
next action (plan §0/§4). Plan §4 requires *runtime* loop/repetitive-call
detection, and plan §13.4 defines routing-quality metrics — both were missing.

## Decisions

1. **Loop guard (runtime, not model-side).** The kernel tracks a rolling window
   of the last 8 *successful* action signatures (`hash(kind, payload)` for
   execute/retrieve/verify). When the same signature would run a 3rd
   consecutive time, it is refused with `LOOP_DETECTED` (status `denied`,
   machine-readable diagnostic) and `loop_guard_trips` is counted. The refusal
   is recorded in the trace and the decision audit stays exact.
2. **Error retries are NOT loops.** Only *successful* actions enter loop
   history; a failing action is retried under the existing consecutive-error
   bound (3 → `blocked`). The two protections cover different failure modes:
   useless repetition vs persistent tool failure.
3. **Routing telemetry (plan §13.4).** Per run the kernel collects
   `RoutingStats`: intent-kind histogram, tool calls, duplicate replays
   (idempotency absorptions), loop-guard trips, misroutes (`UNKNOWN_OPERATION`),
   ask_user count, skill activations, discover calls. It ships in
   `TaskRunResult.routing` and in the trace (`routing` entry).
4. **Benchmark routing metrics.** `BenchTaskResult` now records steps,
   misroutes, loop-guard trips, duplicate replays, ask_user; the report renders
   a **Routing quality** table per suite × ablation (mean steps/tool calls,
   misroutes, loops, dup replays, ask_user rate).
5. **Misroutes count, never execute.** An intent to an unregistered operation is
   a counted misroute with an auditable denial — discovery is explicit.

## Semantics notes

- A replayed (idempotent) intent returns `denied`? No — replays return the
  stored `ok` result with `replayed=true` and count as `duplicate_replays`,
  not tool calls.
- The guard triggers on the 3rd consecutive identical successful action;
  interleaving different actions resets the streak.

## Consequences

- Router quality is now measurable in every experiment; E8 (self-routing vs
  forced pipeline) can report route length, misroutes and guard trips.
- Remaining router work: uncertainty-triggered ask_user policy, capability
  eligibility filtering in `discover`, and rule-router baseline (plan §13.3 #9).
