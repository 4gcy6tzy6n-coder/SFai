# E8 — Decide-Act 消融：自路由 vs 固定管线（deepseek-chat，n=4/cell）

**配置:** 同模型、每步咨询次数相同（固定管线也每步咨询模型供内容）、仅路由被强制化。

| suite | 路由 | success | correct | in tokens | p50 | steps |
|---|---|---|---|---|---|---|
| long-horizon | **自路由 (Decide-Act)** | **1.00** | **1.00** | 21876 | 12.0s | 12.0 |
| long-horizon | 固定管线 | **0.00** | 0.00 | 12154 | 9.0s | 7.0 |
| dataset | 自路由 | 0.25 | 0.25 | 8338 | 9.6s | 7.2 |
| dataset | **固定管线** | **0.75** | **0.75** | 5184 | 5.7s | 4.5 |
| tool-use | 自路由 | 1.00 | 1.00 | 19023 | 23.8s | 13.5 |
| tool-use | 固定管线 | 1.00 | 1.00 | **4956** | **5.2s** | 4.0 |

## 解读（诚实的双向结果）

1. **Decide-Act 在需要适应的任务上是决定性的**：long-horizon 固定检索路线全灭
   （0.00），自路由 1.00——模型自主选择检索/读取顺序并从信封调整，是任务成功的原因。
2. **固定管线在流程化任务上更高效**：dataset 固定 read→write 路线 0.75 vs 自路由
   0.25（自路由消耗步数在多余动作上）；tool-use 打平但自路由花 4× token。
3. **结论不是"自路由处处更优"，而是"自路由的代价换来适应性"**：
   流程固定时固定管线赢效率，任务不确定时只有自路由能完成。论文以此为
   Decide-Act 的定位主张（route when uncertain, pipeline when certain），
   并指向 future work：混合路由（简单任务走固定子程序）。
4. 路由遥测佐证：自路由 7–13.5 步（含 2 误路由、4 幂等重放），固定 4–7 步无波动。

## 复现

```bash
./target/release/sfai bench --config experiments/configs/deepseek-e8-routing.json
```
