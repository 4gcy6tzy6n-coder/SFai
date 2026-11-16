# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T10:14:18.425097+00:00
- tasks per suite per repetition: 3
- repetitions: 2
- base seed: 0
- config: e1-deepseek-baseline
- provider: deepseek

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 6 | 0.67 [0.30,0.90] | 0.67 [0.30,0.90] | 0.00 | 7469.36 | 23107.47 | 9530.75 ± 5668.62 | 8423 | 1244 | 0.003643 ± 0.002195 | 0.005464 |
| dataset | none | 6 | 0.50 [0.19,0.81] | 0.50 [0.19,0.81] | 0.00 | 8100.09 | 30702.10 | 13308.62 ± 9272.96 | 12096 | 1705 | 0.005142 ± 0.003743 | 0.010283 |
| edge-case | no_verification | 6 | 0.00 [0.00,0.39] | 0.00 [0.00,0.39] | 0.00 | 5548.67 | 7857.60 | 5138.38 ± 1628.20 | 3658 | 697 | 0.001754 ± 0.000590 | 0.000000 |
| edge-case | none | 6 | 0.00 [0.00,0.39] | 0.00 [0.00,0.39] | 0.00 | 6948.08 | 8662.36 | 6068.26 ± 1621.67 | 4379 | 831 | 0.002096 ± 0.000575 | 0.000000 |
| long-horizon | no_verification | 6 | 1.00 [0.61,1.00] | 0.50 [0.19,0.81] | 0.50 | 9036.02 | 14660.64 | 9656.04 ± 2154.77 | 11450 | 1582 | 0.004831 ± 0.001101 | 0.004831 |
| long-horizon | none | 6 | 1.00 [0.61,1.00] | 1.00 [0.61,1.00] | 0.00 | 17440.72 | 23481.68 | 17156.05 ± 4315.48 | 19910 | 2550 | 0.008181 ± 0.001945 | 0.008181 |
| tool-use | no_verification | 6 | 0.83 [0.44,0.97] | 0.67 [0.30,0.90] | 0.17 | 19983.82 | 24574.34 | 18544.67 ± 4084.04 | 15843 | 2791 | 0.007348 ± 0.001733 | 0.008817 |
| tool-use | none | 6 | 0.83 [0.44,0.97] | 1.00 [0.61,1.00] | 0.00 | 27623.44 | 41223.62 | 26735.30 ± 8735.29 | 20936 | 4122 | 0.010187 ± 0.003456 | 0.012224 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e1-deepseek-baseline.json
