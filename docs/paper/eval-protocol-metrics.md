# Y/C/A 六指标评价协议 — 应用于冻结数据（Paper A 表 1 草稿）

**语义**: Y=独立真值(交付物正确) · C=模型声称完成 · A=验证门放行(最终 completed)
**口径**: 全部为任务级比率 + Wilson 95% CI；**平凡零标记**：当声称/放行为 0 时零值不作证据
**旧数据注**: 无 claimed 字段的历史行以 status 代理 C（协议 v1 起新跑均有精确 claimed）

## e1-deepseek-baseline（deepseek/deepseek-chat）

| 消融 | n | P(Y=1) True Success | P(C=1,Y=0) Claim FS | P(A=1,Y=0) Delivered FS | P(Y=0\|A=1) Accepted Err | P(A=1\|Y=1) Valid Recall | P(A=0\|Y=1) False Rej | 平凡零 |
|---|---|---|---|---|---|---|---|---|
| no_verification | 24 | 0.46 [0.28,0.65] | 0.17 [0.07,0.36] | 0.17 [0.07,0.36] | 0.27 [0.11,0.52] | 1.00 [0.74,1.00] | 0.00 [0.00,0.26] | — |
| none | 24 | 0.62 [0.43,0.79] | 0.00 [0.00,0.14] | 0.00 [0.00,0.14] | 0.00 [0.00,0.22] | 0.93 [0.70,0.99] | 0.07 [0.01,0.30] | — |

## e5-gate-ci-deepseek（deepseek/deepseek-chat）

| 消融 | n | P(Y=1) True Success | P(C=1,Y=0) Claim FS | P(A=1,Y=0) Delivered FS | P(Y=0\|A=1) Accepted Err | P(A=1\|Y=1) Valid Recall | P(A=0\|Y=1) False Rej | 平凡零 |
|---|---|---|---|---|---|---|---|---|
| no_verification | 32 | 0.31 [0.18,0.49] | 0.38 [0.23,0.55] | 0.38 [0.23,0.55] | 0.55 [0.35,0.73] | 1.00 [0.72,1.00] | 0.00 [0.00,0.28] | — |
| none | 32 | 0.50 [0.34,0.66] | 0.00 [0.00,0.11] | 0.00 [0.00,0.11] | 0.00 [0.00,0.20] | 0.94 [0.72,0.99] | 0.06 [0.01,0.28] | — |

## e9-public-deepseek（deepseek/deepseek-chat）

| 消融 | n | P(Y=1) True Success | P(C=1,Y=0) Claim FS | P(A=1,Y=0) Delivered FS | P(Y=0\|A=1) Accepted Err | P(A=1\|Y=1) Valid Recall | P(A=0\|Y=1) False Rej | 平凡零 |
|---|---|---|---|---|---|---|---|---|
| no_verification | 40 | 0.35 [0.22,0.50] | 0.12 [0.05,0.26] | 0.12 [0.05,0.26] | 0.26 [0.12,0.49] | 1.00 [0.78,1.00] | 0.00 [0.00,0.22] | — |
| none | 40 | 0.47 [0.33,0.63] | 0.00 [0.00,0.09] | 0.00 [0.00,0.09] | 0.00 [0.00,0.17] | 1.00 [0.83,1.00] | 0.00 [0.00,0.17] | — |

## e9-public-sheets-bailian（bailian/deepseek-v4.1-flash）

| 消融 | n | P(Y=1) True Success | P(C=1,Y=0) Claim FS | P(A=1,Y=0) Delivered FS | P(Y=0\|A=1) Accepted Err | P(A=1\|Y=1) Valid Recall | P(A=0\|Y=1) False Rej | 平凡零 |
|---|---|---|---|---|---|---|---|---|
| no_verification | 20 | 0.15 [0.05,0.36] | 0.00 [0.00,0.16] | 0.00 [0.00,0.16] | 0.00 [0.00,0.56] | 1.00 [0.44,1.00] | 0.00 [0.00,0.56] | — |
| none | 20 | 0.15 [0.05,0.36] | 0.00 [0.00,0.16] | 0.00 [0.00,0.16] | 0.00 [0.00,0.56] | 1.00 [0.44,1.00] | 0.00 [0.00,0.56] | — |

## e10-parity-sfai（bailian/deepseek-v4.1-flash）

| 消融 | n | P(Y=1) True Success | P(C=1,Y=0) Claim FS | P(A=1,Y=0) Delivered FS | P(Y=0\|A=1) Accepted Err | P(A=1\|Y=1) Valid Recall | P(A=0\|Y=1) False Rej | 平凡零 |
|---|---|---|---|---|---|---|---|---|
| none | 12 | 1.00 [0.76,1.00] | 0.00 [0.00,0.24] | 0.00 [0.00,0.24] | 0.00 [0.00,0.24] | 1.00 [0.76,1.00] | 0.00 [0.00,0.24] | — |

## 解读规则（写作时执行）

1. **零假成功必须与 P(A=1) 联读**：accepted=0 的零是平凡零（HumanEval 教训）；
2. Claim FS 与 Delivered FS 的差 = 门的拦截贡献；
3. Accepted Error Rate 是'交付可信度'（False Discovery）；
4. Valid Success Recall + False Rejection = 门的覆盖率代价（风险-覆盖权衡图轴）；
5. 配对比较用 McNemar（scripts/stats_tests.py），不做 CI 相交推断。
