[bench] tool-use/none/0 tool-00 FAILED: model error: provider request failed: https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions: status code 400
[bench] tool-use/none/0 tool-01 FAILED: model error: provider request failed: https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions: status code 400
[bench] tool-use/no_verification/0 tool-00 FAILED: model error: provider request failed: https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions: status code 400
[bench] tool-use/no_verification/0 tool-01 FAILED: model error: provider request failed: https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions: status code 400
[bench] dataset/none/0 data-00 FAILED: model error: provider request failed: https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions: status code 400
[bench] dataset/none/0 data-01 FAILED: model error: provider request failed: https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions: status code 400
[bench] dataset/no_verification/0 data-00 FAILED: model error: provider request failed: https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions: status code 400
[bench] dataset/no_verification/0 data-01 FAILED: model error: provider request failed: https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions: status code 400
[bench] long-horizon/none/0 long-00 FAILED: model error: provider request failed: https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions: status code 400
[bench] long-horizon/none/0 long-01 FAILED: model error: provider request failed: https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions: status code 400
[bench] long-horizon/no_verification/0 long-00 FAILED: model error: provider request failed: https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions: status code 400
[bench] long-horizon/no_verification/0 long-01 FAILED: model error: provider request failed: https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions: status code 400
[bench] edge-case/none/0 edge-00 FAILED: model error: provider request failed: https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions: status code 400
[bench] edge-case/none/0 edge-01 FAILED: model error: provider request failed: https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions: status code 400
[bench] edge-case/no_verification/0 edge-00 FAILED: model error: provider request failed: https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions: status code 400
[bench] edge-case/no_verification/0 edge-01 FAILED: model error: provider request failed: https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1/chat/completions: status code 400
# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T15:23:21.151787+00:00
- tasks per suite per repetition: 2
- repetitions: 1
- base seed: 7
- config: e2m-decision-model-preview
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 106.75 | 106.75 | 106.49 ± 0.52 | 0 | 0 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 182.60 | 182.60 | 140.68 ± 82.17 | 0 | 0 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 96.27 | 96.27 | 94.48 ± 3.50 | 0 | 0 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 176.45 | 176.45 | 135.76 ± 79.74 | 0 | 0 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 96.82 | 96.82 | 95.05 ± 3.48 | 0 | 0 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 177.19 | 177.19 | 134.66 ± 83.34 | 0 | 0 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 100.83 | 100.83 | 98.28 ± 5.00 | 0 | 0 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 180.72 | 180.72 | 138.34 ± 83.07 | 0 | 0 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | no_verification | 0.0 | 0.0 | 0 | 0 | 0 | 0.00 |
| dataset | none | 0.0 | 0.0 | 0 | 0 | 0 | 0.00 |
| edge-case | no_verification | 0.0 | 0.0 | 0 | 0 | 0 | 0.00 |
| edge-case | none | 0.0 | 0.0 | 0 | 0 | 0 | 0.00 |
| long-horizon | no_verification | 0.0 | 0.0 | 0 | 0 | 0 | 0.00 |
| long-horizon | none | 0.0 | 0.0 | 0 | 0 | 0 | 0.00 |
| tool-use | no_verification | 0.0 | 0.0 | 0 | 0 | 0 | 0.00 |
| tool-use | none | 0.0 | 0.0 | 0 | 0 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2m-decision-model-preview.json
