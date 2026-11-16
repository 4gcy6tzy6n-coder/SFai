# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T14:53:04.229975+00:00
- tasks per suite per repetition: 2
- repetitions: 1
- base seed: 7
- config: e2m-deepseek-v4-1-flash-1017
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 11956.04 | 11956.04 | 9581.20 ± 4654.70 | 4576 | 1420 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 16775.41 | 16775.41 | 15456.32 ± 2585.42 | 5900 | 2316 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 17378.64 | 17378.64 | 16783.68 ± 1166.11 | 6056 | 2384 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 2 | 0.00 [0.00,0.66] | 0.50 [0.09,0.91] | 0.00 | 154103.73 | 154103.73 | 84889.05 ± 135660.77 | 20446 | 11866 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 27469.88 | 27469.88 | 26937.85 ± 1042.78 | 15416 | 3774 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 45665.20 | 45665.20 | 40536.87 ± 10051.54 | 20128 | 5806 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 30123.21 | 30123.21 | 29872.89 ± 490.62 | 12330 | 4060 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 76738.25 | 76738.25 | 52681.67 ± 47150.88 | 20342 | 7150 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | no_verification | 4.0 | 3.0 | 0 | 0 | 0 | 0.00 |
| dataset | none | 5.0 | 4.0 | 0 | 0 | 0 | 0.00 |
| edge-case | no_verification | 5.5 | 5.0 | 0 | 0 | 0 | 0.50 |
| edge-case | none | 13.5 | 11.5 | 2 | 0 | 0 | 1.00 |
| long-horizon | no_verification | 8.5 | 7.5 | 0 | 0 | 0 | 0.00 |
| long-horizon | none | 11.0 | 9.5 | 0 | 1 | 0 | 0.00 |
| tool-use | no_verification | 9.0 | 8.0 | 0 | 0 | 0 | 0.00 |
| tool-use | none | 13.5 | 12.5 | 0 | 3 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2m-deepseek-v4-1-flash-1017.json
