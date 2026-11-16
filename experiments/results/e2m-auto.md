# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T15:23:19.088482+00:00
- tasks per suite per repetition: 2
- repetitions: 1
- base seed: 7
- config: e2m-auto
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 27368.72 | 27368.72 | 23113.68 ± 8339.88 | 5143 | 1550 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 21022.47 | 21022.47 | 20933.94 ± 173.51 | 4488 | 1370 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 51004.74 | 51004.74 | 34915.55 ± 31534.82 | 6482 | 2086 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 27655.14 | 27655.14 | 20465.31 ± 14092.05 | 3116 | 1181 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 50809.29 | 50809.29 | 48663.87 ± 4205.03 | 17692 | 3063 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 56929.70 | 56929.70 | 47536.24 ± 18411.19 | 18534 | 3015 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 2 | 1.00 [0.34,1.00] | 0.50 [0.09,0.91] | 0.50 | 118350.60 | 118350.60 | 116876.47 ± 2889.29 | 22896 | 7524 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 293296.76 | 293296.76 | 198076.04 ± 186632.62 | 23116 | 11610 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | no_verification | 4.5 | 3.5 | 0 | 0 | 0 | 0.00 |
| dataset | none | 4.0 | 3.0 | 0 | 0 | 0 | 0.00 |
| edge-case | no_verification | 6.0 | 5.5 | 0 | 1 | 0 | 0.50 |
| edge-case | none | 3.0 | 2.0 | 0 | 0 | 0 | 1.00 |
| long-horizon | no_verification | 9.5 | 8.5 | 0 | 0 | 0 | 0.00 |
| long-horizon | none | 9.5 | 7.5 | 1 | 0 | 0 | 0.00 |
| tool-use | no_verification | 15.5 | 14.5 | 2 | 3 | 0 | 0.00 |
| tool-use | none | 15.0 | 13.5 | 0 | 2 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2m-auto.json
