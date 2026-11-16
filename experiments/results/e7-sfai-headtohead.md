# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T12:12:45.297281+00:00
- tasks per suite per repetition: 3
- repetitions: 2
- base seed: 0
- config: e7-sfai-headtohead
- provider: deepseek

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | none | 6 | 0.83 [0.44,0.97] | 0.83 [0.44,0.97] | 0.00 | 7645.25 | 9780.48 | 7238.40 ± 1442.05 | 6599 | 996 | 0.002878 ± 0.000620 | 0.003453 |
| tool-use | none | 6 | 1.00 [0.61,1.00] | 1.00 [0.61,1.00] | 0.00 | 24243.41 | 46375.11 | 25267.49 ± 9064.76 | 20289 | 3863 | 0.009727 ± 0.003013 | 0.009727 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | none | 5.8 | 5.0 | 0 | 0 | 0 | 0.00 |
| tool-use | none | 14.3 | 13.2 | 4 | 3 | 1 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e7-sfai-headtohead.json
