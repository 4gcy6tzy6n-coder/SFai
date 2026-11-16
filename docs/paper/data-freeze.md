# 数据冻结清单 (DATA FREEZE)

**冻结时间:** 2026-11-16 · **bench:** v0.2.1 · **commit:** 见 git log 尾部
**状态:** ✅ 冻结完成 —— 论文所有数字以此清单为准；任何重跑产生新文件、不覆盖本清单文件

## 范围（31 个结果文件，全部已提交入库）

| 文件 | sha256[:16] | 大小 | 主体 | 条目 |
|---|---|---|---|---|
| `experiments/results/e1-deepseek-baseline.json` | `6dabd6b7836d659a` | 29281 | deepseek-chat | 48 |
| `experiments/results/e2-mimo-baseline.json` | `7782d2ad3800f747` | 28590 | mimo-v2.6-pro | 48 |
| `experiments/results/e2m-auto.json` | `3b1b0b731c97d686` | 17511 | auto | 16 |
| `experiments/results/e2m-decision-model-preview.json` | `fc8c34c2aa1e392b` | 19608 | decision-model-preview | 16 |
| `experiments/results/e2m-deepseek-v4-1-flash-1017.json` | `27495bee4693e61d` | 17493 | deepseek-v4.1-flash-1017 | 16 |
| `experiments/results/e2m-deepseek-v4-1-flash.json` | `469e067c9ba548f5` | 17524 | deepseek-v4.1-flash | 16 |
| `experiments/results/e2m-deepseek-v4-flash-0731.json` | `88bce7d624889938` | 17566 | deepseek-v4-flash-0731 | 16 |
| `experiments/results/e2m-deepseek-v4-pro-0813.json` | `27a02aa906ac9e58` | 17526 | deepseek-v4-pro-0813 | 16 |
| `experiments/results/e2m-deepseek-v4-pro.json` | `a9e19bb40e533e0b` | 17542 | deepseek-v4-pro | 16 |
| `experiments/results/e2m-glm-5-2.json` | `34f10cc0d74cc848` | 17546 | glm-5.2 | 16 |
| `experiments/results/e2m-glm-5-3.json` | `b7bc37d3069db2ce` | 17533 | glm-5.3 | 16 |
| `experiments/results/e2m-qwen3-6-flash.json` | `4eeb007f487322fa` | 17512 | qwen3.6-flash | 16 |
| `experiments/results/e2m-qwen3-7-max.json` | `11fb8e7b5ee44c75` | 17497 | qwen3.7-max | 16 |
| `experiments/results/e2m-qwen3-7-plus.json` | `7d659f3d765c6ea4` | 17567 | qwen3.7-plus | 16 |
| `experiments/results/e2m-qwen3-8-flash.json` | `0fca0e1c7b504c08` | 17552 | qwen3.8-flash | 16 |
| `experiments/results/e2m-qwen3-8-max.json` | `de5fcc2f962687a4` | 17609 | qwen3.8-max | 16 |
| `experiments/results/e2m-qwen4-max-preview.json` | `5fefd240f0c7d78f` | 17467 | qwen4-max-preview | 16 |
| `experiments/results/e2m-qwen4-max.json` | `2fa762d1356b388f` | 17538 | qwen4-max | 16 |
| `experiments/results/e4-deepseek-ablations.json` | `8e3eb52d3583771c` | 27233 | deepseek-chat | 40 |
| `experiments/results/e5-gate-ci-deepseek.json` | `557655ad58d68da5` | 41064 | deepseek-chat | 64 |
| `experiments/results/e7-langchain-baseline.json` | `70e8f55f872cb0c1` | 5612 | deepseek-chat | 12 |
| `experiments/results/e7-sfai-headtohead.json` | `b3336b3b6a61ea61` | 9713 | deepseek-chat | 12 |
| `experiments/results/e7ext-pydantic-ai.json` | `d93206d2738f1552` | 5514 | deepseek-chat | 12 |
| `experiments/results/e7ext-smolagents.json` | `8c84d3d0183e3dd9` | 5510 | deepseek-chat | 12 |
| `experiments/results/e8-routing-deepseek.json` | `c9a6ea2b3adac963` | 20451 | deepseek-chat | 24 |
| `evals/reports/e5-sweep.json` | `ed7f83ed2560af0b` | 13535 | e5-sweep | 63 |
| `evals/reports/gate-metrics.json` | `8335d42de675f8fc` | 2935 | gate-metrics | 0 |
| `evals/reports/v0.1-agent-benchmark.json` | `d3ef6b96cd0e080e` | 41218 | v0.1-agent-benchmark | 80 |
| `evals/reports/v0.1-sparse-context-baseline.json` | `f497462784dcdeab` | 17989 | sparse-context | 5 |
| `evals/reports/v0.2-agent-benchmark.json` | `8c8da467f8c97fa4` | 50803 | v0.2-agent-benchmark | 80 |
| `evals/reports/v0.2.1-agent-benchmark.json` | `3f44cb83d5af9e11` | 50794 | v0.2.1-agent-benchmark | 80 |

## 冻结口径

- **bench v0.2.1**（repetitions/seed/Wilson CI）；判分器修正史共 3 次（见 Debug 报告 #13），
  本清单仅含修正后重跑数据，作废版本不在清单内。
- 模型：18（DeepSeek chat、MiMo v2.6-pro、百炼 16 文本模型；Kimi 不在端点、
  7 个非文本模型排除，均已注明）。
- 框架：sfai + LangGraph + smolagents + Pydantic AI（v2 修正口径）。
- 已知边界（论文 Limitations 必须携带）：n=2~16/cell、合成套件、单机延迟、
  订阅制计价、E7 并发扫描延迟噪声、E8 双向结果。

## 复现命令

```bash
cargo test                                    # 109 tests
./scripts/run_experiments.sh                  # 各 config 依 key 而跑
python3 scripts/grand_model_table.py          # 总表
cargo run --release -- eval --mode sweep|gate # 评测
```
