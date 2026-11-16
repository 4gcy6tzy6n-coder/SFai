# E7 — 跨框架真实端到端对比（sfai vs LangChain/LangGraph）

**日期:** 2026-11-16 · **模型:** deepseek-chat（双方同模型）· **n=6/suite**（3 任务 × 2 seeds）
**公平性契约:** 相同任务目标字符串、相同 CSV 数据（同一生成公式）、相同工具信息
（3 条 canned claims 带 source:// 引用 id）、框架无关判分器（答案值匹配 / ≥3 个可解析引用）。
LangChain 臂 = LangGraph ReAct + langchain-openai + file/search 工具（无验证门——框架原样）。
sfai 臂 = 完整内核（自路由 + 索引上下文 + **验证门 + 有界修复**）。

## 结果

| 指标 | LangChain (ReAct) | sfai | 判定 |
|---|---|---|---|
| dataset 成功率 | 0.83 | 0.83 | 平 |
| **dataset 正确率** | **0.17** | **0.83** | **sfai 5×** |
| **dataset 假成功** | **0.83** | **0.00** | **sfai 消除** |
| tool-use 成功/正确/假成功 | 1.00 / 1.00 / 0.00 | 1.00 / 1.00 / 0.00 | 平 |
| dataset p50 延迟 | **3.0s** | 7.6s | LangChain 快 |
| dataset 输入 token | **1918** | 6599 | LangChain 省 |
| dataset 成本 | **$0.0009** | $0.0029 | LangChain 省 |

## 诚实解读

1. **核心发现：验证门把"静默算错"变成"修复后正确"。** 同一个 deepseek-chat 算 CSV
   总和会出错；LangChain 臂把错误答案当成功交付（**假成功 0.83**），sfai 的验收门
   （`artifact_contains:<total>`）拦截错误 → 有界修复 → 正确率 0.83、假成功 **0.00**。
   代价：3.4× 输入 token、2.5× 延迟——**质量-成本权衡正是论文要的曲线**。
2. **LangChain 更快更省但交付不可信**：无验证层的框架把模型错误直接暴露给用户。
3. **tool-use 双方打平**（1.00/1.00/0.00）——该任务模型不易错，两框架都好；
   差距在**容错任务**上显现。这是框架差异的真实边界，写进论文。
4. **判分器修正记录（诚实）**：初版判分器键名强校验（要求 "total" 键）导致 LangChain
   臂假阴性（其智能体用 `total_revenue` 键、值是对的）；修正为**值匹配**后重跑，
   结论从"0.00 正确"修正为"0.17 正确"。第一版结论作废，本报告数字为修正后重跑值。
5. **sfai 侧路由遥测**（真实运行）：tool-use 14.3 步 / 4 误路由 / 3 loop-guard 拦截后恢复
   ——运行时守卫在真实模型上真实工作，且未阻断任务完成。

## 对论文的用法

- 表：同模型同任务跨框架对比（上表）+ "验证门价值"曲线（E1/E4/E2/E5-gate CI）。
- 叙事："agent 框架的差异不在模型能力，在于**错误是静默流出还是被拦截修复**；
  sfai 的验证门以 3.4× token 代价把假成功从 0.83 压到 0.00。"
- 必须标注：n=6、单 provider、合成任务；LangChain 臂无验证层是框架原样（非刻意削弱）。

## 复现

```bash
DEEPSEEK_API_KEY=... python3 experiments/langchain_baseline/langchain_agent.py 2
./target/release/sfai bench --config experiments/configs/deepseek-e7-headtohead.json \
  --out experiments/results/e7-sfai-headtohead.json
python3 scripts/compare_results.py experiments/results/e7-*.json
```
