# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T13:50:13.173361+00:00
- tasks per suite per repetition: 2
- repetitions: 1
- base seed: 7
- config: e2m-qwen3-6-flash
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 2 | 0.50 [0.09,0.91] | 0.50 [0.09,0.91] | 0.00 | 31586.37 | 31586.37 | 22914.27 ± 16997.31 | 6167 | 4154 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 2 | 0.50 [0.09,0.91] | 0.50 [0.09,0.91] | 0.00 | 23981.75 | 23981.75 | 17832.17 ± 12053.18 | 4842 | 3134 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 29541.16 | 29541.16 | 23099.76 ± 12625.14 | 3022 | 3810 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 212678.97 | 212678.97 | 112733.10 ± 195893.90 | 14714 | 18068 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 2 | 1.00 [0.34,1.00] | 0.00 [0.00,0.66] | 1.00 | 38624.75 | 38624.75 | 29727.23 ± 17439.13 | 10120 | 5076 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 206741.30 | 206741.30 | 166748.25 ± 78386.39 | 43126 | 30413 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 2 | 1.00 [0.34,1.00] | 0.50 [0.09,0.91] | 0.50 | 81785.22 | 81785.22 | 65878.22 ± 31177.72 | 13442 | 11114 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 230700.73 | 230700.73 | 229786.09 ± 1792.69 | 34060 | 38710 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | no_verification | 5.5 | 5.0 | 0 | 0 | 0 | 0.00 |
| dataset | none | 4.5 | 4.0 | 0 | 0 | 0 | 0.00 |
| edge-case | no_verification | 3.0 | 3.0 | 0 | 0 | 0 | 0.00 |
| edge-case | none | 11.5 | 10.0 | 0 | 0 | 0 | 0.50 |
| long-horizon | no_verification | 5.5 | 4.5 | 0 | 0 | 0 | 0.00 |
| long-horizon | none | 22.0 | 20.0 | 0 | 4 | 0 | 0.00 |
| tool-use | no_verification | 10.0 | 9.0 | 0 | 0 | 0 | 0.00 |
| tool-use | none | 22.0 | 20.0 | 0 | 3 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2m-qwen3-6-flash.json
