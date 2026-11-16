# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T14:11:55.394788+00:00
- tasks per suite per repetition: 2
- repetitions: 1
- base seed: 7
- config: e2m-qwen3-8-flash
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 33929.99 | 33929.99 | 30690.89 ± 6348.63 | 6344 | 1763 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 43647.04 | 43647.04 | 34917.65 ± 17109.60 | 8248 | 2322 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 20509.66 | 20509.66 | 19396.19 ± 2182.39 | 3715 | 1310 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 64870.90 | 64870.90 | 53168.24 ± 22937.23 | 8142 | 3272 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 2 | 1.00 [0.34,1.00] | 0.50 [0.09,0.91] | 0.50 | 38826.70 | 38826.70 | 36998.06 ± 3584.15 | 12584 | 2440 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 53114.36 | 53114.36 | 52388.60 ± 1422.49 | 17846 | 3522 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 2 | 1.00 [0.34,1.00] | 0.50 [0.09,0.91] | 0.50 | 235258.83 | 235258.83 | 160033.42 ± 147441.81 | 20622 | 8888 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 154324.46 | 154324.46 | 116548.81 ± 74040.27 | 21012 | 6944 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | no_verification | 5.5 | 4.5 | 0 | 0 | 0 | 0.00 |
| dataset | none | 7.0 | 6.0 | 0 | 0 | 0 | 0.00 |
| edge-case | no_verification | 3.5 | 2.5 | 0 | 0 | 0 | 1.00 |
| edge-case | none | 7.0 | 5.0 | 0 | 0 | 0 | 1.00 |
| long-horizon | no_verification | 7.0 | 6.0 | 0 | 0 | 0 | 0.00 |
| long-horizon | none | 9.5 | 8.5 | 0 | 0 | 0 | 0.00 |
| tool-use | no_verification | 13.5 | 12.5 | 0 | 0 | 0 | 0.00 |
| tool-use | none | 14.0 | 13.0 | 0 | 3 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2m-qwen3-8-flash.json
