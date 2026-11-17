# 数据冻结清单 v2 (DATA FREEZE)

**冻结时间:** 2026-11-17（v1 于 2026-11-16；v2 增补 E8/E7v2/E9 公开基准）· **bench:** v0.2.1
**状态:** ✅ 冻结完成 —— 论文所有数字以本清单为准；重跑产生新文件、不覆盖清单文件
**总数据表:** `docs/paper/data-summary.md`（写作唯一数据源）

## 全部结果文件（36 个）

| 文件 | sha256[:16] | 大小 |
|---|---|---|
| `experiments/results/e1-deepseek-baseline.json` | `6dabd6b7836d659a` | 29281 |
| `experiments/results/e2-mimo-baseline.json` | `7782d2ad3800f747` | 28590 |
| `experiments/results/e2m-auto.json` | `3b1b0b731c97d686` | 17511 |
| `experiments/results/e2m-decision-model-preview.json` | `fc8c34c2aa1e392b` | 19608 |
| `experiments/results/e2m-deepseek-v4-1-flash-1017.json` | `27495bee4693e61d` | 17493 |
| `experiments/results/e2m-deepseek-v4-1-flash.json` | `469e067c9ba548f5` | 17524 |
| `experiments/results/e2m-deepseek-v4-flash-0731.json` | `88bce7d624889938` | 17566 |
| `experiments/results/e2m-deepseek-v4-pro-0813.json` | `27a02aa906ac9e58` | 17526 |
| `experiments/results/e2m-deepseek-v4-pro.json` | `a9e19bb40e533e0b` | 17542 |
| `experiments/results/e2m-glm-5-2.json` | `34f10cc0d74cc848` | 17546 |
| `experiments/results/e2m-glm-5-3.json` | `b7bc37d3069db2ce` | 17533 |
| `experiments/results/e2m-qwen3-6-flash.json` | `4eeb007f487322fa` | 17512 |
| `experiments/results/e2m-qwen3-7-max.json` | `11fb8e7b5ee44c75` | 17497 |
| `experiments/results/e2m-qwen3-7-plus.json` | `7d659f3d765c6ea4` | 17567 |
| `experiments/results/e2m-qwen3-8-flash.json` | `0fca0e1c7b504c08` | 17552 |
| `experiments/results/e2m-qwen3-8-max.json` | `de5fcc2f962687a4` | 17609 |
| `experiments/results/e2m-qwen4-max-preview.json` | `5fefd240f0c7d78f` | 17467 |
| `experiments/results/e2m-qwen4-max.json` | `2fa762d1356b388f` | 17538 |
| `experiments/results/e4-deepseek-ablations.json` | `8e3eb52d3583771c` | 27233 |
| `experiments/results/e5-gate-ci-deepseek.json` | `557655ad58d68da5` | 41064 |
| `experiments/results/e7-langchain-baseline.json` | `70e8f55f872cb0c1` | 5612 |
| `experiments/results/e7-sfai-headtohead.json` | `b3336b3b6a61ea61` | 9713 |
| `experiments/results/e7ext-pydantic-ai.json` | `d93206d2738f1552` | 5514 |
| `experiments/results/e7ext-smolagents.json` | `8c84d3d0183e3dd9` | 5510 |
| `experiments/results/e8-routing-deepseek.json` | `c9a6ea2b3adac963` | 20451 |
| `experiments/results/e9-public-deepseek.json` | `0c6b7dfd5c98a0ac` | 53027 |
| `experiments/results/e9-public-humaneval-bailian.json` | `7c2f2a3f7b7a02d4` | 25611 |
| `experiments/results/e9-public-humaneval-rerun.json` | `f14962aaf2e86a38` | 33618 |
| `experiments/results/e9-public-sheets-bailian.json` | `0ce526244a20327a` | 25373 |
| `evals/reports/e5-sweep.json` | `ed7f83ed2560af0b` | 13535 |
| `evals/reports/gate-metrics.json` | `8335d42de675f8fc` | 2935 |
| `evals/reports/v0.1-agent-benchmark.json` | `d3ef6b96cd0e080e` | 41218 |
| `evals/reports/v0.1-sparse-context-baseline.json` | `f497462784dcdeab` | 17989 |
| `evals/reports/v0.2-agent-benchmark.json` | `8c8da467f8c97fa4` | 50803 |
| `evals/reports/v0.2.1-agent-benchmark.json` | `3f44cb83d5af9e11` | 50794 |
| `evals/public/tasks.json` | `d541c8be60e61a28` | 178060 |

## 冻结口径

- bench v0.2.1；判分器修正史 3 次（Debug 报告 #13），作废版本不在清单
- 模型 18（DeepSeek、MiMo、百炼 16 文本）；框架 4（sfai/LangGraph/smolagents/Pydantic AI）
- 公开任务集 60（GSM8K 20 / HumanEval 20 / SpreadsheetBench-sheets 20，许可见 evals/public/README）
- E8 双向结果如实呈现；E9 网络事故作废记录见 Debug 报告 #14
