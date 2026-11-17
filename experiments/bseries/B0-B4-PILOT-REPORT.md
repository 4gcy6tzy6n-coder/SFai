# B0–B4 试点报告（交接框架团队）

**日期:** 2026-11-17 · **判定:** 流水线验证通过；试点任务存在天花板效应

## 结果（12 任务，百炼 deepseek-v4.1-flash，同基线运行完美配对）

- B0–B4 全臂：True Success 1.00、Delivered FS 0.00；
- 判分一致性：B1 确定性门 / B3 LLM 自检 / B4 独立判官 **均 12/12 与真值一致**（误放 0、误杀 0）；
- **流水线机制可信**（判分正确性经真值核验）。

## 两个工程修复（复现价值高）

1. `SFAI_BENCH_PERSIST=1`：bench 工作区持久化（同一批运行做多种后验判分）；
2. **交付物在对象库不在目录**：judge 必须从 `sfai.sqlite3 + blobs/` 物化
   （b0_b4_pipeline.materialize_from_store）——首轮 0/12 一致性即此假象。

## 实验设计结论（给框架团队的建议）

1. **天花板效应**：e10parity × flash 全对 → 观察不到门/自检的区分度。主实验须用
   **易错条件**：AppWorld 多步任务（A5 在建）、弱模型、或注入扰动（B5 故障注入）；
2. **判分一致性是核心量**：B3 自检 vs B4 判官 vs B1 确定性的"误放/误杀"差异，需要
   在错误样本丰富时才可分——这是 E11 的真实看点；
3. 建议主跑：AppWorld 20 任务 × 3 模型 × B0–B4（Y=AppWorld 官方状态判分器，
   完全独立真值）。

## 交接材料

- 流水线：`experiments/bseries/b0_b4_pipeline.py`（含对象库物化）
- 基线数据：`experiments/results/bseries-pilot.json` + `/tmp/bseries-ws`（24 个持久 app root）
- 修复已入 SFai 仓；本报告同步 decide-act 仓
