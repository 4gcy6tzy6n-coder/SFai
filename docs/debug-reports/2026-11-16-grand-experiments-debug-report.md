# Debug Report — 总实验收尾（18 模型 × 4 框架 × E8 + 论文框架）

- **日期**: 2026-11-16 · **判定**: **PASS**（附三次判分器事故的完整修正史）

## 1. 范围

百炼 16 文本模型全测（23 模型中 7 个非文本排除并注明）+ DeepSeek + MiMo =
**18 模型 × 4 suite × 门/无门（72 格）**；三框架 v2 重跑（LangGraph/smolagents/
Pydantic AI）；E8 Decide-Act 消融（真实）；论文框架定稿。

## 2. 根因与修复（判分器事故史，全部作废旧数字）

| # | 事故 | 根因 | 修复 | 影响 |
|---|---|---|---|---|
| 1 | LangChain dataset 正确率 0.00 | 键名强校验（"total" vs "total_revenue"） | 值匹配判分 | 旧数字作废 |
| 2 | 444.0 vs "444" 假阴性 | 字符串比较 | 数值比较（1e-9 容差） | 旧数字作废 |
| 3 | smolagents tool-use 0.00 / 陈旧交付物泄漏 | 共享工作区清理被误删 + 文件名强校验（agent 自选文件名） | 每任务清理交付物 + 文件名无关判分（任何文件含 ≥3 引用即对） | **E7 框架臂数字全部重跑** |

## 3. 终版数据

**18 模型聚合（72 格）：gate-on 假成功 0.000 vs gate-off 0.324。**
- 无门假成功最高 1.00（deepseek-v4-pro/qwen3.7-max 等多格）；MiMo edge-case 0.67。
- E8 Decide-Act（双向结果）：long-horizon 自路由 1.00 vs 固定 0.00；dataset
  0.25 vs 0.75；tool-use 平、自路由 4× token——"适应性换开销"定位成立。
- E7 v2：框架任务成功率接近（smolagents/Pydantic AI 本次 fs 0.00），差异在
  错误处理（LangGraph dataset fs 0.50 vs sfai 0.00）+ 18 模型大样本佐证。

## 4. 回归

- 109+ 测试全过、clippy 0、fmt clean（改动仅限判分脚本/分析文档/论文框架）。
- 全部结果 JSON 归档 `experiments/results/`；总表 `scripts/grand_model_table.py`。

## 5. 遗留

1. E7 n=6 CI 宽；v2 可扩样。
2. E8 双向结果 → future work：混合路由（uncertain→Decide-Act，certain→固定子程序）。
3. Kimi 不在百炼端点（已注明）；非文本模型 7 个排除。

## 6. 判定

**PASS** — 18 模型总表、三框架修正版、E8 消融、论文框架齐备；判分器三次
事故均发现-修正-重跑-归档，结论以本报告为准。
