# Debug Report — Router Block 补强（loop guard + 路由遥测）

- **日期**: 2026-11-16
- **关联**: 项目核心 router block（模型自路由）；计划 §4 运行时循环防护 + §13.4 路由指标
- **判定**: **PASS**

## 1. 症状（Scope）

补强 router：① 运行时循环/重复调用防护（计划明文要求、此前缺失）；② 路由质量遥测
（意图直方图、误路由、重复重放、loop-guard、ask_user、激活/发现计数）；
③ benchmark 新增 Routing quality 报表。

## 2. 根因与修复

| # | 症状 | 根因 | 修复 | 回归 |
|---|---|---|---|---|
| 1 | `failed_tool` fixture 回归（3 连错不再 blocked） | loop guard 把重复失败动作也当循环拦截，绕过错误界限 | 仅"成功动作"入 loop 历史；失败重试归连续错误界限管 | `fixtures::failed_tool_blocks_after_bounded_retry` 恢复通过 |
| 2 | `result.routing` 恒为 Null | completed 路径（try_complete）未挂载路由报告；fmt 后补丁两次未命中 | 精确编辑补上挂载 + trace 记录 | `router::routing_report_is_recorded_for_self_routed_runs` |
| 3 | 守卫拦截时决策审计缺失 | guarded 分支 `continue` 跳过 record_decision_started | 守卫分支内补记录（model_calls 统计精确） | 代码审查 |
| 4 | 测试沿用 Rc/RefCell 访问模式 | 新测试文件 | `.lock().unwrap()` | 编译 |

## 3. 回归验证

- `cargo test --release`：**97 passed / 0 failed**（新增 `tests/router.rs` 5 个：守卫拦第 3 次、
  异动作不拦、误路由/重复重放计数、自路由报告、ask_user 计数）
- clippy 0 告警；fmt clean
- benchmark Routing quality 报表实测渲染（steps/tool calls/misroutes/loop-guard/dup/ask_user）

## 4. 遗留项

1. ask_user 触发策略（不确定性→提问）尚未自动化（当前由模型自主选择）。
2. `capabilities.discover` 资格过滤仍为关键词匹配。
3. 规则路由器基线（计划 §13.3 #9）与 E8 自路由消融待跑。
4. replayed 意图也计入 loop 历史（语义可再收紧，影响极小）。

## 5. 判定

**PASS** — router 核心获得计划要求的运行时防护与可观测性；97 测试全绿。
