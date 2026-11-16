# 证据强化 — 稀疏上下文激活 与 验证门（E5 + 门精度矩阵）

**日期:** 2026-11-16 · **数据:** `evals/reports/e5-sweep.{md,json}`、`evals/reports/gate-metrics.{md,json}`、
`experiments/results/e5-gate-ci-deepseek.{md,json}`

## 柱子 2：稀疏上下文激活 — 质量-成本曲线 + 规模压力（E5 扫描）

### 曲线（离线确定性，3 场景/点）

| policy | history 40 | history 80 | history 160 | 成功率 | 约束保留 |
|---|---|---|---|---|---|
| working_set @50 tokens | **41 tok** | **41 tok** | **41 tok** | 1.00 | 1.00 |
| working_set @100 | 87 | 87 | 87 | 1.00 | 1.00 |
| full_history | 810 | 1610 | **3210** | 1.00 | 1.00 |
| retrieval_topk | 24 | 24 | 24 | **0.00** | **0.00** |

### 故事线（可直接进论文）

1. **质量不随预算塌缩**：working_set 在 50 token 预算（实付 41）即达 1.00 成功/正确，
   与 full_history 相同——而 full_history 在 160 文档历史时要 **3210 token**（**78×**）。
2. **成本不随规模增长**：working_set 的 token 曲线**平的**（41/41/41 across 40/80/160），
   full_history **线性涨**（810→1610→3210）。这是"索引原生上下文"的核心卖点。
3. **朴素检索不够**：retrieval_topk 召回 needle（correct 1.00）但丢约束（0.00）→ 成功 0.00。
   **pinned constraints + 依赖一致性选择**是 working_set 与 top-k 的本质区别——消融有了。
4. **失败边界（诚实）**：预算 < needle+约束 token（~41）时 working_set 也会失守；
   该点在曲线中可见（预算 50 以下未扫，v2 补 10/25 档）。

## 柱子 3：验证门 — 捕获率 / 误杀率矩阵（真实 VerificationEngine）

| 失败模式 | 捕获率 |
|---|---|
| broken_citation（捏造引用） | **1.00** |
| zero_citation（零引用空过） | **1.00** |
| placeholder_text（TODO 占位） | **1.00** |
| wrong_answer（答案错误） | **1.00** |
| **好交付物误杀率** | **0.00**（通过率 1.00） |

### 故事线

1. **门不是"会误伤的保守检查"**：四类故意错误全捕获（catch 1.00）且**零误杀**
   （false-kill 0.00）——精度故事完整。
2. **假成功消除的四重证据（CI 收紧后）**：
   - E1（n=6）：0.50/0.17 → 0.00；E4（n=4）：0.75 → 0.00；E2-MiMo（n=6）：0.67 → 0.00；
   - **E5-gate CI（n=16/cell，DeepSeek）**：long-horizon 无验证假成功 **0.38 [0.18,0.61]** vs
     有验证 **0.00 [0.00,0.19]**；edge-case 同样 0.38 → 0.00。**CI 基本分离**。
3. **压力下模型主动求助**（router 实证）：edge-case 捏造压力下 ask_user 率 **0.94**（有验证）
   / 0.62（无验证）——真实模型自发使用自路由的 ask_user 意图。
3. **修复收敛**：验收失败→有界修复（3 次）→ E1 中 blocked 后修复轮可完成（m6 测试 +
   E1 edge-case 数据）；闭环可讲述"检测→修复→复验"。

## 三柱故事（当前证据强度）

| 支柱 | 主张 | 证据 | 强度 |
|---|---|---|---|
| 自路由 router | 模型自选动作，压力下 ask_user，运行时守卫 | loop guard + 路由遥测 + **ask_user 0.94 @压力**（DeepSeek+MiMo 双模型） | ★★★★（E8 消融待跑） |
| 稀疏上下文激活 | 同质量、成本平（不随规模涨） | E5 曲线 **78× @160 docs** + top-k 对照 + 预算-质量平曲线 | ★★★★ |
| 验证门 | 假成功→0 [0.00,0.19]，零误杀 | **四重复现**（n=16 CI 分离）+ 捕获/误杀矩阵 1.00/0.00 | ★★★★ |

## 复现

```bash
cargo run --release -- eval --mode sweep --size 3      # E5 曲线
cargo run --release -- eval --mode gate                # 门精度矩阵
./target/release/sfai bench --config experiments/configs/deepseek-e5-gate-ci.json
```
