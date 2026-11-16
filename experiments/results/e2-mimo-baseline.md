[bench] edge-case/none/1 edge-02 FAILED: model error: provider response is not JSON: timed out reading response
# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T11:08:58.114641+00:00
- tasks per suite per repetition: 3
- repetitions: 2
- base seed: 0
- config: e2-mimo-baseline
- provider: mimo

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 6 | 1.00 [0.61,1.00] | 1.00 [0.61,1.00] | 0.00 | 50771.63 | 62423.37 | 44444.69 ± 13327.75 | 5950 | 2466 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 6 | 1.00 [0.61,1.00] | 1.00 [0.61,1.00] | 0.00 | 55116.67 | 80325.25 | 53613.20 ± 16107.99 | 5398 | 2952 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 6 | 0.67 [0.30,0.90] | 0.00 [0.00,0.39] | 0.67 | 37395.39 | 46707.74 | 33320.32 ± 8413.31 | 2979 | 1644 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 6 | 0.00 [0.00,0.39] | 0.00 [0.00,0.39] | 0.00 | 79611.29 | 228646.02 | 81406.55 ± 61519.75 | 4155 | 2106 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 6 | 1.00 [0.61,1.00] | 1.00 [0.61,1.00] | 0.00 | 43201.35 | 58268.68 | 40932.42 ± 10969.40 | 8989 | 2068 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 6 | 1.00 [0.61,1.00] | 1.00 [0.61,1.00] | 0.00 | 39496.29 | 197211.57 | 67922.50 ± 52935.41 | 12654 | 3718 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 6 | 1.00 [0.61,1.00] | 1.00 [0.61,1.00] | 0.00 | 52907.55 | 68186.74 | 54243.45 ± 8969.15 | 9320 | 2851 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 6 | 1.00 [0.61,1.00] | 1.00 [0.61,1.00] | 0.00 | 65487.08 | 85847.40 | 68747.08 ± 10896.78 | 10037 | 3761 | 0.000000 ± 0.000000 | 0.000000 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2-mimo-baseline.json
