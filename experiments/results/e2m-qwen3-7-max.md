# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T13:04:02.768293+00:00
- tasks per suite per repetition: 2
- repetitions: 1
- base seed: 7
- config: e2m-qwen3-7-max
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 42301.06 | 42301.06 | 36253.17 ± 11853.87 | 6213 | 2952 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 26581.43 | 26581.43 | 25620.75 ± 1882.93 | 4284 | 2106 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 2 | 1.00 [0.34,1.00] | 0.00 [0.00,0.66] | 1.00 | 22135.89 | 22135.89 | 19851.03 ± 4478.33 | 2973 | 1422 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 100248.33 | 100248.33 | 86527.30 ± 26893.21 | 8145 | 6769 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 2 | 1.00 [0.34,1.00] | 0.00 [0.00,0.66] | 1.00 | 24452.12 | 24452.12 | 24106.11 ± 678.18 | 7352 | 2143 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 66953.21 | 66953.21 | 58087.44 ± 17376.91 | 14748 | 5080 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 2 | 1.00 [0.34,1.00] | 0.00 [0.00,0.66] | 1.00 | 35032.77 | 35032.77 | 33493.22 ± 3017.52 | 3503 | 2762 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 104980.42 | 104980.42 | 88869.48 ± 31577.46 | 13502 | 6830 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | no_verification | 5.5 | 4.5 | 1 | 0 | 0 | 0.00 |
| dataset | none | 4.0 | 3.0 | 1 | 0 | 0 | 0.00 |
| edge-case | no_verification | 3.0 | 2.0 | 0 | 0 | 0 | 0.00 |
| edge-case | none | 7.0 | 4.0 | 0 | 0 | 0 | 0.00 |
| long-horizon | no_verification | 4.0 | 3.0 | 0 | 0 | 0 | 0.00 |
| long-horizon | none | 7.5 | 5.5 | 0 | 1 | 0 | 0.00 |
| tool-use | no_verification | 3.0 | 2.0 | 0 | 0 | 0 | 0.00 |
| tool-use | none | 9.5 | 7.5 | 0 | 0 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2m-qwen3-7-max.json
