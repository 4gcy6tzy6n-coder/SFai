# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T14:43:50.556093+00:00
- tasks per suite per repetition: 2
- repetitions: 1
- base seed: 7
- config: e2m-deepseek-v4-1-flash
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 15825.14 | 15825.14 | 12738.35 ± 6050.11 | 5720 | 1795 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 22737.07 | 22737.07 | 18273.61 ± 8748.38 | 7057 | 2480 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 2 | 0.50 [0.09,0.91] | 0.00 [0.00,0.66] | 0.50 | 11935.20 | 11935.20 | 11775.29 ± 313.42 | 3162 | 1628 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 15518.25 | 15518.25 | 14168.10 ± 2646.29 | 3740 | 2010 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 27771.23 | 27771.23 | 24886.70 ± 5653.66 | 12234 | 3444 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 50568.36 | 50568.36 | 37972.57 ± 24687.75 | 16049 | 5426 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 32199.90 | 32199.90 | 30047.69 ± 4218.32 | 10954 | 4286 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 2 | 0.50 [0.09,0.91] | 1.00 [0.34,1.00] | 0.00 | 96936.73 | 96936.73 | 68396.27 ± 55939.29 | 24527 | 9588 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | no_verification | 5.0 | 4.0 | 1 | 0 | 0 | 0.00 |
| dataset | none | 6.0 | 5.0 | 2 | 0 | 0 | 0.00 |
| edge-case | no_verification | 3.0 | 2.0 | 0 | 0 | 0 | 0.50 |
| edge-case | none | 3.5 | 2.5 | 0 | 0 | 0 | 1.00 |
| long-horizon | no_verification | 7.0 | 6.0 | 0 | 0 | 0 | 0.00 |
| long-horizon | none | 9.0 | 8.0 | 1 | 0 | 0 | 0.00 |
| tool-use | no_verification | 8.0 | 7.0 | 0 | 0 | 0 | 0.00 |
| tool-use | none | 5.5 | 15.5 | 0 | 0 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2m-deepseek-v4-1-flash.json
