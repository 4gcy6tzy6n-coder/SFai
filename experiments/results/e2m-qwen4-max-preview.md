# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T15:06:17.765334+00:00
- tasks per suite per repetition: 2
- repetitions: 1
- base seed: 7
- config: e2m-qwen4-max-preview
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 56163.16 | 56163.16 | 43800.35 ± 24231.10 | 5213 | 1986 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 45413.41 | 45413.41 | 40795.83 ± 9050.46 | 5182 | 1878 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 2 | 1.00 [0.34,1.00] | 0.00 [0.00,0.66] | 1.00 | 32044.14 | 32044.14 | 29243.41 ± 5489.44 | 3667 | 1272 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 50741.87 | 50741.87 | 42331.91 ± 16483.52 | 4838 | 1886 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 54472.10 | 54472.10 | 48578.38 ± 11551.69 | 10328 | 1932 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 51464.69 | 51464.69 | 50768.23 ± 1365.04 | 11678 | 2122 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 68948.91 | 68948.91 | 67138.27 ± 3548.86 | 9952 | 3258 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 88143.44 | 88143.44 | 74014.02 ± 27693.67 | 10900 | 3643 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | no_verification | 5.0 | 4.0 | 0 | 0 | 0 | 0.00 |
| dataset | none | 5.0 | 4.0 | 0 | 0 | 0 | 0.00 |
| edge-case | no_verification | 4.0 | 3.0 | 0 | 0 | 0 | 0.00 |
| edge-case | none | 5.0 | 3.0 | 0 | 0 | 0 | 1.00 |
| long-horizon | no_verification | 6.0 | 5.0 | 0 | 0 | 0 | 0.00 |
| long-horizon | none | 6.5 | 5.5 | 0 | 0 | 0 | 0.00 |
| tool-use | no_verification | 8.0 | 7.0 | 0 | 0 | 0 | 0.00 |
| tool-use | none | 8.5 | 7.0 | 0 | 0 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2m-qwen4-max-preview.json
