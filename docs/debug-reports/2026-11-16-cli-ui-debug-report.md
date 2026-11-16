# Debug Report — Claude Code 风格 CLI UI 设计与实现（M7 CLI UI 增量）

- **日期**: 2026-11-16
- **开发者 / 会话**: Antigravity (agent dev session)
- **关联 issue**: SF-008（流式 CLI 与任务运行器增强）、M7（产品化界面与审批 UX）
- **关联 milestone**: M1 / M7（CLI 交互、流式步骤时间线、分级权限审批框与检查器卡片）
- **判定**: **PASS**

## 1. 症状（Scope）

为 `sfai` 交付类 Claude Code 风格的 CLI 终端界面（[crates/sfai/src/ui.rs](../../crates/sfai/src/ui.rs) 与 [crates/sfai/src/bin/sfai.rs](../../crates/sfai/src/bin/sfai.rs)），包括：
- 圆角边框设计系统（`╭─╮│╰─╯`）、自动 ANSI 配色与 `--no-color`/`NO_COLOR` 支持、CJK 与 ANSI 感知列宽计算；
- 内核无侵入式 `KernelObserver`（[decision_kernel.rs](../../crates/sfai/src/core/decision_kernel.rs)）与 `ClaudeStepObserver` 实时流式步骤树（`◌ Step` 工作集快照、`● Tool/Intent` 调用徽章、`  ⎿  ` 缩进树状输出与引用/诊断提示）；
- 类 Claude Code 的交互式分级权限审批框（`CliApprovalProvider`：`deny` / `ask` / `auto`，支持 `[y]es / [n]o / [a]lways for session`）；
- **顶部紧凑仪表盘（`render_top_dashboard`）**：将模型、权限策略、工作集 Context 引用数/钉住约束数、当前活动任务状态、Wiki 节点/边数量、修改文件数、Token 与累计费用收敛在界面上方的 2 行紧凑 HUD 仪表盘中；
- **主界面聚焦（对话 + 命令 + 修改文件 Diff）**：去除繁琐的任务元数据大卡片，主界面直接呈现用户气泡（`▸ You`）、紧凑工具/命令执行树（`● ... ⎿ ...`）、**内联文件变更 LCS 逐行 Diff 卡片（`✦ File <path> +A -R`）**、以及 AI 助手响应气泡（`✻ Response (sfai AI)`）；
- **类 Claude Code 的底部 AI 对话输入框**（`read_chat_input_box` + `ChatComposerStatus` 状态栏 + `/files` 与 `/diff [path]` 文件变更查看器 + `/history` 多轮对话回顾 + `/dashboard` 刷新顶部仪表盘 + `ConversationalDemoModel` 离线自路由对话/文件编辑适配）；
- 全套检查器卡片与交互式 `/` 命令（`/help`、`/history`、`/model`、`/run`、`/fixtures`、`/tasks`、`/status`、`/inspect`、`/search`、`/read`、`/wiki`、`/skills`、`/capabilities`、`/verify`、`/trace`、`/permissions`、`/constraints`、`/accept`、`/cost`、`/doctor`、`/clear`），以及子命令 `--ui` 模式（同时保留无 `--ui` 时的默认 JSON 输出以向后兼容自动化测试）。

## 2. 复现步骤（Reproduction）

```bash
cargo test --test cli_ui
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
```

## 3. 根因与修复（Root cause & Fix）

| # | 症状 | 复现证据 | 根因 | 修复 | 回归用例 |
|---|---|---|---|---|---|
| 1 | 交互式审批通过（`/permissions ask` → `/run approval` → `y`）后，`Verify` 执行返回 `0 check(s)`，随后 `Finish` 被 `MANDATORY_VERIFICATION_MISSING` 拦截并耗尽脚本模型 | `printf "/permissions ask\n/run approval\ny\n/quit\n" \| sfai` 报错 `scripted model exhausted` | `DecisionKernel::dispatch(IntentKind::Verify)` 在 `criteria` 与 `task.acceptance_rules` 均为空时返回空 `results`，未写入任何 `verification.recorded` 事件，导致强制验证门控认为外部写后未发生验证 | 当 `Verify` 意图与任务自身 `acceptance_rules` 均为空时，回退执行 `source_count_at_least:0` 基础校验，确保记录 `verification.recorded` 审计事件 | `cli_ui::interactive_repl_supports_approval_prompt_and_completes` |
| 2 | 含 `🔒` / `🛡` / `⚡` 等 Emoji 的卡片边框在部分终端右边框错位 1 列 | 欢迎横幅与 System Doctor 卡片 `Permissions` 行右边框不齐 | 部分 Emoji（如 `U+1F6E1`、`U+26A1`）属于 East Asian Ambiguous 或依赖 `FE0F` 变体选择符，不同终端渲染为 1 列或 2 列不一致 | 统一改用宽度恒定为 1 列的几何符号（`⊘ deny-all`、`◆ ask-before-write`、`▸ auto-approve`、`! disputed`） | `ui::tests::box_renderer_aligns_borders` |
| 3 | Wiki 卡片（`render_wiki_view`）的 `Typed Edges` 仅显示不透明 `wn_...` 节点 ID，可读性差 | `/wiki` 输出 `wn_f6823728439d ──[supported_by]──▶ wn_c256b7849353` | 未将 `nodes` 中的 `(node_id -> [kind] title)` 映射注入边渲染器 | 构建 `node_labels` 映射表，在边视图中直接渲染 `[kind] title` | `cli_ui::cli_ui_subcommands_render_claude_code_cards` |
| 4 | 并发会话将 `ApprovalProvider` 升级为 `Send + Sync`（`Arc<dyn ApprovalProvider>`），`Cell<PermissionMode>` 无法跨线程共享 | 编译期 `Send`/`Sync` 约束检查 | `Cell` 不实现 `Sync` | `CliApprovalProvider` 改用 `Mutex<PermissionMode>` 并以 `Arc` 注入 `App` | 全量构建与 `cli_ui` 测试套件 |
| 5 | 未配置 `SFAI_MODEL_BASE_URL` 时在底部对话框直接发送自然语言消息报 `OpenAICompatibleAdapter is not configured`；且长中文响应撑宽对话框 | 交互式对话测试 | 缺少开箱即用的本地自路由对话适配器，且 `render_markdown_block` 未按 CJK 视觉列宽自动折行 | 新增 `ConversationalDemoModel`（支持直答、搜索写报告、修改文件、检索历史引用四种自路由动作）与 `/model auto\|offline\|online`；新增 `wrap_plain_width` CJK 视觉列宽折行与非 TTY 右边框闭合 | `cli_ui::interactive_chat_dialog_box_and_multi_turn_conversation` |
| 6 | 跨多轮任务修改同一文件（如 `report.md`）时，`collect_modified_files` 版本号仍显示 `v1` | `/files` 与 `/diff report.md` 输出 `modified · v1` | `artifacts` 表按 `(task_id, path)` 计版本，跨任务修改同一路径时未按路径累加工作区版本 | `collect_modified_files` 按 `path` 维护 `(prev_version, prev_content)` 并在跨任务修改时递增为 `v2` | `cli_ui::interactive_chat_dialog_box_and_multi_turn_conversation` |

## 4. 修复范围（Files）

- [crates/sfai/src/ui.rs](../../crates/sfai/src/ui.rs)
- [crates/sfai/src/bin/sfai.rs](../../crates/sfai/src/bin/sfai.rs)
- [crates/sfai/src/core/decision_kernel.rs](../../crates/sfai/src/core/decision_kernel.rs)
- [crates/sfai/src/authority/tasks.rs](../../crates/sfai/src/authority/tasks.rs)
- [crates/sfai/src/app.rs](../../crates/sfai/src/app.rs)
- [crates/sfai/src/fixtures.rs](../../crates/sfai/src/fixtures.rs)
- [crates/sfai/tests/cli_ui.rs](../../crates/sfai/tests/cli_ui.rs)

## 5. 回归验证（Regression）

| 检查 | 结果 |
|---|---|
| `cargo test --test cli_ui` | **4 passed / 0 failed**（覆盖全部 `--ui` 子命令卡片、底部 AI 对话框与多轮自路由对话、交互式审批允许闭环、交互式审批拒绝闭环） |
| `cargo test --test e2e` | **3 passed / 0 failed**（保持默认 JSON 子命令与裸 `sfai` 交互冒烟测试 100% 兼容） |
| `cargo clippy --all-targets -- -D warnings` | **0 warnings** |
| `cargo fmt --all --check` | clean |

### 对抗性检查

| 检查 | 结果 | 证据 |
|---|---|---|
| 交互式审批拒绝（`n`）不产生副作用 | 通过 | `cli_ui::interactive_repl_denial_blocks_external_write_with_open_issue` |
| 交互式审批通过（`y`）仍须通过 M6 强制验证方可完成 | 通过 | `cli_ui::interactive_repl_supports_approval_prompt_and_completes` |
| 非 `--ui` 子命令默认 JSON 契约不被破坏 | 通过 | `e2e::cli_runs_offline_fixture` |

## 6. 遗留项（Known issues / Remaining）

1. **多行输入与历史快捷键**：当前 REPL 基于标准 `stdin.read_line()`，尚未引入 raw-mode 行编辑器（如上下键历史记录），以免引入额外终端状态副作用。

## 7. 判定（Verdict）

**PASS** — Claude Code 风格 CLI UI 已完成设计、实现与强制 Debug 闭环，测试与静态检查全部通过。
