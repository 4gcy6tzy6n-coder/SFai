# ADR-021: Core mechanism naming — "Decide-Act"

**Date:** 2026-11-16
**Status:** Accepted (project-owner directive)

## Decision

The harness's core router — the model-native self-routing loop — is named
**Decide-Act**. Each iteration is a *decide* step (the model emits exactly one
action intent: answer / discover / retrieve / activate / execute / verify /
ask_user / pause / finish) followed by an *act* step (the runtime validates and
executes it through the policy-aware broker, with idempotency, receipts, loop
guards and routing telemetry).

## Rationale

- The name captures the actual mechanism (model decides, runtime acts) rather
  than a generic "agent loop"; it is the project's differentiator.
- The three-pillar story: **Decide-Act** (routing) + **sparse context
  activation** (bounded index-native context) + **verification gate**
  (runtime-enforced acceptance).
- Code identity: `core/decision_kernel.rs` exposes `DecideActKernel` as the
  canonical alias of `DecisionKernel` (module path kept for stability).

## Consequences

- README / architecture / research-plan use the name; papers and talks should
  lead with "Decide-Act" when describing the router.
- E8 (self-routing vs forced pipeline) is the direct ablation of Decide-Act.
