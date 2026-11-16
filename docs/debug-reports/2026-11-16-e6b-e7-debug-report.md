# Debug Report — E6b 内存/GIL 并发实测 + E7 LangChain 跨框架真实对比

- **日期**: 2026-11-16
- **关联**: 系统级主张补证（性能：内存/GIL 并发；端到端：LangChain 真实对比写入内容）
- **判定**: **PASS**（含一次判分器假阴性的发现与修正，见 §2.1）

## 1. 症状（Scope）

① E6 中"内存/GIL 并发优势（待测）"必须实测；② E7 跨框架真实对比（LangChain 等）
必须写入真实内容。交付：Rust vs Python/LangGraph 并发/内存实测 + 同模型同任务
跨框架端到端对比（LangGraph ReAct vs sfai 完整内核）。

## 2. 根因与修复

### 2.1 E7 判分器假阴性（重要，已修正并重跑）

| 症状 | 根因 | 修复 | 回归 |
|---|---|---|---|
| LangChain dataset 正确率 0.00（可疑） | 判分器键名强校验 `data["total"]`；智能体实际写 `{"total_revenue": 444}`（**值是对的**） | 判分器改为**值匹配**（答案值出现在 JSON 任意值位即正确） | probe 取证 + 重跑 6 任务；结论修正为 0.17，第一版 0.00 作废 |

### 2.2 其他

| 症状 | 根因 | 修复 |
|---|---|---|
| Rust cpu 模式编译错（format 捕获 / json! 数组重复） | 宏内语法限制 | 显式 format 参数 + 预构造 Vec |
| /usr/bin/time -l RSS 解析得 0 | awk 取错字段 | 读原始输出（bytes） |
| Python 全管线并发吞吐随 workers 下降 | GIL + 文件锁竞争 | 保留为诚实数据（I/O 密集双方都受限），CPU 模式直测 GIL |

## 3. 实测结果

**E6b 并发（CPU 密集，双方同负载 200×50KB 信封处理/任务）：**

| workers | Rust tasks/sec | Python threads |
|---|---|---|
| 1 | 19.4 | 10.5 |
| 16 | **98.5（5.1×）** | **10.6（1.0×，GIL 平线）** |

内存：sfai **10.6 MB** vs python+langgraph **14.7–27.6 MB**。冷启动/大信封数据见 E6 报告。

**E7 跨框架（deepseek-chat 同模型、同任务、同判分器，n=6/suite）：**

| 指标 | LangChain ReAct | sfai |
|---|---|---|
| dataset 正确率 | 0.17 | **0.83** |
| dataset 假成功 | **0.83** | **0.00** |
| dataset p50 / tokens | 3.0s / 1918 | 7.6s / 6599 |
| tool-use（全指标） | 1.00/1.00/0.00 | 1.00/1.00/0.00 |

核心结论：验证门把模型算错从**静默假成功（0.83）**变成**修复后正确（0.83）**，
代价 3.4× token / 2.5× 延迟；tool-use 双方平——差距在容错任务显现。

## 4. 回归验证

- `cargo test --release`：**99 passed / 0 failed**；clippy 0；fmt clean
- sfai 臂路由遥测（真实）：tool-use 4 误路由 + 3 loop-guard 拦截后恢复完成
- 判分器修正经 probe 复现验证（total_revenue=444 判对）

## 5. 遗留项

1. LangChain 臂未接验证层是**框架原样**（其生态验证需另配）；论文需注明。
2. 温度设置差异（LangChain 臂 temperature=0；sfai 臂 provider 默认）——v2 统一。
3. E6b I/O 密集并发双方均受文件系统限制，需磁盘更快环境复测。
4. E8 自路由消融仍是 router 柱子最后一块。

## 6. 判定

**PASS** — 内存/GIL 并发优势实测成立（5.1× vs 1.0×、1.4–2.6× 内存）；E7 真实
跨框架对比已写入，核心发现（验证门消除静默假成功）与判分器修正均已诚实归档。
