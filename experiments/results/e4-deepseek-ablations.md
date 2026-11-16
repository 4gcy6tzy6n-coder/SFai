# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T10:24:29.876526+00:00
- tasks per suite per repetition: 2
- repetitions: 2
- base seed: 0
- config: e4-deepseek-ablations
- provider: deepseek

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | full_context | 4 | 0.75 [0.30,0.95] | 0.75 [0.30,0.95] | 0.00 | 14611.06 | 20809.61 | 12028.27 ± 6902.40 | 10732 | 1578 | 0.004633 ± 0.002528 | 0.006178 |
| dataset | no_pinned | 4 | 0.50 [0.15,0.85] | 0.50 [0.15,0.85] | 0.00 | 11514.75 | 16378.70 | 11970.75 ± 3188.27 | 10859 | 1541 | 0.004627 ± 0.001129 | 0.009254 |
| dataset | no_verification | 4 | 0.75 [0.30,0.95] | 0.75 [0.30,0.95] | 0.00 | 10134.78 | 17519.89 | 10652.39 ± 4962.21 | 9597 | 1388 | 0.004118 ± 0.002083 | 0.005491 |
| dataset | no_wiki | 4 | 0.75 [0.30,0.95] | 0.75 [0.30,0.95] | 0.00 | 19242.77 | 19412.00 | 16165.66 ± 4349.58 | 13980 | 2102 | 0.006086 ± 0.001599 | 0.008115 |
| dataset | none | 4 | 0.50 [0.15,0.85] | 0.50 [0.15,0.85] | 0.00 | 21362.10 | 27653.06 | 17699.31 ± 8217.92 | 15934 | 2339 | 0.006876 ± 0.003153 | 0.013751 |
| long-horizon | full_context | 4 | 0.75 [0.30,0.95] | 1.00 [0.51,1.00] | 0.00 | 32875.98 | 34767.83 | 25119.54 ± 9912.58 | 50207 | 3723 | 0.017651 ± 0.007817 | 0.023534 |
| long-horizon | no_pinned | 4 | 1.00 [0.51,1.00] | 1.00 [0.51,1.00] | 0.00 | 15259.72 | 33573.74 | 18403.71 ± 10000.89 | 21744 | 2797 | 0.008948 ± 0.004925 | 0.008948 |
| long-horizon | no_verification | 4 | 1.00 [0.51,1.00] | 0.25 [0.05,0.70] | 0.75 | 7541.51 | 7716.01 | 6459.75 ± 1326.00 | 7342 | 954 | 0.003032 ± 0.000574 | 0.003032 |
| long-horizon | no_wiki | 4 | 1.00 [0.51,1.00] | 1.00 [0.51,1.00] | 0.00 | 23300.27 | 25335.99 | 19271.98 ± 5820.23 | 22157 | 3015 | 0.009299 ± 0.002506 | 0.009299 |
| long-horizon | none | 4 | 1.00 [0.51,1.00] | 1.00 [0.51,1.00] | 0.00 | 15456.23 | 17373.10 | 14972.91 ± 1812.87 | 18038 | 2301 | 0.007402 ± 0.000723 | 0.007402 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e4-deepseek-ablations.json
