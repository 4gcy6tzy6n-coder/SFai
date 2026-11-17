# P0 审计报告 #1 — HumanEval 判分器独立复核（ICML 冲刺第一项）

**日期:** 2026-11-17 · **判定:** **旧 HumanEval 数据全部作废；三 bug 修复后复核通过**

## 1. 异常

HumanEval 正确率 0.00 跨模型一致（deepseek-chat、deepseek-v4.1-flash）——GSM8K 0.95
的同一模型不可能 HumanEval 0 分。独立复核（he_audit.rs，真实模型全程插桩）定位到
**三个叠加 bug**，全部在判分侧而非模型侧：

## 2. 三个 bug（按发现顺序）

| # | Bug | 证据 | 修复 |
|---|---|---|---|
| 1 | **工件从未落盘**：`code_tests_pass` 在空目录执行命令，对象库里的 solution.py/test_solution.py 从未物化 → `python3 test_solution.py` exit 2（文件不存在） | 同一份交付物手动跑官方测试 exit 0 通过，判分器报 exit 2 | `materialize_workspace()`：执行前把全部工件物化到工作区 |
| 2 | **评分卷可被覆写（自评作弊风险）**：模型把 test_solution.py 改写成自己的弱测试并通过 | 审计抓到 file.write 覆写 test_solution.py 事件 | `protected_files`（BenchSpec 字段）：验证物化时用**初始 fixture 版本**（`find_artifact_oldest`），模型无法改评分标准 |
| 3 | **转换器缺 import**（本项目生成任务文件的 bug）：`check(has_close_elements)` 未导入函数 → NameError | 修复 1 后官方测试仍 Traceback | 重生成 20 个测试文件补 `from solution import <entry_point>`；**标准答案自检 3/3 通过** |

## 3. 复核结果（修复后端到端）

| 任务 | 模型解题 | 判分器 | 官方测试 |
|---|---|---|---|
| humaneval-0 | 写出 solution.py | **Pass** | **ALL TESTS PASSED** |
| humaneval-1 | 写出 solution.py | **Pass** | **ALL TESTS PASSED** |

- 判分链独立验证：标准答案（canonical_solution）跑固定测试 **3/3 通过**；
- 防作弊验证：模型再写 test_solution.py 也无效（保护生效）。

## 4. 影响与作废范围

- **作废**：e9-public-deepseek（HumanEval 部分）、e9-public-humaneval-bailian 的
  正确率/假成功/完成率全部不可信（判分器恒判失败 → 门全拒 → "平凡零"结论亦需重估）。
- **重测**：修正版 HumanEval 双消融已在跑（e9-public-humaneval-fixed.json）。
- **连锁修正**：先前"HumanEval fs 0.80→0.00 是最狠对照"的表述**撤回**——其中
  gate-on 的 0.00 完成率由 bug 造成；无门 fs 0.80 中含"代码对但被判错"的成分
  （无门臂不看判分器所以其 fs 计数不受 bug 影响？——无门臂的 correct 由同一
  checker 打分，同样受 bug 影响 → 其 cor 0.00 也作废）。以重测为准。

## 5. 方法论收获（写入实验协议）

1. **判分器必须先过"标准答案正向自检"**（canonical solution 必须满分）才能用于实验；
2. **评分文件必须保护**（agent 不可改评分标准）；
3. **判分器失败模式必须区分** exit 2（基础设施）vs exit 1（真失败）——本 bug 正是
   把 exit 2 混进了"任务失败"。
