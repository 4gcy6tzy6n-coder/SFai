# E10 — 等验证器对等实验（回应导师风险 #3：验证门的独特性）

**设计:** 同 12 任务（public-e10parity，与框架臂同 goal/同数据/同判分器）、同模型
（百炼 deepseek-v4.1-flash）、同修复预算（2 轮 = sfai MAX_ACCEPTANCE_RETRIES）、
同执行权限（同工具面）。三方对照：

| 臂 | 完成率 | 正确率 | fs/全部 | fs/声称成功 | 修复触发 | 失败性质 |
|---|---|---|---|---|---|---|
| **sfai（门+运行时）** | **1.00** (12/12) | **1.00** | 0.00 | 0.00 | 0 | 无 |
| LangGraph 无门 | 0.83 (10/12) | 0.83 | 0.00 | 0.00 | — | 2 次裸异常 |
| LangGraph + 等验证器 | 0.92 (11/12) | 0.92 | 0.00 | 0.00 | 0 | 1 次裸异常→门阻断 |

## 诚实解读（正面回应风险 #3）

1. **在低错误任务上，框架身份不重要**：该 flash 模型在这些简单任务上"声称即正确"
   （两臂 fs=0，无门也是 0）——验证门的价值在本组**未被触发**（0 次修复触发：
   没有假声称可修）。这正是评审预判的情形，如实呈现。
2. **验证门的价值是条件性的**：随模型错误率放大。错误率高的场景（18 模型平均 fs
   0.324、HumanEval 声称成功 100% 是假的）门把 fs 归零；错误率低的场景（本组）
   门中性。论文必须如此限定主张（"有效验证是活性成分"）。
3. **Decide-Act 的独特价值在此组体现为运行时鲁棒性**：LangGraph 裸 invoke 两次
   未捕获异常直接丢任务（10/12、11/12），sfai 运行时（重试/有界恢复/事件账本）
   12/12 全完成；另有 E8 适应性（long-horizon 1.00 vs 0.00）、审计、记忆、稀疏激活。
4. **对照真实性**：修复预算（2 轮）、验收规则（artifact_exists/json_valid/
   artifact_contains/source_count_at_least 语义一致）、判分器（数值/≥3 引用）
   全部对齐；任务集三方同源（tasks.json 冻结）。

## 对论文主张的修正（收紧）

- C2（门消除假成功）→ 改为"**有效验证**消除假成功（18 模型/公开基准实证），收益随
  模型错误率放大；零假成功必须与完成率联报"（双分母 + 完成率表）。
- C1（Decide-Act）→ 独特性主张限定于：审计账本、自路由适应性（E8 双向）、运行时
  鲁棒性（E10 12/12）、三层记忆；不再隐含"门效果归于框架"。
- Limitations 新增："低错误率场景下 LangGraph+等验证器可比"（本组实证）。

## 复现

```bash
sfai bench --config experiments/configs/e10-parity-sfai.json
SFAI_E10_BASE_URL=... SFAI_E10_API_KEY=... python3 experiments/frameworks/e10_parity.py nogate
SFAI_E10_BASE_URL=... SFAI_E10_API_KEY=... python3 experiments/frameworks/e10_parity.py gate
```
