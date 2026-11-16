# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T14:36:33.058176+00:00
- tasks per suite per repetition: 2
- repetitions: 1
- base seed: 7
- config: e2m-deepseek-v4-pro-0813
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 2 | 0.50 [0.09,0.91] | 0.50 [0.09,0.91] | 0.00 | 91739.05 | 91739.05 | 65053.69 ± 52303.30 | 7390 | 3951 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 2 | 0.50 [0.09,0.91] | 0.50 [0.09,0.91] | 0.00 | 134795.59 | 134795.59 | 89167.20 ± 89431.65 | 10682 | 5216 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 2 | 1.00 [0.34,1.00] | 0.00 [0.00,0.66] | 1.00 | 25321.18 | 25321.18 | 24585.76 ± 1441.43 | 3340 | 1447 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 90792.78 | 90792.78 | 74054.45 ± 32807.13 | 5324 | 4468 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 50944.39 | 50944.39 | 41049.09 ± 19394.80 | 9782 | 2593 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 65298.66 | 65298.66 | 60041.36 ± 10304.31 | 11182 | 3669 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 2 | 1.00 [0.34,1.00] | 0.00 [0.00,0.66] | 1.00 | 108640.39 | 108640.39 | 89134.07 ± 38232.38 | 9854 | 5372 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 724209.41 | 724209.41 | 420466.08 ± 595336.93 | 23982 | 22255 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | no_verification | 6.0 | 5.5 | 0 | 0 | 0 | 0.00 |
| dataset | none | 8.5 | 8.0 | 1 | 0 | 0 | 0.00 |
| edge-case | no_verification | 3.0 | 2.0 | 0 | 0 | 0 | 0.00 |
| edge-case | none | 4.5 | 2.5 | 0 | 0 | 0 | 1.00 |
| long-horizon | no_verification | 5.5 | 4.5 | 0 | 0 | 0 | 0.00 |
| long-horizon | none | 6.0 | 5.0 | 0 | 0 | 0 | 0.00 |
| tool-use | no_verification | 7.0 | 6.0 | 0 | 0 | 0 | 0.00 |
| tool-use | none | 15.0 | 13.0 | 0 | 2 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2m-deepseek-v4-pro-0813.json
