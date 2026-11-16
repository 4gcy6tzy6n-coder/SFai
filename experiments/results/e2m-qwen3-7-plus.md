# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T13:27:55.561571+00:00
- tasks per suite per repetition: 2
- repetitions: 1
- base seed: 7
- config: e2m-qwen3-7-plus
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 2 | 0.50 [0.09,0.91] | 0.50 [0.09,0.91] | 0.00 | 43666.50 | 43666.50 | 39742.12 ± 7691.79 | 4250 | 2048 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 2 | 0.50 [0.09,0.91] | 0.50 [0.09,0.91] | 0.00 | 64288.98 | 64288.98 | 48380.77 ± 31180.09 | 6036 | 2290 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 2 | 1.00 [0.34,1.00] | 0.00 [0.00,0.66] | 1.00 | 38244.34 | 38244.34 | 34349.88 ± 7633.13 | 3523 | 1923 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 76590.51 | 76590.51 | 68618.45 ± 15625.24 | 5501 | 3780 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 2 | 1.00 [0.34,1.00] | 0.50 [0.09,0.91] | 0.50 | 53777.77 | 53777.77 | 43029.69 ± 21066.24 | 8162 | 2370 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 350561.20 | 350561.20 | 197032.96 ± 300915.34 | 25668 | 10058 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 2 | 1.00 [0.34,1.00] | 0.00 [0.00,0.66] | 1.00 | 66831.50 | 66831.50 | 62801.97 ± 7897.88 | 7978 | 3490 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 251775.48 | 251775.48 | 222363.41 ± 57647.65 | 26576 | 12528 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | no_verification | 4.0 | 3.5 | 0 | 0 | 0 | 0.00 |
| dataset | none | 5.5 | 5.0 | 0 | 0 | 0 | 0.00 |
| edge-case | no_verification | 3.5 | 2.5 | 0 | 0 | 0 | 0.00 |
| edge-case | none | 5.0 | 2.0 | 0 | 0 | 0 | 0.00 |
| long-horizon | no_verification | 4.5 | 3.5 | 0 | 0 | 0 | 0.00 |
| long-horizon | none | 13.5 | 12.0 | 0 | 0 | 1 | 0.00 |
| tool-use | no_verification | 6.5 | 5.5 | 0 | 0 | 0 | 0.00 |
| tool-use | none | 18.5 | 16.5 | 0 | 4 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2m-qwen3-7-plus.json
