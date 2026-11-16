# Debug Report — M8 实验基础设施 v0（v0.1 增量）

- **日期**: 2026-11-16
- **开发者 / 会话**: lead（agent dev session）
- **关联 milestone**: M8 基线（稀疏上下文评测 harness、研究计划、双轨产出落档）
- **判定**: **PASS**

## 1. 症状（Scope）

落实双轨产出约束（Agent 应用 + 顶级论文 + 实验），交付：

- `docs/research-plan.md`：双轨定义、RQ1–RQ4、baselines/消融、指标、协议、论文 gate；
- `sfai eval` 离线评测 harness：5 种上下文策略、确定性场景、指标聚合、JSON+Markdown 报告；
- v0.1 基线报告（12 场景 / 120 token 预算）；
- 新增 `tests/eval.rs`（5 个测试）。

本报告记录该增量的 Debug 环节。

## 2. 复现步骤（Reproduction）

```bash
cargo test                                   # 73 个离线测试
cargo clippy --all-targets                    # 0 warnings
cargo fmt --all --check                       # clean
cargo run -- eval --size 12 --out evals/reports/v0.1-sparse-context-baseline.json
```

## 3. 根因与修复（Root cause & Fix）

| # | 症状 | 复现证据 | 根因 | 修复 | 回归用例 |
|---|---|---|---|---|---|
| 1 | 确定性测试失败：两次运行 metrics 不一致 | `eval::suite_metrics_are_deterministic` 左右差异仅在 `mean_latency_ms` | 指标包含实测墙钟时间，天然非确定 | 测试比较时排除 `mean_latency_ms`（保留在报告中） | 同左 |
| 2 | 默认预算下 working-set 无 token 优势 | 初版 `DEFAULT_BUDGET_TOKENS=900` 大于全历史 token（~810） | 预算大于历史总量 → 选择器保留全部文档 | 默认预算调为 120 token，使稀疏选择可观测 | `eval::working_set_matches_full_history_success_at_lower_token_cost` |
| 3 | `cargo fmt --check` 失败 | 增量后格式未整理 | 新代码未格式化 | `cargo fmt` | fmt clean |

## 4. 修复范围（Files）

`crates/sfai/src/eval.rs`（新增）、`crates/sfai/src/lib.rs`、`crates/sfai/src/bin/sfai.rs`（`eval` 子命令）、
`crates/sfai/tests/eval.rs`（新增）、`docs/research-plan.md`、`docs/decisions/ADR-017-evaluation-harness.md`、
`evals/reports/v0.1-sparse-context-baseline.{md,json}`、`README.md`。

## 5. 回归验证（Regression）

| 检查 | 结果 |
|---|---|
| `cargo test` | **73 passed / 0 failed**（lib 30 + api 4 + cli_ui 4 + e2e 3 + eval 5 + fixtures 5 + m6 5 + security 4 + skills 4 + storage 4 + wiki 5） |
| `cargo clippy --all-targets` | **0 warnings** |
| `cargo fmt --all --check` | clean |
| 评测确定性 | 同参数两次运行除 latency 外逐字段一致 |
| 策略不变量 | working_set 成功 1.00、约束保留 1.00、token 约为 full_history 的 13%；sliding_window 召回 0.33；summary/retrieval 约束保留 0.00 |
| 报告格式 | Markdown 表格包含全部策略与指标；JSON 可解析 |

### v0.1 基线（12 场景，120 token 预算）

| policy | success | needle recall | constraint retention | mean input tokens |
|---|---|---|---|---|
| full_history | 1.00 | 1.00 | 1.00 | 810.0 |
| working_set | 1.00 | 1.00 | 1.00 | 107.0 |
| sliding_window | 0.00 | 0.33 | 0.00 | 121.3 |
| summary | 0.00 | 1.00 | 0.00 | 24.0 |
| retrieval_topk | 0.00 | 1.00 | 0.00 | 24.0 |

> 重要限定：答案为确定性模拟器生成、任务为合成 fixture、策略在上下文选择层模拟。
> 该基线的意义是**验证测量管线与协议**，不构成论文贡献证据（见 research-plan §6）。

## 6. 遗留项（Known issues / Remaining）

1. **real-model pilot 未做**（v1）：需要配置 provider、≥5 seeds、冻结模型版本、置信区间。
2. **公开可复现任务未纳入**（v2/v3）。
3. **路由/能力条件基线（§13.3 6–9）与消融未实现**。
4. **预算扫描与 Pareto 曲线**未实现。
5. 用户正在开发的 CLI UI（`ui.rs` / `tests/cli_ui.rs`）已随提交保留，未改动其行为。

## 7. 判定（Verdict）

**PASS** — 双轨产出已落档（文档 + 版本库），实验基础设施 v0 可运行、可复现、
指标可审计；73 个离线测试通过，clippy 0 告警，格式干净。下一步进入 v1
真实模型 pilot 与消融实验。
