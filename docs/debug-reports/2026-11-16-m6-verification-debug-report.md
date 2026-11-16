# Debug Report — M6 独立验证与调试强化（v0.1 增量）

- **日期**: 2026-11-16
- **开发者 / 会话**: lead（agent dev session）
- **关联 milestone**: M6（Independent Verify & Debug）
- **判定**: **PASS**

## 1. 症状（Scope）

在既有验证骨架之上交付：工作区代码测试（`code_tests_pass`）、JSON/占位符
文档检查、风险触发的强制验证边界、验证事件与状态、有界修复重试与可见
部分失败；本报告记录该增量的 Debug 环节。

## 2. 复现步骤（Reproduction）

```bash
cargo test                       # 53 个离线测试，无 API key
cargo clippy --all-targets        # 0 warnings
cargo fmt --all --check           # clean
```

## 3. 根因与修复（Root cause & Fix）

| # | 症状 | 复现证据 | 根因 | 修复 | 回归用例 |
|---|---|---|---|---|---|
| 1 | 外部写成功后不验证也能完成任务（强制验证未触发） | `m6::consequential_actions_require_a_later_verification` 期望 Blocked 实得 Completed | broker 的 `tool.committed` 事件 payload 未带 `risk_class`（补丁未命中实际代码位置） | 在事件 payload 写入 `risk_class`，内核据此判定最后的风险动作后是否有验证 | 同左 |
| 2 | 验收失败后"修复"仍被判定失败，修复轮被耗尽 | `m6::acceptance_failures_are_visible_and_bounded` 修复轮 `scripted model exhausted` | `put_artifact` 的 `version` 恒为 1，`ORDER BY version DESC` 无法确定最新工件；旧草稿仍被检查 | 工件版本按 `(task, path)` 递增；`citations_resolve` 只检查每个路径的最新版本 | 同左 |
| 3 | 同一任务的过期草稿污染引用检查 | 同上（症状 2 的直接原因） | `verify_citations` 遍历全部工件版本 | 按路径去重保留最高版本 | 同左 |
| 4 | 测试脚本步数不足导致模型耗尽 | `m6::consequential_actions_require_a_later_verification` 第二次失败信息 | 测试模型脚本只给了 1 个 finish，而有界重试需要 3 次 | 调整测试脚本（非产品缺陷，记录备查） | 同左 |

## 4. 修复范围（Files）

`runtime/broker.rs`、`verify/engine.rs`、`storage/objects.rs`、
`core/decision_kernel.rs`、`authority/tasks.rs`、`storage/event_ledger.rs`、
`context/wiki.rs`、`capabilities/registry.rs`、`app.rs`、`tests/m6.rs`。

## 5. 回归验证（Regression）

| 检查 | 结果 |
|---|---|
| `cargo test` | **53 passed / 0 failed**（lib 24 + e2e 2 + fixtures 5 + storage 4 + security 4 + skills 4 + wiki 5 + m6 5） |
| `cargo clippy --all-targets` | **0 warnings** |
| `cargo fmt --all --check` | clean |
| 工作区代码测试 | 通过命令退出 0/非 0 判定；失败诊断 `CODE_TESTS_FAILED` 携带输出片段 |
| 文档检查 | `json_valid` / `no_placeholders` 对故意错误（坏 JSON、TODO 占位）均判 Fail 并给出可执行修复提示 |
| 强制验证边界 | `external_write`/`high_impact` 后无验证 → `MANDATORY_VERIFICATION_MISSING`，重试后有验证 → Completed |
| 有界修复 | 验收失败最多 3 次尝试后 `blocked`，open issues 可见（含 `PLACEHOLDER_TEXT_FOUND`、`BROKEN_CITATION_REF`） |
| Wiki 验证边 | `verification.recorded` → verification 节点 + `verified_by` 边 |

### 对抗性检查

| 检查 | 结果 | 证据 |
|---|---|---|
| 故意错误代码被检出 | 通过 | `m6::code_checks_run_inside_the_task_workspace` |
| 故意错误文档被检出 | 通过 | `m6::json_and_placeholder_checks_detect_deliberate_errors` |
| 坏引用被检出且可修复 | 通过 | `m6::acceptance_failures_are_visible_and_bounded`（修复后完成） |
| 无静默完成 | 通过 | 强制验证门控与验收门控均有 blocked 路径测试 |
| 修复轮不产生幽灵成功 | 通过 | 修复后才 Completed，blocked 状态保留 open issues |

## 6. 遗留项（Known issues / Remaining）

1. **无 OS 级沙箱**：`code_tests_pass` 是工作区内的本地子进程，尚未有容器/命名空间隔离（M7）。
2. **超时只杀直接子进程**：`sh` 的孙进程可能存活；跟进：进程组 + 组杀。
3. **超大输出死锁风险**：stdout/stderr 在退出后读取，管道填满可能阻塞；跟进：带大小上限的流式读取。
4. **验证成本未单独计价**：验证运行次数已计数，代码测试耗时未单独计费。
5. **ACL 未实现**（M7）：工作区与 wiki/search 读取尚无 tenant/user 过滤。

## 7. 判定（Verdict）

**PASS** — M6 出口条件（代码/引用/文档中的故意错误被检出、失败产生可行动
证据、不静默完成）均有独立测试证据；53 个离线测试、clippy 0 告警、格式检查
通过；遗留项均为后续里程碑或已登记跟进。
