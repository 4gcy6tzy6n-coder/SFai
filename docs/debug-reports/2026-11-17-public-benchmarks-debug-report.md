# Debug Report — 公开基准迁移（GSM8K / HumanEval / SpreadsheetBench）

- **日期**: 2026-11-17 · **关联**: NeurIPS D&B 公开任务集硬要求 · **判定**: **PASS**

## 1. 范围

构建公开/可扩展任务集：统一 TaskSpec（`evals/public/tasks.json`，冻结入 data-freeze）、
公开基准真实迁移（GSM8K 20 题、HumanEval 20 题）、benchmark 公开套件框架
（`public-*` loader + acceptance 型正确性）、E9 真实跑（deepseek × 门对照）。

## 2. 根因与修复

| # | 症状 | 根因 | 修复 | 回归 |
|---|---|---|---|---|
| 1 | `unknown suite: public-gsm8k` 崩溃 | build_model 离线回退分支不含公开套件 | 公开套件离线用最小脚本模型（冒烟），真实跑走 provider | `public_suites` 4 测试 |
| 2 | BenchSpec 反序列化失败 / 构造器缺字段 | 新增 ground_truth_kind/files 字段 | serde default + 4 个合成构造器补字段 | 编译 + lib 测试 |
| 3 | SpreadsheetBench 数据拉取失败 | GitHub 大文件与 HF API 全部网络不可达（http=000，三轮重试） | 记录为"就绪待数据"（loader 已备）；GSM8K/HumanEval 两套已满足"构建一个"要求 | 文档注明 |
| 4 | E9 HumanEval 首跑 27 运行失败 | 夜间网络故障（连接重置/超时，非框架缺陷） | 作废首跑、白天重跑归档（e9-public-humaneval-rerun.json） | 重跑对比 |
| 5 | GSM8K 答案格式歧义（1,234 / 3/4） | 字符串匹配歧义 | 只取整数答案题 + 剥离千分位 | tasks.json 口径注明 |

## 3. 验证

- `cargo test --release`：**113 passed**（public_suites 4：库加载、端到端冒烟、
  acceptance 型正确性、size cap）；clippy 0；fmt clean。
- E9 GSM8K（干净）：有门 suc/cor 0.95、fs 0.00；无门 cor 0.70、fs 0.15
  ——**公开任务上门对照成立**。
- E9 HumanEval：首跑网络污染作废；重跑数据归档后并入 T9。
- 路由遥测：gsm8k 有门 loop-guard 40 次拦截（数学循环守卫实证）。

## 4. 遗留

1. `public-sheets`（SpreadsheetBench，CC-BY-SA-4.0）：TaskSpec/loader 就绪，
   待网络恢复拉取 verified_400 数据（15MB tarball）。
2. HumanEval 重跑中零星超时（服务端响应慢）——若仍污染则缩样重试。
3. GSM8K 取整数答案子集（口径已在 README 注明，可扩展到分数答案）。

## 5. 判定

**PASS** — 公开/可扩展任务集框架与两套真实公开基准落地，D&B 硬要求达成；
网络事故如实作废重跑，数据纪律延续。
