# Debug Report — 真实模型实验执行（E1 / E4 / E2-MiMo）

- **日期**: 2026-11-16
- **开发者 / 会话**: lead（agent dev session）
- **关联 milestone**: 真实模型实验（DeepSeek E1 基线 + E4 消融；MiMo Token Plan E2 对比）
- **判定**: **PASS**（E1/E4 数据干净落盘；E2 见 §5.4）

## 1. 症状（Scope）

用真实模型执行论文实验并修复实验暴露的全部缺陷：

1. E1（DeepSeek 基线：4 suites × none/no_verification × 3 任务 × 2 重复 = 48 运行）
2. E4（消融：2 suites × 5 消融 × 2 任务 × 2 重复 = 40 运行）
3. E2-MiMo（对比：4 suites × 2 消融 × 3 任务 × 2 重复 = 48 运行，mimo-v2.6-pro / Token Plan）
4. 实验过程中暴露的基准指标语义缺陷与传输层缺陷的修复

## 2. 复现步骤（Reproduction）

```bash
export DEEPSEEK_API_KEY=...   # 未入库，仅环境变量
export XIAOMI_MIMO_API_KEY=...
./target/release/sfai bench --config experiments/configs/deepseek-e1.json \
  --storage experiments/.run/storage --out experiments/results/e1-deepseek-baseline.json
# 同理 deepseek-ablations.json / mimo-e2.json
python3 scripts/compare_results.py experiments/results/*.json
```

## 3. 根因与修复（Root cause & Fix）

| # | 症状 | 复现证据 | 根因 | 修复 | 回归用例 |
|---|---|---|---|---|---|
| 1 | 首次 E1 大面积 `error`（~5s 均匀延迟、0 token） | e1 第一轮 JSON：48 运行中 30+ error | 批量请求触发限流/瞬时故障，重试仅 3 次且退避过短（0.5/1s） | 5 次重试、指数退避（1/2/4/8/16s）、尊重 Retry-After、封顶 30s | 第二轮 E1 零限流错误 |
| 2 | 第二轮 E1 tool-use `correct=0.17` 但验收全过 | 复现运行：真实报告含 3 个可解析 `source://` 引用，验收 pass | `correct` 依赖字面 marker "Citations"，真实模型用其他标题 | ground_truth 改为 `source://`（可验证引用证据） | `tests/benchmark.rs`；E1 最终轮 correct=1.00 |
| 3 | `citations_resolve` 对零引用报告空过 | 代码审查：checked==0 → pass | 判定逻辑只查"是否有坏引用" | 新增 `NO_CITATIONS_FOUND`：零引用即 fail | `m6::citations_resolve_rejects_reports_without_any_citation` |
| 4 | edge-case 真实模型语义反转 | E1 第二轮：`correct=1` 表示"含坏引用" | ground_truth 是坏引用，"correct=contains" 衡量的是捏造合规 | `ground_truth_inverted`：正确交付物必须**不含**捏造引用；goal 显式施加引用压力 | `tests/benchmark.rs`；E1 最终轮 edge-case correct=0.00（未捏造） |
| 5 | 3/48 运行死于模型协议违规（JSON 截断、kind="read"、JSON EOF） | E1 第二轮 stderr | 模型偶发输出不可解析 | 适配器级**一次协议修复重试**（附错误提示的纠正往返） | `repair_retry_recovers_from_malformed_intent`（mock 双响应） |
| 6 | E4 整场作废（全部 0 token 瞬时失败） | E4 stderr：`tls connection init failed: CaUsedAsEndEntity` ×40 | DeepSeek 证书链瞬时异常（TLS 故障期）；传输层错误不重试 | 传输层错误纳入重试（与 429/5xx 同策略）；E4 重跑 | 重跑后 E4 零错误；TLS 已恢复（openssl 验证 ok） |
| 7 | bench 失败运行丢失错误信息/成本 | 第一轮 E1 error 行 0 token 无原因 | 错误路径未记录 | `BenchTaskResult.error` 字段 + stderr 即时打印 | E1/E4 中任何错误即时可见 |
| 8 | 脚本化 runner 参数错 | `--acceptance` 不存在 | 用户 UI 改版后 CLI 参数为 `--accept` | 文档与脚本使用正确参数 | 复现脚本 |
| 9 | `cargo fmt`/版本断言遗漏 | `--check` 失败、`0.2.0` 断言 | 增量修改后未同步 | 格式化 + 断言更新 | fmt clean、92 测试全过 |

## 4. 回归验证（Regression）

| 检查 | 结果 |
|---|---|
| `cargo test --release` | **92 passed / 0 failed** |
| `cargo clippy --all-targets` | **0 warnings** |
| `cargo fmt --all --check` | clean |
| 密钥安全 | 全仓库扫描 `sk-`/`tp-`/`ttp-` 模式：0 命中（密钥仅经环境变量传递） |
| E1（最终轮） | 48 运行 **0 错误**；关键单元格见 §5 |
| E4（重跑后） | 40 运行 **0 错误**；消融信号成立 |

### 5.1 E1 核心结果（deepseek-chat，n=6/cell）

| suite | ablation | success | correct | false-success | in tokens |
|---|---|---|---|---|---|
| long-horizon | none | 1.00 | 1.00 | **0.00** | 19910 |
| long-horizon | no_verification | 1.00 | 0.50 | **0.50** | 11450 |
| tool-use | none | 0.83 | 1.00 | **0.00** | 20936 |
| tool-use | no_verification | 0.83 | 0.67 | 0.17 | 15843 |

### 5.2 E4 核心结果（消融）

- `full_context`：long-horizon 输入 token **50207 vs 18038（2.8×）**，正确率相同 1.00；
- `no_verification`：假成功 **0.75**（correct 1.00→0.25）；
- `no_wiki` / `no_pinned`：代价温和（噪声范围内）。

### 5.3 分析文档

`experiments/analysis/e1-e4-deepseek-analysis.md`（含对论文的含义与诚实限定）。

### 5.4 E2-MiMo 结果（已完成）

mimo-v2.6-pro（Token Plan，`token-plan-cn.xiaomimimo.com/v1`）48 运行完成（1 次 228s 超时）：

| suite | ablation | success | correct | false-success |
|---|---|---|---|---|
| dataset | none | 1.00 | 1.00 | 0.00 |
| tool-use | none | 1.00 | 1.00 | 0.00 |
| long-horizon | none | 1.00 | 1.00 | 0.00 |
| edge-case | none | 0.00 | 0.00 | 0.00 |
| edge-case | no_verification | 0.67 | 0.00 | **0.67** |

**验证门价值第三次复现（跨 provider）**：MiMo 无验证时 4/6 捏造引用并"成功"（假成功 0.67）；
开验证后 0.00 —— 且 3 次模型主动 `ask_user` 拒绝捏造、3 次被验收拦截。MiMo 正确性全面
强于 DeepSeek（dataset 1.00 vs 0.50）、输入 token 更省，但延迟高 2–7×（推理型）。
对比表：`experiments/analysis/` 与 `scripts/compare_results.py` 输出。

## 6. 遗留项（Known issues / Remaining）

1. Qwen 按用户指示暂缓（E2 对比现为 DeepSeek vs MiMo 双方）。
2. **bench 无进度输出**（只打错误行）——后续加 per-task 进度行（观测缺口）。
3. **MiMo Token Plan 无按量单价**：成本按 0 计价，比较以 token 为主（已在 config note 中标注）。
4. **n 小（4–6/cell）**：CI 宽；v2 需扩样本（repetitions 5+）并加 E5 预算扫描。
5. **模型协议违规仅单次修复重试**；残余失败会以 error 归档（当前 0 残余）。
6. 两个密钥已在聊天中暴露，**建议用户在控制台轮换**（未入库、仅环境变量使用）。

## 7. 判定（Verdict）

**PASS** — E1/E4 数据干净（88 运行 0 错误）、指标语义缺陷全部修复并回归、传输层
故障可自愈、密钥零泄漏；92 个测试通过、clippy 0 告警。E2-MiMo 执行中，完成后
并入分析并提交。
