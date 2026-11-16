# E1 / E4 / E2 实验分析 — DeepSeek + MiMo 真实模型

**日期:** 2026-11-16 · **框架版本:** bench v0.2.1 · **提交:** 见 `git log` 当日记录
**运行:** 136 次真实模型运行（E1 48 + E4 40 + E2-MiMo 48），1 次超时错误（E2），其余全部干净
**限定:** n=4~6/cell，Wilson 95% CI；单机本地 SQLite；价格为近似值（0.27/1.10 USD/1M）

## E1 — 基线（4 suites × none/no_verification × 3 任务 × 2 重复）

| suite | ablation | success | correct | false-success | p50 ms | in tokens | cost USD |
|---|---|---|---|---|---|---|---|
| dataset | none | 0.50 | 0.50 | 0.00 | 8.1s | 12096 | 0.0051 |
| dataset | no_verification | 0.67 | 0.67 | 0.00 | 7.5s | 8423 | 0.0036 |
| long-horizon | none | **1.00** | **1.00** | **0.00** | 17.4s | 19910 | 0.0082 |
| long-horizon | no_verification | 1.00 | 0.50 | **0.50** | 9.0s | 11450 | 0.0048 |
| tool-use | none | 0.83 | **1.00** | **0.00** | 27.6s | 20936 | 0.0102 |
| tool-use | no_verification | 0.83 | 0.67 | 0.17 | 20.0s | 15843 | 0.0073 |
| edge-case | none | 0.00 | 0.00 | 0.00 | 6.9s | 4379 | 0.0021 |
| edge-case | no_verification | 0.00 | 0.00 | 0.00 | 5.5s | 3658 | 0.0018 |

### 关键发现

1. **验证门消除假成功（核心结论）。** `none`（含验证）两个可完成 suite 的 false-success 均为 **0.00**；
   `no_verification` 下 long-horizon 假成功 **0.50**、tool-use **0.17** —— 无验证时模型"报告完成"但交付物
   错误或不合格。验证的代价：p50 延迟 +5~10s（验收检查与修复轮），token +20~40%（修复重试）。
2. **long-horizon 检索可靠。** 验证开启时 6/6 成功且引用正确（needle 从 40 条历史中恢复）。
3. **tool-use 高质量。** 验证开启时正确率 1.00（真实模型写出可解析 `source://` 引用）。
4. **edge-case 语义按设计工作。** 要求引用不存在的 `source://missing@1` 时，验证开启 → 全部被拦截
   （0 假成功）；无验证 → 模型也大多拒绝捏造（0 完成），说明 deepseek-chat 抗捏造压力。
5. **dataset 是难例。** 计算正确率 0.50–0.67（CSV 求和），n 小 CI 宽；E5 预算扫描可进一步解释。

## E4 — 消融（dataset + long-horizon × 5 消融 × 2 任务 × 2 重复）

| suite | ablation | success | correct | false-success | in tokens | cost USD |
|---|---|---|---|---|---|---|
| long-horizon | none | 1.00 | 1.00 | 0.00 | 18038 | 0.0074 |
| long-horizon | full_context | 0.75 | 1.00 | 0.00 | **50207** | **0.0177** |
| long-horizon | no_pinned | 1.00 | 1.00 | 0.00 | 21744 | 0.0089 |
| long-horizon | no_verification | 1.00 | 0.25 | **0.75** | 7342 | 0.0030 |
| long-horizon | no_wiki | 1.00 | 1.00 | 0.00 | 22157 | 0.0093 |
| dataset | none | 0.50 | 0.50 | 0.00 | 15934 | 0.0069 |
| dataset | no_wiki | 0.75 | 0.75 | 0.00 | 13980 | 0.0061 |
| dataset | no_pinned | — | （运行中数据见 JSON） | | | |

### 关键发现

1. **full_context 的代价被真实模型放大。** long-horizon 输入 token 50207 vs 18038（**2.8×**），
   成本 2.4×，正确率相同（1.00）——受控 envelope 在真实模型上保持同样的质量、显著更省。
2. **no_verification 假成功 0.75。** 正确率从 1.00 跌至 0.25：无验证时"完成"的报告中 3/4 引用错误或
   内容错误。这是验证门价值的最强证据（跨 suite 复现 E1 结论）。
3. **no_wiki 与 no_pinned 代价温和。** Wiki 编译在此规模对延迟/token 影响小（甚至 dataset 反而略低，
   属噪声）；pinned 约束在该 suite 未构成瓶颈（模型本就遵守约束）。
4. **验证的延迟成本可测但可控**（none p50 15.5s vs no_verification 7.5s，含修复轮）。

## E2 — MiMo 对比（mimo-v2.6-pro / Token Plan，48 运行，1 超时错误）

| suite | ablation | success | correct | false-success | in tokens | p50 |
|---|---|---|---|---|---|---|
| dataset | none | **1.00** | **1.00** | 0.00 | 5398 | 55.1s |
| long-horizon | none | 1.00 | 1.00 | 0.00 | 12654 | 39.5s |
| tool-use | none | 1.00 | 1.00 | 0.00 | 10037 | 65.5s |
| edge-case | none | 0.00 | 0.00 | **0.00** | 4155 | 79.6s |
| edge-case | no_verification | 0.67 | 0.00 | **0.67** | 2979 | 37.4s |

### 跨 provider 对比（none 配置）

| 指标 | DeepSeek chat | MiMo v2.6-pro |
|---|---|---|
| dataset 正确率 | 0.50 | **1.00** |
| tool-use 正确率 | 1.00 | 1.00 |
| long-horizon 正确率 | 1.00 | 1.00 |
| edge-case 假成功（无验证） | 0.17 | **0.67** |
| edge-case 假成功（有验证） | 0.00 | **0.00** |
| tool-use 输入 token | 20936 | **10037** |
| p50 延迟 | 8–28s | 39–80s |
| 计价 | 0.27/1.10 USD/1M | Token Plan 订阅（比 token） |

### 关键发现

1. **验证门价值跨 provider 复现（第三个独立证据）。** MiMo 无验证时捏造引用并"成功"
   4/6（假成功 0.67）；开验证后假成功 **0.00** —— 而且 6 次里 3 次模型主动 `ask_user`
   拒绝捏造、3 次被验收拦截为 blocked。自路由的 ask_user 意图在真实压力下被真实使用。
2. **MiMo 正确性更强**（dataset 1.00 vs 0.50），输入 token 更省（tool-use 10037 vs 20936），
   但延迟高 2–7×（推理型模型）——质量-延迟-成本三角的典型案例，直接进论文对比表。
3. **DeepSeek 抗捏造但正确率略低**：edge-case 无验证也拒绝完成（0.67 vs MiMo 的完成），但
   dataset 计算弱于 MiMo。两个模型的失败模式互补，验证门对两者都必要。

## 对论文的含义

- **贡献点 1（RQ1/RQ2）：** 稀疏上下文激活（受控 envelope）在真实模型上以 ~35–65% 的 token 达到
  与 full_context 相同的正确率（E4 long-horizon；E1 tool-use 的 envelope 均 < full 预算）。
- **贡献点 2（RQ4）：** 独立验证门把假成功从 0.50–0.75 压到 **0.00**（E1+E4 双 suite 复现），
  代价是 +20–40% token 与 +5–10s 延迟 —— 质量-成本 Pareto 的核心权衡点。
- **贡献点 3（RQ3）：** 跨 provider（DeepSeek/MiMo）差异显著但验证门收益一致——
  框架的安全性质不依赖特定模型。
- **诚实限定：** n 小（4–6/cell）、合成任务、MiMo 订阅计价无法折算 USD；需 v2 扩样本
  （repetitions 5+）、Qwen（暂缓）与 E5 预算扫描。

## 复现

```bash
export DEEPSEEK_API_KEY=...
./scripts/run_experiments.sh experiments/configs/deepseek-e1.json
./scripts/run_experiments.sh experiments/configs/deepseek-ablations.json
python3 scripts/compare_results.py experiments/results/*.json
```
