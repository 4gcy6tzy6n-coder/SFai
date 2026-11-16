# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T14:07:45.761604+00:00
- tasks per suite per repetition: 2
- repetitions: 1
- base seed: 7
- config: e2m-deepseek-v4-flash-0731
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 2 | 0.50 [0.09,0.91] | 0.50 [0.09,0.91] | 0.00 | 68824.69 | 68824.69 | 48663.43 ± 39516.07 | 11867 | 4738 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 67577.87 | 67577.87 | 65124.88 ± 4807.86 | 17450 | 6412 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 2 | 1.00 [0.34,1.00] | 0.00 [0.00,0.66] | 1.00 | 20964.18 | 20964.18 | 18844.77 ± 4154.04 | 3340 | 1812 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 38798.81 | 38798.81 | 30403.50 ± 16454.81 | 3975 | 2892 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 2 | 1.00 [0.34,1.00] | 0.50 [0.09,0.91] | 0.50 | 79864.06 | 79864.06 | 77293.98 ± 5037.35 | 17671 | 8002 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 89992.49 | 89992.49 | 83021.82 ± 13662.53 | 20700 | 8512 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 163079.70 | 163079.70 | 127869.04 ± 69012.89 | 22511 | 12742 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 109444.33 | 109444.33 | 74974.32 ± 67561.21 | 14597 | 7927 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | no_verification | 9.5 | 9.0 | 1 | 1 | 0 | 0.00 |
| dataset | none | 13.5 | 12.5 | 2 | 0 | 0 | 0.00 |
| edge-case | no_verification | 3.0 | 2.0 | 0 | 0 | 0 | 0.00 |
| edge-case | none | 3.5 | 2.0 | 0 | 0 | 0 | 1.00 |
| long-horizon | no_verification | 9.5 | 8.5 | 0 | 0 | 0 | 0.00 |
| long-horizon | none | 11.0 | 10.0 | 0 | 0 | 0 | 0.00 |
| tool-use | no_verification | 14.5 | 13.5 | 1 | 0 | 0 | 0.00 |
| tool-use | none | 10.0 | 9.0 | 1 | 0 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2m-deepseek-v4-flash-0731.json
