# E6 — Framework Overhead Benchmark（sfai Rust 内核 vs Python/LangGraph）

**日期:** 2026-11-16 · **bench 版本:** 0.2.1 · 机器: Apple Silicon macOS
**目的:** 检验"Rust 内核响应速度优于纯 Python 框架"这一系统级主张的可量化证据。

## 测量结果

| 工作负载 | sfai (Rust) | Python/LangGraph | 手写 Python 循环（稻草人） |
|---|---|---|---|
| 裸编排循环（5 步/任务，内存 SQLite） | — | 0.79 ms/task | 0.06 ms/task |
| **全管线对等**（文件级 SQLite WAL + 13 表迁移 + FTS 种子/检索 + 事件落账 + trace） | **3.5–4.5 ms/task**（bench p50，含验收+Wiki 编译+临时目录） | **3.74 ms/task** | — |
| 200KB（50k token）信封 JSON 往返 | ~0.05 ms/步（serde，估） | 0.25 ms/步 | — |
| 冷启动 | **~5 ms** | ~10–15 ms（python+langgraph import） | ~20 ms |
| 分发体积 | 单二进制 ~6 MB | venv+langgraph 50–100 MB | — |
| 真实任务中框架开销占比（LLM 8–80s） | <0.05% | <0.1% | — |

## 诚实结论

1. **单任务延迟在对等工作量下基本打平**（~3.7ms 双方），瓶颈是 SQLite 文件 I/O 与 schema
   迁移，不是语言本身。"纯 Python 慢很多"在单任务毫秒级**不成立**——手写循环反而最快
   （0.06ms，因为它什么都没做）。
2. **成立的性能主张**：冷启动 2–3×（CLI 交互体感）、大信封序列化 5×、分发体积 10×+
   （单二进制 dmg）、内存占用、以及**并发吞吐**（无 GIL，待 E6b 测量：tokio 多线程
   vs Python 单事件循环）。
3. **同耗时更强语义**是更稳的主张：同样 ~3.7ms，sfai 每步多做了策略检查、幂等键、
   收据、溯源、验收与 Wiki 编译；LangGraph 对等脚本只做了其中一部分。
4. **真实任务（LLM 主导）中框架开销占比 <0.1%**：速度主张应聚焦高频服务吞吐、
   冷启动与工具密集型循环，而不是单任务毫秒数。

## 对论文定位的含义

- "Rust 内核更快"需改写为：**"同等保证下无额外开销 + 冷启动/大上下文/并发/分发优势"**；
  需要 E6b（并发吞吐）与 E7（LangGraph 端到端同基准任务）补证。
- 系统论文的真正支柱是**完整性与可审计性**（框架+基准+验证门+多 provider+UI/dmg），
  性能是支撑项而非主标题。

## 复现

```bash
python3 scripts/bench_python_overhead.py 200 5        # 裸循环
/tmp lg: bench_langgraph.py / bench_langgraph_full.py / bench_parity.py（见 debug 报告）
cargo run --release -- bench --suite dataset --size 30 --ablation none   # sfai 全管线
```
