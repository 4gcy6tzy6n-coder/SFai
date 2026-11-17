# 论文总数据表（Data Summary — 写作唯一数据源）

**冻结基线:** bench v0.2.1 · 判分器修正史 3 次（Debug 报告 #13/#14）· 全部数字从结果 JSON 程序化提取

## T1 — E1 DeepSeek 基线（验证门主对照）（e1-deepseek-baseline · deepseek/deepseek-chat · seed=0)

| suite | ablation | n | success | correct | **false-success** | in tok | out tok | p50 |
|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 6 | 0.67 | 0.67 | **0.00** | 8423 | 1244 | 7.5s |
| dataset | none | 6 | 0.50 | 0.50 | **0.00** | 12096 | 1705 | 8.1s |
| edge-case | no_verification | 6 | 0.00 | 0.00 | **0.00** | 3658 | 697 | 5.5s |
| edge-case | none | 6 | 0.00 | 0.00 | **0.00** | 4379 | 831 | 6.9s |
| long-horizon | no_verification | 6 | 1.00 | 0.50 | **0.50** | 11450 | 1582 | 9.0s |
| long-horizon | none | 6 | 1.00 | 1.00 | **0.00** | 19910 | 2550 | 17.4s |
| tool-use | no_verification | 6 | 0.83 | 0.67 | **0.17** | 15843 | 2791 | 20.0s |
| tool-use | none | 6 | 0.83 | 1.00 | **0.00** | 20936 | 4122 | 27.6s |

## T2 — E4 消融（DeepSeek）（e4-deepseek-ablations · deepseek/deepseek-chat · seed=0)

| suite | ablation | n | success | correct | **false-success** | in tok | out tok | p50 |
|---|---|---|---|---|---|---|---|---|
| dataset | full_context | 4 | 0.75 | 0.75 | **0.00** | 10732 | 1578 | 14.6s |
| dataset | no_pinned | 4 | 0.50 | 0.50 | **0.00** | 10859 | 1541 | 11.5s |
| dataset | no_verification | 4 | 0.75 | 0.75 | **0.00** | 9597 | 1388 | 10.1s |
| dataset | no_wiki | 4 | 0.75 | 0.75 | **0.00** | 13980 | 2102 | 19.2s |
| dataset | none | 4 | 0.50 | 0.50 | **0.00** | 15934 | 2339 | 21.4s |
| long-horizon | full_context | 4 | 0.75 | 1.00 | **0.00** | 50207 | 3723 | 32.9s |
| long-horizon | no_pinned | 4 | 1.00 | 1.00 | **0.00** | 21744 | 2797 | 15.3s |
| long-horizon | no_verification | 4 | 1.00 | 0.25 | **0.75** | 7342 | 954 | 7.5s |
| long-horizon | no_wiki | 4 | 1.00 | 1.00 | **0.00** | 22157 | 3015 | 23.3s |
| long-horizon | none | 4 | 1.00 | 1.00 | **0.00** | 18038 | 2301 | 15.5s |

## T3 — E5 稀疏激活曲线（working_set vs full_history）

见 `evals/reports/e5-sweep.json` 与 `experiments/analysis/pillar-evidence-strengthening.md`：
- working_set：40/80/160 文档下 **41 tok 恒定**（约束保真 1.00）
- full_history：**810 → 3210 tok**（@160 docs，**78× 成本差**，约束保真 0.00 衰减）
- top-k 曲线：k=2 于 160 文档翻车（漏检约束）→ 32 处 target 权重锚定

## T4 — E5-gate CI（n=16）+ 门精度矩阵 + ask_user

| suite | ablation | n | success | correct | **false-success** |
|---|---|---|---|---|---|
| edge-case | no_verification | 16 | 0.38 | 0.00 | **0.38** |
| edge-case | none | 16 | 0.00 | 0.00 | **0.00** |
| long-horizon | no_verification | 16 | 1.00 | 0.62 | **0.38** |
| long-horizon | none | 16 | 0.94 | 1.00 | **0.00** |

- false-success CI（Wilson）：no_verification **0.38 [0.18, 0.61]** → none **0.00 [0.00, 0.19]**
- **配对 McNemar 精确检验**（同任务配对）：pooled b=12/c=0 **p=0.000488**；18 模型 176 对 b=52/c=0 **p=4.4e-16**（详见 `docs/paper/stats-appendix.md`）
- 注：Wilson CI 在 0.18–0.19 存在小幅重叠，『CI 不相交』的旧表述已废除；推断以配对检验为准
- 门精度矩阵（Eval/gate）：捕获 **1.00**（全部模式）/ 误杀 **0.00**（零误杀）
- ask_user 率：捏造压力下 **0.94**（真实自路由行为，两 provider 均复现）

## T5 — 18 模型 Grand Table（主表）

完整表：`experiments/analysis/grand-model-table.md`（`scripts/grand_model_table.py` 再生）。聚合：

> **72 格：有验证门假成功 = 0.000（72/72 全绿）｜无验证门 = 0.324｜零例外**

模型家族：DeepSeek-chat、MiMo-v2.6-pro、百炼 16 文本模型（Qwen3.6/3.7/3.8/4-max、GLM-5.2/5.3、DeepSeek-v4 系、auto、decision-model-preview）

## T6 — E7 跨框架（v2 修正口径，同模型同判分器）

### e7-langchain-baseline

| suite | n | success | correct | **false-success** | p50 |
|---|---|---|---|---|---|
| dataset | 6 | 0.50 | 0.00 | **0.50** | 2.2s |
| tool-use | 6 | 1.00 | 1.00 | **0.00** | 8.6s |

### e7ext-pydantic-ai

| suite | n | success | correct | **false-success** | p50 |
|---|---|---|---|---|---|
| dataset | 6 | 1.00 | 1.00 | **0.00** | 3.9s |
| tool-use | 6 | 1.00 | 1.00 | **0.00** | 10.3s |

### e7ext-smolagents

| suite | n | success | correct | **false-success** | p50 |
|---|---|---|---|---|---|
| dataset | 6 | 1.00 | 1.00 | **0.00** | 5.5s |
| tool-use | 6 | 1.00 | 1.00 | **0.00** | 5.7s |

sfai 对照（e7-sfai-headtohead）：dataset 0.83/0.83/0.00 · tool-use 1.00/1.00/0.00（更快框架更省时间，差异在错误处理）

## T7 — E8 Decide-Act 消融（自路由 vs 强定管线，deepseek-chat）（e8-routing-ablation-deepseek · deepseek/deepseek-chat · seed=11)

| suite | ablation | n | success | correct | **false-success** | in tok | p50 | steps |
|---|---|---|---|---|---|---|---|---|
| dataset | forced_pipeline | 4 | 0.75 | 0.75 | **0.00** | 5184 | 5.7s | - |
| dataset | none | 4 | 0.25 | 0.25 | **0.00** | 8338 | 9.6s | - |
| long-horizon | forced_pipeline | 4 | 0.00 | 0.00 | **0.00** | 12154 | 9.0s | - |
| long-horizon | none | 4 | 1.00 | 1.00 | **0.00** | 21876 | 12.0s | - |
| tool-use | forced_pipeline | 4 | 1.00 | 1.00 | **0.00** | 4956 | 5.2s | - |
| tool-use | none | 4 | 1.00 | 1.00 | **0.00** | 19023 | 23.8s | - |

双向结论：long-horizon 自路由 **1.00 vs 0.00**（决定性）；dataset 0.25 vs 0.75（流程化任务固定管线更优）；tool-use 平、自路由 4× token → **「适应性换开销」**

## T8 — E6/E6b 性能（对齐版 + Rust vs Python）

| 指标 | sfai (Rust) | 对照 | 比值 |
|---|---|---|---|
| 单任务 p50 | 1.54s | LangGraph 2.2s / DeepAgents 3.5s | 1.4×–2.3× |
| 冷启动 | 0.6–1.0ms | DeepAgents 255ms / Claude Agent SDK 378ms | **255×–378×** |
| 200k envelope 构建 | 4.2ms | — | 微秒级激活 |
| 16 路并发 CPU 扩展 | 19.4→**98.5** tasks/s（**5.1×**） | Python 线程 10.5→10.6（**1.0×**，GIL） | GIL 实证 |
| 内存 RSS | 10.6MB | 14.7–27.6MB | **2.6×** |
| 分发开销 | 5.6µs ±14.2µs | — | 排队噪声 > 分发本身 |

## T9 — E9 公开基准（真实迁移，GSM8K/HumanEval/Sheets）

### e9-public-deepseek（e9-public-deepseek · deepseek/deepseek-chat · seed=0)

| suite | ablation | n | success | correct | **false-success** | in tok |
|---|---|---|---|---|---|---|
| public-gsm8k | no_verification | 20 | 0.80 | 0.70 | **0.15** | 3806 |
| public-gsm8k | none | 20 | 0.95 | 0.95 | **0.00** | 8027 |
| public-humaneval | no_verification | 20 | 0.15 | 0.00 | **0.15** | 4719 |
| public-humaneval | none | 20 | 0.00 | 0.00 | **0.00** | 31331 |

### e9-public-humaneval-bailian（e9-public-humaneval-bailian · bailian/deepseek-v4.1-flash · seed=0)

| suite | ablation | n | success | correct | **false-success** | in tok |
|---|---|---|---|---|---|---|
| public-humaneval | no_verification | 20 | 0.80 | 0.00 | **0.80** | 31005 |
| public-humaneval | none | 20 | 0.00 | 0.00 | **0.00** | 46175 |

### e9-public-sheets-bailian（e9-public-sheets-bailian · bailian/deepseek-v4.1-flash · seed=0)

| suite | ablation | n | success | correct | **false-success** | in tok |
|---|---|---|---|---|---|---|
| public-sheets | no_verification | 20 | 0.15 | 0.15 | **0.00** | 7481 |
| public-sheets | none | 20 | 0.15 | 0.15 | **0.00** | 7313 |

亮点：**HumanEval 无门 fs 0.80 vs 有门 0.00**（80% 假成功被门归零——公开任务上最狠对照）；GSM8K 门对照成立（0.15→0.00，cor 0.95）；sheets 双 0.15 中性。

## T10 — E10 等验证器对等实验（风险 #3 回应）

| 臂 | 完成率 | 正确率 | fs/全部 | fs/声称成功 | 修复触发 |
|---|---|---|---|---|---|
| sfai（门+运行时） | 1.00 | 1.00 | 0.00 | 0.00 | 0 |
| LangGraph 无门 | 0.83 | 0.83 | 0.00 | 0.00 | — |
| LangGraph + 等验证器（2 轮） | 0.92 | 0.92 | 0.00 | 0.00 | 0 |

结论：低错误任务上门不触发（0 修复）；有效验证是活性成分（收益随错误率放大）；
Decide-Act 独特价值 = 运行时鲁棒性（12/12 vs 裸异常丢任务）+ 审计 + E8 适应性。
详见 `experiments/analysis/e10-verifier-parity.md`。

## 评测基线（sfai eval）

- sweep（5 策略）：`evals/reports/eval-sweep-report.json` — full_history 衰减、working_set 平、archived_only 危险
- gate 矩阵：`evals/reports/eval-gate-metrics.json` — 捕获 1.00 / 误杀 0.00
- `sfai verify --fail-on-incomplete` CI 强制闸门自证通过

## 数据可信度附录（必须随论文披露）

1. 判分器三次事故与修正（Debug 报告 #13）：键名强校验 → 值匹配 → 数值比较 → 文件名无关 + 任务间清理；作废版本不入表
2. 网络事故：E9 HumanEval 两次作废（27 超时 + 37 DNS 失败）后改百炼补测完成
3. 并发扫描延迟噪声（e2m 双链并发）：跨模型延迟对比仅供参考；成功率/正确率/token 不受影响
4. n 范围 2–16/cell；订阅制计价（MiMo/百炼 price 0.00，比 token 不比美元）；合成 + 公开混合任务
