# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-17T04:03:35.881105+00:00
- tasks per suite per repetition: 20
- repetitions: 1
- base seed: 0
- config: e9-public-sheets-bailian
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| public-sheets | no_verification | 20 | 0.15 [0.05,0.36] | 0.15 [0.05,0.36] | 0.00 | 15416.35 | 52343.48 | 22900.82 ± 11950.11 | 7481 | 3806 | 0.000000 ± 0.000000 | 0.000000 |
| public-sheets | none | 20 | 0.15 [0.05,0.36] | 0.15 [0.05,0.36] | 0.00 | 15632.62 | 29693.29 | 18531.12 ± 5334.15 | 7313 | 2964 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| public-sheets | no_verification | 5.7 | 5.5 | 3 | 1 | 0 | 0.00 |
| public-sheets | none | 5.7 | 5.5 | 0 | 5 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e9-public-sheets-bailian.json
