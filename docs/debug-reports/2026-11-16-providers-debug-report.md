# Debug Report — 真实 Provider 接入（DeepSeek / Qwen / MiMo）

- **日期**: 2026-11-16
- **开发者 / 会话**: lead（agent dev session）
- **关联 milestone**: v1 真实模型接入（provider profiles + 适配器强化 + CLI/API/bench 集成）
- **判定**: **PASS**（离线验证；真实调用需用户提供 API key）

## 1. 症状（Scope）

- 新增 provider profiles：`deepseek`（首发）、`qwen`、`mimo`、`custom`；
- 适配器改为单工具 `submit_action` + content JSON 回退，429/5xx 重试，按 profile 计价；
- CLI `--provider/--model/--price-in/--price-out` 集成 run / serve / bench；
- `GET /api/providers`；`POST /api/tasks/:id/run` 支持 provider/model 覆盖；
- benchmark v1：`sfai bench --provider ...` 用真实模型跑同一套任务集/消融/指标；
- 文档 `docs/providers.md`、ADR-019、README。

## 2. 复现步骤（Reproduction）

```bash
cargo test
cargo clippy --all-targets          # 0 warnings
cargo fmt --all --check             # clean
# 真实调用（需要 key，未在 CI/本会话执行）：
export DEEPSEEK_API_KEY=sk-...
cargo run -- run --goal "..." --provider deepseek --model deepseek-chat
cargo run -- bench --provider deepseek --model deepseek-chat --suite dataset --ablation none --size 3
```

## 3. 根因与修复（Root cause & Fix）

| # | 症状 | 复现证据 | 根因 | 修复 | 回归用例 |
|---|---|---|---|---|---|
| 1 | `profiles` 模块无法解析 | `cargo check` E0432 | `providers` 在 lib.rs 用 inline 模块声明，`providers/mod.rs` 不生效 | 在 lib.rs 的 inline `providers` 中声明 `profiles`，删除无用的 mod.rs | 编译 + profiles 测试 |
| 2 | `serve` 调用参数不匹配 | E0061（3 vs 4） | serve 新增默认 provider 参数，CLI 调用未同步 | CLI 传入解析后的 profile | 编译 |
| 3 | 真实模型对象无法移入 `spawn_blocking` | 设计阶段发现（`Box<dyn ModelAdapter>` 非 Send） | 在请求线程构造 model 再 move 进阻塞线程 | 改为在阻塞闭包内用 `ProviderProfile` 构造适配器 | server 编译 + api 测试 |
| 4 | API 测试编译失败 | `ServerState::new` 签名变化 | 新增 provider 字段 | 测试更新为三参数 | api 测试 |
| 5 | 格式检查失败 | `cargo fmt --check` | 新代码未格式化 | `cargo fmt` | fmt clean |

## 4. 修复范围（Files）

`integrations/providers/profiles.rs`（新增）、`openai_compat.rs`（重写）、
`core/action_intents.rs`（`action_tool_schema`）、`server.rs`、`benchmark.rs`、
`bin/sfai.rs`、`tests/api.rs`、`docs/providers.md`、ADR-019、README。

## 5. 回归验证（Regression）

| 检查 | 结果 |
|---|---|
| `cargo test` | **86 passed / 0 failed**（lib 36 + api 6 + benchmark 5 + cli_ui 4 + e2e 3 + eval 5 + fixtures 5 + m6 5 + security 4 + skills 4 + storage 4 + wiki 5） |
| `cargo clippy --all-targets` | **0 warnings** |
| `cargo fmt --all --check` | clean |
| profiles | 内置四个 profile、DeepSeek 默认值、overrides、未知 provider 报错 |
| 适配器 | mock HTTP：tool-call 归一化、content JSON（含代码围栏）回退、纯文本→respond、按 profile 计价 |
| API | `/api/providers` 列出 deepseek/qwen/mimo/custom；未配置 provider 的 run 返回 400 且错误可读 |
| benchmark v0 | 仍走脚本化模型（provider=None），v0.1 基线不变 |

## 6. 遗留项（Known issues / Remaining）

1. **真实调用未执行**：本机无 `DEEPSEEK_API_KEY`（检查过全部相关 env）；用户 export key 后即可跑，命令见上。
2. 无 streaming；重试已实现但未做熔断/限流。
3. 价格为近似值，正式成本对比前需 `--price-in/--price-out` 或写入合同价。
4. reasoning 模型（如 deepseek-reasoner）的 reasoning token 计费按 provider usage 记录，报告需注明。
5. MiMo 端点/价格待官方信息；当前通过 env 配置。

## 7. 判定（Verdict）

**PASS**（离线路径全部验证）— provider 抽象、适配器、CLI/API/bench 集成完成，
86 个离线测试通过、clippy 0 告警、格式干净；真实调用等待 API key。
