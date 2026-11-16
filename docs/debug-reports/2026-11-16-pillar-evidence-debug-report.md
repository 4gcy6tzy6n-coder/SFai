# Debug Report — 证据强化：稀疏上下文激活 + 验证门（E5 + 门精度）

- **日期**: 2026-11-16
- **关联**: 故事三柱之二（稀疏上下文激活、验证门消除假成功）
- **判定**: **PASS**（离线证据齐备；E5-gate CI 真实跑见 §4）

## 1. 症状（Scope）

两根柱子此前只有单点数据（2.8× token、0.50→0.00）。补强为：
① E5 预算×规模扫描（质量-成本-规模曲线，含 top-k 对照）；
② 门精度矩阵（4 类失败模式捕获率 + 好交付物误杀率）；
③ 真实模型 CI 收紧（n=16/cell）。

## 2. 根因与修复

| # | 症状 | 根因 | 修复 | 回归 |
|---|---|---|---|---|
| 1 | eval.rs 缺 BTreeMap 导入 / 索引类型歧义 | 新代码遗漏 | 补导入、显式解引用 `*mode` | 编译 + `tests/eval.rs` |
| 2 | clippy type_complexity | 复杂元组 Vec | 类型别名 `ModeSpec` | clippy 0 |
| 3 | gate 报告 .md 未落盘 | CLI stdout 未重定向 | 重新生成保存 | 文件归档 |
| 4 | retrieval_topk 成功恒 0（曲线里的对照组） | 设计如此：top-k 不 pin 约束 | 保留为消融对照（这正是故事点 3） | `eval` 测试断言 |

## 3. 离线证据（确定性、可复现）

**E5 扫描**（3 场景/点，40/80/160 历史 × 7 档预算 × 3 策略）：
- working_set @50 token：成功/正确/约束 1.00，实付 41 token，**跨 40→160 文档不变**；
- full_history：810→1610→**3210** token（线性增长，@160 是 working_set 的 **78×**）；
- retrieval_topk：needle 召回 1.00 但约束 0.00 → 成功 0.00（pinned 约束的消融证据）。

**门精度矩阵**（真实 VerificationEngine，5 模式 × 4 实例）：
- 捕获率：broken_citation / zero_citation / placeholder_text / wrong_answer 全部 **1.00**；
- 好交付物误杀率 **0.00**（通过率 1.00）。

## 4. E5-gate CI 真实跑（DeepSeek，n=16/cell）

`experiments/configs/deepseek-e5-gate-ci.json`（long-horizon + edge-case ×
none/no_verification × 4 任务 × 4 重复 = 64 运行）执行中，结果落盘后并入
`experiments/analysis/pillar-evidence-strengthening.md`。

## 5. 回归验证

- `cargo test --release`：**99 passed / 0 failed**（新增 `tests/eval.rs` 2 个：
  sweep 覆盖预算/规模、门全捕获零误杀）
- clippy 0 告警；fmt clean

## 6. 遗留项

1. working_set 失守边界（预算 <41 token）未扫（v2 补 10/25 档）。
2. 门矩阵的失败模式为构造样本；真实模型坏交付物的捕获由 E1/E2/E4 真实数据支撑。
3. 修复收敛率（blocked→修复→完成）建议从 E5-gate CI trace 提取（下一步）。

## 7. 判定

**PASS** — 两根柱子从单点数据升级为**曲线 + 率值矩阵**，故事可讲：
"同质量、成本平（78×@160 docs）+ 门全捕获零误杀 + 三重复现"。
