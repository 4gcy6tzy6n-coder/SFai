# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T15:16:38.337641+00:00
- tasks per suite per repetition: 2
- repetitions: 1
- base seed: 7
- config: e2m-qwen4-max
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 69720.59 | 69720.59 | 56459.20 ± 25992.32 | 5813 | 2588 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 47897.88 | 47897.88 | 43973.82 ± 7691.17 | 5190 | 2056 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 2 | 0.50 [0.09,0.91] | 0.00 [0.00,0.66] | 0.50 | 29732.32 | 29732.32 | 27306.43 ± 4754.75 | 2643 | 1141 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 55819.07 | 55819.07 | 41326.14 ± 28406.13 | 3738 | 1742 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 53534.10 | 53534.10 | 45840.93 ± 15078.61 | 11605 | 2002 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 43810.24 | 43810.24 | 43717.22 ± 182.30 | 11228 | 1957 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 2 | 1.00 [0.34,1.00] | 0.50 [0.09,0.91] | 0.50 | 66157.26 | 66157.26 | 53987.50 ± 23852.73 | 8634 | 2444 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 62122.72 | 62122.72 | 61012.50 ± 2176.03 | 10738 | 2810 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | no_verification | 5.5 | 4.5 | 0 | 0 | 0 | 0.00 |
| dataset | none | 5.0 | 4.0 | 0 | 0 | 0 | 0.00 |
| edge-case | no_verification | 3.0 | 2.0 | 0 | 0 | 0 | 0.50 |
| edge-case | none | 4.0 | 2.5 | 0 | 0 | 0 | 1.00 |
| long-horizon | no_verification | 6.5 | 5.5 | 0 | 0 | 0 | 0.00 |
| long-horizon | none | 6.5 | 5.5 | 0 | 0 | 0 | 0.00 |
| tool-use | no_verification | 7.0 | 6.0 | 0 | 0 | 0 | 0.00 |
| tool-use | none | 8.5 | 7.5 | 0 | 0 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2m-qwen4-max.json
