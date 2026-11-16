# Debug Report — Agent 框架基准 v0.1（增量）

- **日期**: 2026-11-16
- **开发者 / 会话**: lead（agent dev session）
- **关联 milestone**: M8+（时间 / 正确率 / token / 成本 / 消融的端到端 benchmark）
- **判定**: **PASS**

## 1. 症状（Scope）

在 `sfai eval`（上下文策略层）之外，交付端到端 **Agent 框架基准**：

- 4 套任务集：`tool-use`、`dataset`、`long-horizon`、`edge-case`；
- 5 个消融：`none`、`full_context`、`no_pinned`、`no_verification`、`no_wiki`；
- 指标：success、ground-truth correctness、false-success、latency p50/p95、
  input/output tokens、cost、cost/tokens per success；
- `sfai bench` CLI（JSON + Markdown）、规范文档、v0.1 基线归档。

## 2. 复现步骤（Reproduction）

```bash
cargo test
cargo clippy --all-targets          # 0 warnings
cargo fmt --all --check             # clean
cargo run -- bench --size 4 --out evals/reports/v0.1-agent-benchmark.json
```

## 3. 根因与修复（Root cause & Fix）

| # | 症状 | 复现证据 | 根因 | 修复 | 回归用例 |
|---|---|---|---|---|---|
| 1 | `full_context` 消融几乎不增加 token（与 `none` 相近） | 基线 `long-horizon/full_context` 2244 vs `none` 2243 | EnvelopeBuilder 的候选池硬编码 12，抬预算但不抬候选 | 新增可配置 `candidate_limit`（默认 12），消融设为 1000 | 基线变为 4843 vs 2243；`benchmark` 测试 |
| 2 | `Box<dyn ModelAdapter>` 传给 `run_observed` 编译失败 | `cargo check` E0277 | 直接传 `&mut model`，未解引用到 trait object | `model.as_mut()` | 编译 + 全部测试 |
| 3 | `sfai::benchmark` 解析失败 | `cargo check` E0433 | lib.rs 未声明模块 | 添加 `pub mod benchmark;` | 编译 |
| 4 | clippy `manual_contains` 告警 | `cargo clippy` | `keys.iter().any(|k| *k == key)` | 改为 `keys.contains(&key)` | clippy 0 告警 |
| 5 | 正确性检查可能被种子数据误满足 | 代码审查 | 扫描全部工件文本，CSV 可能包含总数字串 | 按 acceptance 规则推导期望工件路径，只在这些工件内匹配 ground truth | `dataset_tasks_are_completed_and_correct` |
| 6 | 格式检查失败 | `cargo fmt --check` | 新模块未格式化 | `cargo fmt` | fmt clean |

## 4. 回归验证（Regression）

| 检查 | 结果 |
|---|---|
| `cargo test` | **78 passed / 0 failed**（lib 30 + api 4 + cli_ui 4 + e2e 3 + eval 5 + fixtures 5 + m6 5 + security 4 + skills 4 + storage 4 + wiki 5 + benchmark 5） |
| `cargo clippy --all-targets` | **0 warnings** |
| `cargo fmt --all --check` | clean |
| 基线复现 | `sfai bench --size 4` 输出见 `evals/reports/v0.1-agent-benchmark.md` |
| 假成功检测 | `edge-case × no_verification` false-success = 1.00；`edge-case × none` success = 0（被验证门拦截） |
| full-context 代价 | `long-horizon`: 4843 vs 2243 input tokens（相同 success/correctness） |

### v0.1 关键单元格

| suite | ablation | success | correct | false-success | p50 ms | in tokens |
|---|---|---|---|---|---|---|
| dataset | none | 1.00 | 1.00 | 0.00 | 4.0 | 494 |
| tool-use | none | 1.00 | 1.00 | 0.00 | 4.2 | 1062 |
| long-horizon | none | 1.00 | 1.00 | 0.00 | 9.7 | 2243 |
| long-horizon | full_context | 1.00 | 1.00 | 0.00 | 12.4 | 4843 |
| edge-case | none | 0.00 | 1.00 | 0.00 | 5.9 | 1175 |
| edge-case | no_verification | 1.00 | 1.00 | **1.00** | 3.2 | 465 |

## 5. 遗留项（Known issues / Remaining）

1. **v1 真实模型**：脚本化模型只度量框架行为，不度量模型能力；需配置 provider 做多 seed 真实模型跑。
2. **置信区间 / 硬件披露**：重复实验与报告规范待补。
3. **公开任务集**：当前任务为合成 fixture。
4. **Pareto 扫描**：预算/规模曲线未实现。
5. 用户前端（UI/dmg）仍在开发中，后端接口与 benchmark 不受影响。

## 6. 判定（Verdict）

**PASS** — 端到端 benchmark 可运行、可复现、指标可审计；78 个离线测试通过、
clippy 0 告警、格式干净；v0.1 基线已归档，其中 `no_verification` 的 100%
假成功率直接量化了验证门的价值。
