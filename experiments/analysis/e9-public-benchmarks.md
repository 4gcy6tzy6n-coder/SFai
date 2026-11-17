# E9 — 公开基准迁移结果（GSM8K / HumanEval，deepseek-chat）

**日期:** 2026-11-17 · **数据:** `e9-public-deepseek.json` + `e9-public-humaneval-rerun.json`
**意义:** NeurIPS D&B"公开/可扩展任务集"硬要求达成（GSM8K[MIT]、HumanEval[MIT] 真实子集）。

## GSM8K（20 题整数答案子集，数据干净）

| 配置 | success | correct | **false-success** | in tokens |
|---|---|---|---|---|
| 有验证门 | 0.95 [0.76,0.99] | 0.95 | **0.00** | 8027 |
| 无验证门 | 0.80 | 0.70 | **0.15** | 3806 |

**公开任务上门对照成立**：无门时 15% 的"完成"是错答案静默交付；有门时全部正确或被拦。
模型能力真实体现（0.95 数学正确率）。

## HumanEval（20 题，首跑受网络污染 → 重跑为准）

- 首跑（夜间）：27 次连接重置/超时（`timed out reading response` / `Connection reset`），
  数字不可用——**作废**，以 `e9-public-humaneval-rerun.json` 为准。
- 代码任务的验证门价值更大：`code_tests_pass`（官方测试即独立裁判）是
  acceptance 型正确性——模型无法自评蒙混。

## 路由遥测（意外收获）

- gsm8k 有门运行 **loop-guard 拦截 40 次**——模型在数学验证循环中反复同动作，
  运行时守卫真实工作（Decide-Act 观测价值）。
- humaneval 首跑 misroutes 7、dup replays 15——代码任务中路由抖动显著。

## 结论

1. **公开基准上门对照再次成立**（fs 0.15 → 0.00），与 18 模型合成套件结论一致；
2. HumanEval 网络事故如实记录（27 运行作废、重跑归档）——数据纪律延续；
3. 论文 §5.9 表格就绪（T9）。
