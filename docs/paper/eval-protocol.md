# A1 — Y/C/A 评价协议（Paper A 形式化核心）

## 1. 三元语义

对每个任务运行定义三个**独立**布尔变量：

| 变量 | 定义 | 数据来源 |
|---|---|---|
| **Y**（真实成功） | **独立环境真值**判定交付物正确（修复结束后的实际状态重新打分，非对同一输出复评） | `correct`（ground-truth 判分，独立于模型自评与门） |
| **C**（声称成功） | Agent 自己声明任务完成（处理过 Respond/Finish 意图） | `claimed`（kernel 记录；历史数据以 status 近似） |
| **A**（放行成功） | 验证门允许最终报告成功 | `status == completed`（门放行） |

**关键约束**：Y 必须由独立于被测系统的判分器给出，且判分器必须先过
**标准答案正向自检**（canonical solution 必须满分，P0 审计方法论）；评分文件必须
对 agent **只读**（protected_files，防自评作弊）。

## 2. 六指标（全部任务级比率 + Wilson 95% CI）

| 指标 | 公式 | 含义 |
|---|---|---|
| True Success | P(Y=1) | 真实任务完成能力 |
| Claim False Success | P(C=1, Y=0) | 原始错误声称率（门介入前） |
| Delivered False Success | P(A=1, Y=0) | 最终错误交付率（门介入后） |
| Accepted Error Rate | P(Y=0 \| A=1) | 交付可信度（假发现率） |
| Valid Success Recall | P(A=1 \| Y=1) | 正确任务被放行的比例（覆盖率） |
| False Rejection | P(A=0 \| Y=1) | 真实成功被误拦的比例 |

**派生量**：
- 门拦截贡献 = Claim FS − Delivered FS
- 风险–覆盖权衡 = (Accepted Error Rate, Valid Success Recall) 二维图
  （横轴覆盖率、纵轴错误接受风险；比较机制在**同覆盖率**下的风险与额外计算）

## 3. 统计规则（防审稿人三板斧）

1. **配对比较**用 McNemar 精确检验（同任务跨消融配对）；**禁止**用 Wilson CI
   "相交/不相交"做推断（CI 只作区间描述）；
2. **平凡零标注**：任何零值必须与分母联报（claims=0 或 accepted=0 的零标记 ⚠️
   不作证据——HumanEval 教训）；
3. **任务差异 vs 采样重复分离**：同一任务的多次采样是重复测量，不得当作独立任务
   汇总 CI；跨任务 CI 用任务级聚合；
4. 双分母披露：Delivered FS 以全部运行为分母、Accepted Error Rate 以放行为分母
   （主表两者并列）。

## 4. 实现与产物

- 指标引擎：`scripts/eval_protocol.py`（输入 bench 任务级 JSON，输出六指标表 + CI + 平凡零标记）
- 配对检验：`scripts/stats_tests.py`（McNemar 精确）
- 数据要求：`claimed` 字段（bench v0.2.2+，kernel 记录 Respond/Finish 处理）
- 当前结果：`docs/paper/eval-protocol-metrics.md`（冻结数据全量应用）
