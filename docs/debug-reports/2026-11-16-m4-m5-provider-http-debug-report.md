# Debug Report — M4 / M5 / provider HTTP（v0.1 增量）

- **日期**: 2026-11-16
- **开发者 / 会话**: lead（agent dev session）
- **关联 milestone**: M4（增量 Wiki 编译器与溯源）、M5（动态能力与 Skill）、M1 剩余项（真实 provider HTTP）、成本计量补强
- **判定**: **PASS**

## 1. 症状（Scope）

在 M0–M3 核心之上交付：真实 OpenAI 兼容 provider HTTP 客户端、M4 增量 Wiki
编译器、M5 Skill 渐进披露，并修复成本计量缺口。本报告记录该增量的 Debug
环节（复现 → 定位 → 根因 → 修复 → 回归 → 记录）。

## 2. 复现步骤（Reproduction）

```bash
cargo test                       # 48 个离线测试，无 API key
cargo clippy --all-targets        # 0 warnings
cargo fmt --all --check           # clean
cargo run -- wiki compile <task_id>
cargo run -- skills list --skills skills
```

## 3. 根因与修复（Root cause & Fix）

| # | 症状 | 复现证据 | 根因 | 修复 | 回归用例 |
|---|---|---|---|---|---|
| 1 | `wiki.nodes_for` 报 `no such column: created_at` | `cargo test --test wiki` 3 个用例失败 | `wiki_nodes` 表只有 `updated_at`，查询按不存在的 `created_at` 排序 | 改为 `ORDER BY updated_at, node_id` | `wiki::repeated_compilation_does_not_duplicate_facts` 等 5 个 |
| 2 | 真实 HTTP 测试报 `Connection reset by peer` | `http_adapter_talks_to_mock_server` 失败 | mock 服务器只 `read()` 一次，body 分包到达时提前写响应并关连接 | 按 Content-Length 循环读到完整请求再响应 | `providers::openai_compat::http_adapter_talks_to_mock_server` |
| 3 | 真实 provider 成本恒为 0 | 代码审查：`normalize_response` 中 `cost_usd: 0.0` | 只取 token 数，未按价格表计价 | 归一化时按 `PRICE_PER_MILLION_*` 由 token 计算成本 | 单测断言 `usage.cost_usd > 0` |
| 4 | 编译错误：`capabilities::skills` 未声明、内核缺 `BTreeMap` 导入 | `cargo check` | lib.rs 未挂载新模块；内核被裁剪过导入 | 补模块声明与导入 | 全量构建 |
| 5 | clippy 告警（needless_borrow / unneeded Ok(? )） | `cargo clippy` | 冗余借用与多余包装 | 按建议简化 | clippy 0 告警 |

## 4. 修复范围（Files）

`context/wiki.rs`、`capabilities/skills.rs`、`core/context_envelope.rs`、
`core/decision_kernel.rs`、`app.rs`、`bin/sfai.rs`、
`integrations/providers/openai_compat.rs`、`contracts.rs`、`storage/db.rs`、
`tests/wiki.rs`、`tests/skills.rs`、示例 `skills/{research,writing}/SKILL.md`。

## 5. 回归验证（Regression）

| 检查 | 结果 |
|---|---|
| `cargo test` | **48 passed / 0 failed**（lib 24 + e2e 2 + fixtures 5 + storage 4 + security 4 + skills 4 + wiki 5） |
| `cargo clippy --all-targets` | **0 warnings** |
| `cargo fmt --all --check` | clean |
| M4 语义 | 重复编译不重复事实；同题冲突证据保留双方并加双向 `contradicts`；stale ref 标记而非丢弃；失败编译不推进游标且任务权威状态逐字段不变 |
| M5 语义 | 未激活 Skill 指令不进入信封（序列化断言）；激活后进入工作集且版本钉住；同一 run 可切换 research→writing；跨 run 不泄漏 |
| Provider HTTP | 本地 mock 服务器验证 POST /chat/completions、模型名、tool-call 归一化与 token 计价 |

### 对抗性检查

| 检查 | 结果 | 证据 |
|---|---|---|
| Wiki 失败不污染任务权威状态 | 通过 | `wiki::failed_compilation_cannot_corrupt_task_authority`（事件数、状态、resume 全等） |
| 非物化事件不触发编译 | 通过 | `wiki::non_material_events_are_not_compiled` |
| 未激活 Skill 不进提示词 | 通过 | `skills::inactive_skill_never_reaches_the_envelope` |
| 激活无越权 | 通过 | 执行仍走 `ToolBroker` 策略门（M5 测试中激活后仅 respond） |
| 未配置 provider 明确失败 | 通过 | `unconfigured_adapter_fails_loudly` |

## 6. 遗留项（Known issues / Remaining）

1. **真实托管模型端到端验收未做**：仅离线 fake 模型与本地 mock HTTP；`M1` 出口条件中"真实模型完成 search→analysis→artifact"仍需一次配置后的在线验证。
2. **Wiki 编译调度为 v0**：在任务结束时与 CLI 手动触发；异步/物化触发调度与 token 成本计量留待后续（ADR-014）。
3. **Skill 安装审批流未建**：`SkillTrust` 已记录但第三方包安装的显式批准流程属 M5 后续。
4. **ACL/多租户过滤未实现**（M7）：`wiki`/`search` 跨任务读取尚无 tenant/user 过滤。
5. **provider 计价为全局价格表**：按 token 计价，尚无按模型区分价格；可配置价格表待补。
6. **M7/M8 计划项未开始**：Web UI、并发/配额、学习式稀疏索引。

## 7. 判定（Verdict）

**PASS** — 增量（M4 + M5 + provider HTTP + 成本补强）已完成闭环：

- 48 个离线测试全部通过，clippy 0 告警，格式检查干净；
- M4/M5 出口条件均有独立测试证据；
- 遗留项均属后续里程碑或已登记跟进，不阻塞本增量交付。
