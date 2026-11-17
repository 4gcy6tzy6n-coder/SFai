[bench] public-gsm8k/none/0 gsm8k-0016 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-gsm8k/no_verification/0 gsm8k-0000 FAILED: model error: provider response is not JSON: Failed to read JSON: Connection reset by peer (os error 54)
[bench] public-gsm8k/no_verification/0 gsm8k-0010 FAILED: model error: provider response is not JSON: Failed to read JSON: Connection reset by peer (os error 54)
[bench] public-gsm8k/no_verification/0 gsm8k-0013 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-gsm8k/no_verification/0 gsm8k-0019 FAILED: model error: provider response is not JSON: Failed to read JSON: Connection reset by peer (os error 54)
[bench] public-humaneval/none/0 humaneval-1 FAILED: model error: provider response is not JSON: Failed to read JSON: Connection reset by peer (os error 54)
[bench] public-humaneval/none/0 humaneval-10 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-humaneval/none/0 humaneval-5 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-humaneval/none/0 humaneval-6 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-humaneval/none/0 humaneval-7 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-humaneval/none/0 humaneval-8 FAILED: model error: provider response is not JSON: Failed to read JSON: Connection reset by peer (os error 54)
[bench] public-humaneval/none/0 humaneval-9 FAILED: model error: provider request failed: https://api.deepseek.com/v1/chat/completions: Network Error: Network Error: Error encountered in the status line: timed out reading response
[bench] public-humaneval/no_verification/0 humaneval-0 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-humaneval/no_verification/0 humaneval-1 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-humaneval/no_verification/0 humaneval-10 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-humaneval/no_verification/0 humaneval-12 FAILED: model error: provider request failed: https://api.deepseek.com/v1/chat/completions: Network Error: Network Error: Error encountered in the status line: timed out reading response
[bench] public-humaneval/no_verification/0 humaneval-13 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-humaneval/no_verification/0 humaneval-15 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-humaneval/no_verification/0 humaneval-16 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-humaneval/no_verification/0 humaneval-17 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-humaneval/no_verification/0 humaneval-18 FAILED: model error: provider response is not JSON: Failed to read JSON: Connection reset by peer (os error 54)
[bench] public-humaneval/no_verification/0 humaneval-2 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-humaneval/no_verification/0 humaneval-3 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-humaneval/no_verification/0 humaneval-4 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-humaneval/no_verification/0 humaneval-5 FAILED: model error: provider request failed: https://api.deepseek.com/v1/chat/completions: Network Error: Network Error: Error encountered in the status line: Connection reset by peer (os error 54)
[bench] public-humaneval/no_verification/0 humaneval-7 FAILED: model error: provider response is not JSON: timed out reading response
[bench] public-humaneval/no_verification/0 humaneval-9 FAILED: model error: provider request failed: https://api.deepseek.com/v1/chat/completions: Network Error: Network Error: Error encountered in the status line: timed out reading response
# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-17T00:35:22.049131+00:00
- tasks per suite per repetition: 20
- repetitions: 1
- base seed: 0
- config: e9-public-deepseek
- provider: deepseek

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| public-gsm8k | no_verification | 20 | 0.80 [0.58,0.92] | 0.70 [0.48,0.85] | 0.15 | 4680.18 | 140356.79 | 26957.60 ± 22943.54 | 3806 | 571 | 0.001656 ± 0.000942 | 0.002070 |
| public-gsm8k | none | 20 | 0.95 [0.76,0.99] | 0.95 [0.76,0.99] | 0.00 | 4761.01 | 28426.88 | 17000.28 ± 17146.08 | 8027 | 1134 | 0.003414 ± 0.001591 | 0.003594 |
| public-humaneval | no_verification | 20 | 0.15 [0.05,0.36] | 0.00 [0.00,0.16] | 0.15 | 184694.61 | 195133.94 | 133519.00 ± 34701.58 | 4719 | 733 | 0.002080 ± 0.002247 | 0.013870 |
| public-humaneval | none | 20 | 0.00 [0.00,0.16] | 0.00 [0.00,0.16] | 0.00 | 40324.07 | 220664.54 | 78474.36 ± 32878.59 | 31331 | 4360 | 0.013255 ± 0.004536 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| public-gsm8k | no_verification | 3.2 | 2.3 | 0 | 4 | 3 | 0.00 |
| public-gsm8k | none | 6.5 | 5.0 | 0 | 40 | 7 | 0.00 |
| public-humaneval | no_verification | 1.8 | 2.6 | 2 | 1 | 1 | 0.00 |
| public-humaneval | none | 5.7 | 14.8 | 7 | 8 | 15 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e9-public-deepseek.json
