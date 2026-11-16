# Debug Report — 实验准备（repetitions / CI / experiment registry）

- **日期**: 2026-11-16
- **开发者 / 会话**: lead（agent dev session）
- **关联 milestone**: 实验准备（benchmark v0.2 + 实验协议/配置/脚本）
- **判定**: **PASS**（离线全绿；真实模型 E1–E4 等待 API key）

## 1. 症状（Scope）

为论文实验做准备：

- benchmark v0.2：`--repetitions`、`--seed`、Wilson 95% CI、latency/cost 置信区间、
  JSON 实验配置 `BenchConfig`、`run_from_config`；
- CLI：`bench --config --repetitions --seed`；
- 实验配置：`experiments/configs/{deepseek-e1,deepseek-ablations,qwen-e2,mimo-e2}.json`；
- 运行脚本 `scripts/run_experiments.sh`（缺 key 自动跳过，不打印密钥）；
- 协议文档 `docs/experiments/README.md`（E1–E5、控制变量、指标、产物策略）；
- 重新生成 v0.2 基线（含 CI）并归档。

## 2. 复现步骤（Reproduction）

```bash
cargo test
cargo clippy --all-targets          # 0 warnings
cargo fmt --all --check             # clean
cargo run -- bench --config experiments/configs/deepseek-e1.json   # 需 DEEPSEEK_API_KEY
cargo run -- bench --repetitions 3 --seed 0                        # 脚本模型可离线
./scripts/run_experiments.sh        # 无 key 时 skip
```

## 3. 根因与修复（Root cause & Fix）

| # | 症状 | 复现证据 | 根因 | 修复 | 回归用例 |
|---|---|---|---|---|---|
| 1 | 确定性测试失败 | 左右差异在 `std_latency_ms` / `ci95_latency_ms` | 新增 latency 派生字段未排除 | 测试比较时一并移除 latency 派生字段 | `benchmark_report_is_deterministic_apart_from_latency` |
| 2 | Wilson CI 断言偶发失败 | 边界值比较 | 浮点边界（rate 与 CI 端点） | 断言加 1e-9 容差并输出诊断 | `confidence_intervals_are_sane` |
| 3 | 文档基线数值与实际不符 | v0.1 表格数值落后于代码 | 协议版本升级（种子参与任务生成） | 重新生成 v0.2 基线，逐行核对并修正 | `evals/reports/v0.2-agent-benchmark.{md,json}` |
| 4 | 格式检查失败 | `cargo fmt --check` | 新代码未格式化 | `cargo fmt` | fmt clean |

## 4. 回归验证（Regression）

| 检查 | 结果 |
|---|---|
| `cargo test` | **90 passed / 0 failed**（lib 36 + api 6 + benchmark 9 + cli_ui 4 + e2e 3 + eval 5 + fixtures 5 + m6 5 + security 4 + skills 4 + storage 4 + wiki 5） |
| `cargo clippy --all-targets` | **0 warnings** |
| `cargo fmt --all --check` | clean |
| benchmark v0.2 | 生成 `evals/reports/v0.2-agent-benchmark.{md,json}`（含 95% CI、seed、repetitions） |
| 配置解析 | `config_parses_and_runs` 通过；脚本在缺 key 时安全跳过 |

## 5. 遗留项（Known issues / Remaining）

1. **E1–E4 尚未真实运行**：本机无 provider key；导出 `DEEPSEEK_API_KEY` 后执行
   `./scripts/run_experiments.sh` 即可产出 `experiments/results/`。
2. E5（预算/质量 Pareto、固定 top-k vs token budget）尚未实现扫描脚本。
3. 置信区间为近似法（Wilson / 正态），论文需注明方法与重复次数。
4. 真实模型重复成本需预算控制（repetitions × size × suites × ablations）。

## 6. 判定（Verdict）

**PASS** — 实验准备完成：协议、配置、一键运行、CI 指标与基线归档就绪；
90 个离线测试通过、clippy 0 告警、格式干净。导出 key 后即可开始 E1。
