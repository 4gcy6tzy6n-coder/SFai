# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-17T06:27:11.066044+00:00
- tasks per suite per repetition: 20
- repetitions: 1
- base seed: 0
- config: e9-public-humaneval-fixed
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| public-humaneval | no_verification | 20 | 0.95 [0.76,0.99] | 1.00 [0.84,1.00] | 0.00 | 27611.16 | 85130.91 | 37604.08 ± 16501.43 | 18340 | 4965 | 0.000000 ± 0.000000 | 0.000000 |
| public-humaneval | none | 20 | 1.00 [0.84,1.00] | 1.00 [0.84,1.00] | 0.00 | 18268.92 | 56952.17 | 25421.41 ± 7569.88 | 8246 | 2552 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| public-humaneval | no_verification | 8.7 | 9.5 | 5 | 12 | 9 | 0.00 |
| public-humaneval | none | 5.7 | 4.5 | 1 | 1 | 4 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e9-public-humaneval-fixed.json
