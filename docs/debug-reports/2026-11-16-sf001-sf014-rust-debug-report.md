# Debug Report — SF-001…SF-014（Rust v0.1）

- **日期**: 2026-11-16
- **开发者 / 会话**: lead（agent dev session）
- **关联 issue**: SF-001…SF-014
- **关联 milestone**: M0–M3 core
- **判定**: **PASS**

## 1. 症状（Scope）

按开发计划 v0.5 的 Immediate Start Checklist 与 SF-001–SF-014 交付 Rust 版
sfai 核心（契约 / 环境 / 评测骨架 / 自路由内核 / 策略 Broker / 事件账本 /
恢复与幂等 / 上下文索引与工作集）。本报告记录开发与 Debug 环节中发现的全部
问题、修复与回归证据。

## 2. 复现步骤（Reproduction）

```bash
cargo test                       # 离线、无 API key
cargo clippy --all-targets        # 0 warnings
cargo fmt --all --check           # clean
cargo run -- run --fixture tool-use --storage /tmp/sfai-demo
```

## 3. 根因与修复（Root cause & Fix）

| # | 症状 | 复现证据 | 根因 | 修复 | 回归用例 |
|---|---|---|---|---|---|
| 1 | `context.search` 静默失败，检索不到证据 | SQLite 实验：`sparse-context policy` → `no such column: context` | FTS5 把未加引号的 `-` 当运算符/减法 | `EnvelopeBuilder` 对每个 OR 词加双引号；fixture 查询改为空格分隔；MATCH 失败回退 LIKE | `fixtures::context_retrieval_recovers_needle_and_keeps_constraints` |
| 2 | 引用抽取恒为空 → `citations_resolve` 误报失败 | 单测断言 `left: []` vs `right: ["source://src_abc@1", ...]` | `refs_in_text` 从整段文本扫描，前缀中的 `:` 立即截断候选 | 前缀之后开始扫描，再拼回 `source://`/`artifact://`；去掉尾部标点 | `ids::text_ref_tests::extracts_refs_from_markdown` |
| 3 | 崩溃恢复后 resume 报 `illegal status transition active -> active` | `storage::crash_mid_run_resumes_without_duplicates` panic | 内核无条件执行 `set_status(Active)` | 仅当状态不是 Active 时才转换 | 同上 |
| 4 | 测试查询 `tool_runs.error_code` 列不存在 | `Storage("no such column: error_code")` | schema 将 error_code 存于 `result_json`，无独立列 | 测试助手改为解析 `result_json` | `fixtures::approval_denial_has_no_side_effect` |
| 5 | 拒绝动作（未知操作 / 审批拒绝）未留痕 | 代码审查（修复前无 ToolRun/事件） | broker 早退分支直接返回，未 commit | 新增 `deny()`：提交 `ToolRun{status: denied}` + `tool.committed` 事件（含 error_code/receipt） | `security::unknown_operation_is_denied_not_executed`、`security::external_write_denied_without_approval` |
| 6 | OpenAI 兼容直答路径无法获得 task_id | `normalize_response` 对 `task_id=""` 抛错 | 直答消息本身不含 task_id | 从信封 root_index 注入 task_id，并回填 tool-call 参数 | `providers::openai_compat::tests::normalizes_direct_answer` |
| 7 | 预算超限直接抛异常中断运行 | 代码审查 | `check_budgets` 只 raise | 改为转换为可见 `blocked` 状态 + trace 落盘 | 预算路径由内核覆盖（`run_task` blocked 分支） |
| 8 | 错误开/闭事件 key 不一致，错误残留 | 代码审查 | 用 seq 推导 key，open/resolve 不匹配 | 使用确定性的 `tool:<operation_id>` key；replay 按 key 去重 | `fixtures::failed_tool_blocks_after_bounded_retry` |
| 9 | 编译期错误（非运行时缺陷，记录备查） | `cargo check` | `ToolStatus` 缺 `Default`；`PolicyEngine`/`ContextIndex` 缺 `Clone`；`fake_tools` 闭包重复 move | 补 derive、拆分 clone、修正关联函数调用 | 全量 `cargo test` |

## 4. 修复范围（Files）

`core/decision_kernel.rs`、`runtime/broker.rs`、`core/context_envelope.rs`、
`ids.rs`、`context/retrieval.rs`、`fixtures.rs`、`integrations/fake_tools.rs`、
`integrations/providers/{fake,openai_compat}.rs`、`tests/common/mod.rs`。

## 5. 回归验证（Regression）

| 检查 | 结果 |
|---|---|
| `cargo test` | **34 passed / 0 failed**（lib 19 + fixtures 5 + storage 4 + security 4 + e2e 2） |
| `cargo clippy --all-targets` | **0 warnings** |
| `cargo fmt --all --check` | clean |
| CLI demo `run --fixture tool-use` | `status=completed, steps=3, tool_calls=2, cost_usd≈0.0019`，trace 重放成本一致 |
| 事件回放一致性 | 同一存储上第二次打开的 App 得到完全相同的 `TaskState`（`events_replay_into_same_task_state`） |
| 幂等 | 相同 idempotency_key 只提交一次（`duplicate_idempotency_key_commits_once`） |
| 崩溃恢复 | kill 后 `resume()` 保留 pinned 约束、无重复写入（`crash_mid_run_resumes_without_duplicates`） |

### 对抗性检查

| 检查 | 结果 | 证据 |
|---|---|---|
| 直答不触发工具/发现调用 | 通过 | `broker.invocations` 为空、`registry.activations` 为空、tool_calls=0 |
| 外部写必须审批 | 通过 | `approval_denial_has_no_side_effect`、`external_write_denied_without_approval` |
| 未注册操作拒绝 | 通过 | `unknown_operation_is_denied_not_executed` |
| 提示注入不能提权 | 通过 | `injected_source_text_cannot_grant_permission` |
| 模型不能跨任务操作 | 通过 | `model_cannot_target_another_task`（事件与 ToolRun 绑定真实 task_id） |
| 无静默完成 | 通过 | 验收失败→open issue→bounded retry→`blocked`；失败工具同理 |

## 6. 遗留项（Known issues / Remaining）

1. **真实模型 HTTP 未实现**（M1 remaining）：`normalize_response` 已实现并离线测试，`OpenAICompatibleAdapter::decide` 显式返回 Provider 错误。
2. **多语句事务缺失**（ADR-013 #4）：事件追加是单条原子 INSERT，task 行与 created 事件分两次写入；跟进 issue：为 create/apply 增加 rusqlite 事务。
3. **ACL/多租户过滤未实现**（M7）：`context.search` 支持 task 范围，但无 tenant/user 过滤。
4. **FTS 回退语义较弱**：MATCH 语法错误时使用 LIKE 扫描；后续应做查询规范化。
5. **M4–M8 计划项未开始**：Wiki 编译器、Skill 包、真实能力适配器（浏览器/代码沙箱）、Web UI、稀疏索引学习。
6. **Python 语义参考已删除**（ADR-012 选项 b）：行为规范由本仓库测试套件与计划文档承载。

## 7. 判定（Verdict）

**PASS** — 强制 Debug 环节（复现 → 定位 → 根因 → 修复 → 回归 → 记录）闭环完成：

- 发现并修复 8 个运行时/接口缺陷 + 1 组编译期问题；
- 34 个离线测试全部通过，clippy 0 告警，格式检查通过；
- 安全、幂等、恢复、成本重放等不变量均有独立测试覆盖；
- 遗留项均为计划中 M1/M7/M8 范围或已登记跟进 issue，不阻塞 M0–M3 出口条件。
