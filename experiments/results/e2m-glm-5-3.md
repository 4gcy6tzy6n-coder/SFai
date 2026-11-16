# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T13:55:06.910028+00:00
- tasks per suite per repetition: 2
- repetitions: 1
- base seed: 7
- config: e2m-glm-5-3
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 104367.00 | 104367.00 | 93343.20 ± 21606.65 | 5868 | 5655 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 216434.36 | 216434.36 | 165654.76 ± 99528.02 | 5874 | 10084 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 2 | 1.00 [0.34,1.00] | 0.00 [0.00,0.66] | 1.00 | 274810.20 | 274810.20 | 180246.63 ± 185344.59 | 3500 | 11388 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 347233.72 | 347233.72 | 317495.76 ± 58286.41 | 5677 | 21440 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 359153.79 | 359153.79 | 229300.47 ± 254512.50 | 10019 | 16180 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 215861.04 | 215861.04 | 205711.23 ± 19893.63 | 10962 | 14426 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 2 | 1.00 [0.34,1.00] | 0.50 [0.09,0.91] | 0.50 | 179855.31 | 179855.31 | 175928.82 ± 7695.91 | 8392 | 10781 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 192646.06 | 192646.06 | 167728.08 ± 48839.24 | 9380 | 10626 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | no_verification | 6.0 | 5.0 | 0 | 0 | 0 | 0.00 |
| dataset | none | 6.0 | 5.0 | 1 | 0 | 0 | 0.00 |
| edge-case | no_verification | 4.0 | 3.0 | 0 | 0 | 0 | 0.00 |
| edge-case | none | 6.0 | 3.5 | 0 | 0 | 0 | 1.00 |
| long-horizon | no_verification | 6.5 | 5.5 | 0 | 0 | 0 | 0.00 |
| long-horizon | none | 7.0 | 5.0 | 0 | 0 | 0 | 0.00 |
| tool-use | no_verification | 7.5 | 6.5 | 0 | 0 | 0 | 0.00 |
| tool-use | none | 8.0 | 7.0 | 0 | 0 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2m-glm-5-3.json
