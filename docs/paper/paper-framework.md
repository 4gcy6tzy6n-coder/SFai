# 论文框架 — Decide-Act: An Auditable Rust Agent Runtime with a False-Success Benchmark

**状态:** 框架定稿 v1（2026-11-16） · **数据来源:** `experiments/results/`（E1–E8）、`evals/reports/`
**定位:** 系统 + 实证（systems & measurement），双轨产出之一；产品即证据。

---

## 0. 标题候选

1. **Decide-Act: An Auditable Agent Runtime That Eliminates Silent Failures**
   — (副) with a Cross-Model Benchmark of 18 LLMs and Five Frameworks
2. Decide-Act: Model-Native Self-Routing with a Runtime Verification Gate
3. When Agents Say "Done": A Rust Runtime, a False-Success Benchmark, and
   the Cost of Trusting Unverified Completions

## 1. Abstract（草稿）

> Agent frameworks report task success — but who verifies the deliverable?
> We present **sfai**, a Rust agent runtime built around **Decide-Act**, a
> model-native self-routing loop (the model decides one action intent per
> step; the runtime acts it under policy, idempotency, loop guards, and a
> **runtime verification gate**). We further introduce a benchmark that
> measures what framework dashboards omit: **false success** — completed runs
> whose deliverables are wrong. Across **18 models** (DeepSeek, Qwen, GLM,
> MiMo families) and **four agent frameworks**, the verification gate holds
> false success at **0.00** (72/72 cells; Wilson 95% CI [0.00, 0.19], n=16/cell)
> where unverified runs average **0.324**, with zero false kills on good
> deliverables; the Decide-Act loop completes retrieval-heavy tasks a fixed
> pipeline cannot (1.00 vs 0.00); sparse context
> activation matches full-history quality at **1/78 of the input tokens** on
> 160-document histories; and the Rust kernel scales **5.1×** under 16-way
> CPU concurrency where Python threads flatten at 1.0× (GIL). The runtime,
> benchmark, and multi-layer memory (Index/Wiki/**Way**) are open source.

（最终数字以 Grand Tables 为准；CI/样本量已锁定口径。）

## 2. Claims → Evidence Map（三柱 + 系统）

| # | Claim | 证据 | 数据文件 | 强度 |
|---|---|---|---|---|
| C1 | **Decide-Act**（模型自路由）可行、真实被使用、且在不确定任务上决定性 | ask_user 0.94 @捏造压力、loop guard、E8：long-horizon 自路由 1.00 vs 固定管线 0.00（代价：流程化任务更贵） | e5-gate-ci, e2-mimo, **e8-routing** | ★★★★ |
| C2 | **验证门消除假成功**（0.38–1.00 → 0.00，零误杀） | 18 模型 × 4 suite、CI 分离、捕获/误杀矩阵 1.00/0.00 | e1/e2/e4/e5-gate/e2m-* | ★★★★ |
| C3 | **稀疏上下文激活**：同质量、成本不随规模涨 | 78× token 差 @160 docs、预算-质量平曲线、top-k 对照 | e5-sweep, e4 | ★★★★ |
| C4 | **系统级保证优于拼凑框架**（修正口径） | E7 v2：任务成功率接近，差异在**错误处理**——sfai 假成功钉 0.00（18 模型大样本佐证），LangGraph dataset fs 0.50；GIL 5.1×；内存 2.6× | e7-*v2, e6/e6b | ★★★☆ |
| C5 | **三层记忆 Index/Wiki/Way**（出错记忆自沉淀） | 架构 + 自写/自动提取测试；（真实收益待 v2 对照） | tests/memory, ADR-023 | ★★★☆ |

## 3. 章节结构

### 1 Introduction
- 问题：框架报"完成"但交付物错误（**silent failure / false success**）；上下文成本随规模线性涨；路由被固化成管线。
- 三个贡献：① Decide-Act 运行时（自路由 + 验证门 + 幂等/守卫/审计）；② false-success benchmark（时间/正确率/token/成本/消融，18 模型 × 4 框架实证）；③ 三层记忆（Index/Wiki/Way，方法与出错记忆）。
- 结果段：直接放 Grand Table 摘要（C2 的 0.00 vs 0.439 聚合 + C3 78× + C4）。

### 2 Related Work
| 方向 | 对照文献 | 我们的差异 |
|---|---|---|
| 记忆/上下文管理 | MemGPT/Letta、A-MEM、Mem0、SLIM、Context Folding (ICML'26)、压缩 (ACL'26) | 三层记忆中 **Way（方法/出错记忆）** 与验证门耦合；成本-规模曲线实证 |
| 验证/自纠错 | Verification-loop decomposition (arXiv 2607.17044)、CRepair、self-correction blind spot | **运行时强制**（非模型自查）；误杀率=0 的完整精度画像 |
| 框架与基准 | LangChain/LangGraph、smolagents、Pydantic AI、OpenAI Agents；GAIA/SWE-bench/Artificial Analysis | 首个带 **false-success + 消融** 维度的框架基准；同模型同判分器公平契约 |

### 3 The sfai Runtime
- **3.1 Decide-Act**：形式化 —— 每步 t，模型输出唯一动作意图 a_t ∈ A（9 类意图）；运行时 R 验证并执行（policy → broker → stores）；循环守卫（同动作×3 无状态变化→拒绝）；路由遥测 R(t)。给出伪代码 + 状态机。
- **3.2 三层记忆**：Index（寻址）/ Wiki（派生结构，承接）/ **Way**（方法+优点+**出错重点**，AI 自写自记）；检索路径 index→wiki→way；渐进披露进信封。
- **3.3 稀疏上下文激活**：budgeted working set（pinned 约束不驱逐、依赖一致、检索回执）。
- **3.4 验证门**：验收规则（artifact_exists/citations_resolve/json_valid/artifact_contains/no_placeholders/code_tests_pass）+ 有界修复；**假成功定义** fs = completed ∧ ¬(acceptance ∧ correct)。
- **3.5 安全**：策略分级、审批收据、幂等、审计事件。

### 4 Benchmark（Agent Framework Benchmark）
- 套件：tool-use / dataset / long-horizon / edge-case（捏造压力）；
- 指标：success、correctness（artifact-scoped ground truth）、**false-success**、p50/p95、in/out tokens、cost/success、routing quality（steps/tool calls/misroutes/loop-guard/ask_user）；
- 消融：none / full_context / no_pinned / no_verification / no_wiki / **forced_pipeline**；
- 公平契约：同模型、同任务字符串、同工具信息、同判分器（值匹配）、fresh storage/任务；
- 复现：`sfai bench --config …`，全部 JSON 归档。

### 5 Experiments
| 小节 | 内容 | 表/图 | 数据 |
|---|---|---|---|
| 5.1 E1 DeepSeek 基线 | 验证门对照（n=6） | T1 | e1-deepseek-baseline |
| 5.2 E4 消融 | full_context 2.8×、no_verification 0.75 fs | T2 | e4-deepseek-ablations |
| 5.3 E5 曲线 | 预算×规模 sweep + 门精度矩阵 | **F1 Pareto 曲线**、T3 | e5-sweep, gate-metrics |
| 5.4 E5-gate CI | n=16 CI 分离 + ask_user 0.94 | T4 | e5-gate-ci |
| 5.5 E2 跨模型 | 18 模型 Grand Table（含 MiMo） | **T5 主表** | e2m-* + e2-mimo + e1 |
| 5.6 E7 跨框架 | LangGraph/smolagents/Pydantic AI vs sfai | T6 | e7-*, e7ext-* |
| 5.7 E8 Decide-Act 消融 | 自路由 vs 强制管线（双向结果：long-horizon 1.00/0.00、dataset 0.25/0.75） | T7 | e8-routing |
| 5.6b E7 修正史 | 三次判分器事故与修正（附录材料） | 附录 | e7 系列 + Debug 报告 |
| 5.8 E6/E6b 性能 | 冷启动/大信封/GIL 5.1×/内存 | T8、F2 扩展曲线 | e6 系列 |

### 6 Analysis
- 验证门收益跨 18 模型零例外（聚合 fs 0.000 vs 0.439）；模型失败模式互补（DeepSeek 抗捏造、MiMo/Qwen 算得准）；
- 路由行为：ask_user 是压力下的真实自路由行为；误路由/loop-guard 首次量化；
- 成本-质量权衡：验证门 +3.4× token 换假成功归零——Pareto 叙事。

### 7 Limitations & Threats to Validity
- n 小（2–16/cell）、合成套件、单机延迟、订阅制计价（MiMo/百炼）、判分器修正记录（3 次，作废两版数字）、并发扫描延迟噪声、LangChain 臂无验证层为框架原样；
- E8 双向结果需如实呈现（自路由非处处优），主张限定为"不确定性任务上的适应性优势"。

### 8 Conclusion + Future Work
- Way 记忆的真实收益对照（v2）、公开基准（GAIA/SWE-bench）、预算 Pareto 扫描（E5 扩展）、多租户 ACL。

## 4. 图表清单（已锁定数据源）

| 编号 | 内容 | 来源 |
|---|---|---|
| F1 | 质量-成本 Pareto（预算扫描 × 策略） | evals/reports/e5-sweep.json |
| F2 | 并发吞吐（GIL 平线 vs Rust 5.1×） | E6b 数据 |
| T5 | **主表**：18 模型 × 4 suite（fs 门/无门） | scripts/grand_model_table.py |
| T6 | 跨框架对比 v2（修正口径，含判分器修正史） | e7 系列 v2 |
| T7 | Decide-Act 消融 | e8-routing |

## 5. 目标 venue
- 首选（系统/测量）：**OSDI / SOSP / ATC / EuroSys**（工程完整性 + 大规模实证）；
- 备选（ML/agents）：NeurIPS / ICLR datasets&benchmarks track；
- 行业轨：ACL/EMNLP Industry。

## 6. 写作顺序建议
1. §4 Benchmark（已有）→ 2. §5 表格（数据齐）→ 3. §3 方法（Decide-Act 形式化）→
4. §1/§2 → 5. §6/§7 → 6. Abstract/Title 定稿（数据冻结后）。
