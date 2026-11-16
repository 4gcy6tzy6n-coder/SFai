# Debug Report — M7 后端 HTTP API（v0.1 增量）

- **日期**: 2026-11-16
- **开发者 / 会话**: lead（agent dev session）
- **关联 milestone**: M7 后端部分（本地 HTTP API + SSE + 取消 + 线程安全化）
- **判定**: **PASS**

## 1. 症状（Scope）

为前端提供统一引擎接口：线程安全化 `App`（Arc/Mutex），新增 `sfai serve`
本地 API（任务 CRUD/运行/取消、事件轮询与 SSE、工件、trace、wiki、skills、
capabilities、health、可选 Bearer token、CORS），并保持既有 CLI UI 可用。
本报告记录该增量的 Debug 环节。

## 2. 复现步骤（Reproduction）

```bash
cargo test                        # 61 个离线测试
cargo clippy --all-targets         # 0 warnings
cargo fmt --all --check            # clean
cargo run -- serve --port 8765     # 真实服务冒烟
curl -s http://127.0.0.1:8765/api/health
curl -s -X POST http://127.0.0.1:8765/api/demo/direct-answer -d '{}' -H 'content-type: application/json'
```

## 3. 根因与修复（Root cause & Fix）

| # | 症状 | 复现证据 | 根因 | 修复 | 回归用例 |
|---|---|---|---|---|---|
| 1 | 线程安全化后 `ui.rs` 无法编译：`Cell<PermissionMode>` 不满足 `Sync` | `cargo check` E0277 | `CliApprovalProvider` 用 `Cell` 存权限模式 | 改为 `Mutex<PermissionMode>`（读加锁、写加锁） | 全量测试 + CLI UI 构建通过 |
| 2 | 既有测试编译失败：`Rc`/`RefCell` 的 `borrow()` 不存在于 `Arc`/`Mutex` | `cargo test` E0599/E0433 | 测试仍用旧并发类型 | 测试改用 `Arc` + `Mutex::lock` | 全部测试套件 |
| 3 | `server.rs` 编译失败：`spec.build_model()` 不存在 | `cargo check` E0599 | `build_model` 是字段闭包，不是方法 | 改为 `(spec.build_model)()` | 编译 + API 测试 |
| 4 | `TaskStore::list` 重复定义 | `cargo check` E0592 | 用户已添加同签名方法，我又加了一份 | 删除重复实现 | 编译 |
| 5 | 未知 fixture 的运行请求返回 500 | `api::error_mapping_and_validation` 期望 400 | `run_task` 在 `spawn_blocking` 内部才解析 fixture | 入口处先校验 fixture 名 → 400 | 同左 |
| 6 | 工件文本端点测试断言失效（解析 JSON 恒为空） | 初版测试用 JSON 解析文本 | HTTP 响应是 `text/markdown` 而非 JSON | 新增 `call_bytes` 断言文本含 `Citations` | `api::task_lifecycle_and_artifacts` |
| 7 | clippy 告警：`db.rs` 未使用的 `RefCell` 导入 | `cargo clippy` | 重构后遗留导入 | 删除导入 | clippy 0 告警 |

## 4. 修复范围（Files）

`storage/db.rs`、`authority/{approvals,tasks}.rs`、`capabilities/{registry,skills}.rs`、
`context/wiki.rs`、`runtime/broker.rs`、`app.rs`、`core/decision_kernel.rs`、
`integrations/fake_tools.rs`、`ui.rs`（仅 `Cell`→`Mutex`）、`bin/sfai.rs`（`Rc`→`Arc` 与 `serve`）、
`server.rs`（新增）、`tests/*`（并发类型迁移）、`tests/api.rs`（新增）。

## 5. 回归验证（Regression）

| 检查 | 结果 |
|---|---|
| `cargo test` | **61 passed / 0 failed**（lib 27 + e2e 3 + fixtures 5 + storage 4 + security 4 + skills 4 + wiki 5 + m6 5 + api 4） |
| `cargo clippy --all-targets` | **0 warnings** |
| `cargo fmt --all --check` | clean |
| 真实服务冒烟 | `/api/health` 返回 ok；`/api/demo/direct-answer` 返回 completed/steps=1；`/api/tasks` 可列出任务 |
| 认证 | 配置 `SFAI_API_TOKEN` 后 `/api/tasks` 无 token → 401；带 Bearer → 200；health 免认证 |
| 错误映射 | 未知任务 404；空 goal 400；未知 fixture 400；未运行任务 cancel 409 |
| 升级任务 | 已存在任务 `POST /run {"fixture":"direct-answer"}` → completed；原有 constraints 保留 |

### 对抗性检查

| 检查 | 结果 | 证据 |
|---|---|---|
| 重复运行拒绝 | 通过 | 每任务运行锁 → 409（代码路径） |
| 取消为协作式 | 通过 | 内核每步检查 cancel flag；取消后任务状态 `cancelled` |
| 线程安全 | 通过 | 编译期 `_assert_send_sync::<App>()`；Arc/Mutex 全链路 |
| 工件归属校验 | 通过 | 工件读取校验 `artifact.task_id == path task_id` |

## 6. 遗留项（Known issues / Remaining）

1. **SSE 为轮询式**（500ms），无推送通道（ADR-016）。
2. **单 SQLite 写锁**：并发请求串行化，多用户前需连接池。
3. **取消只在步骤边界生效**（工具执行中依赖工具超时）。
4. **ACL/多租户未实现**（M7 剩余项）。
5. **无速率限制**；CORS 对本地开发全开，非 loopback 部署前必须收紧。
6. **前端 UI 尚在开发中**（用户负责），后端接口已稳定可供对接。

## 7. 判定（Verdict）

**PASS** — M7 后端交付完成：61 个离线测试全部通过，clippy 0 告警，格式检查
干净，真实 `sfai serve` 冒烟成功；与用户正在开发的 CLI UI 无冲突（UI 测试与
原有能力全部保留）。
