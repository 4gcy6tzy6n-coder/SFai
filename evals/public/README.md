# 公开基准任务集（Public benchmark task sets）

目的：满足 NeurIPS/ICLR Datasets & Benchmarks 轨对"公开/可扩展任务集"的硬要求。
统一 TaskSpec 格式（`tasks.json`），全部**随仓发布、可复现、带确定性 ground truth**。

## 套件

| suite | 来源 | 规模 | 任务形态 | ground truth | 许可 |
|---|---|---|---|---|---|
| `public-gsm8k` | [GSM8K](https://github.com/openai/grade-school-math)（OpenAI，test split） | 20（整数答案子集） | 数学文字题 → answer.json | 精确数值 | MIT |
| `public-humaneval` | [HumanEval](https://github.com/openai/human-eval)（OpenAI，canonical） | 20 | 函数实现 → 随附测试套件 | `code_tests_pass`（测试即独立裁判） | MIT |
| `public-sheets` | [SpreadsheetBench](https://github.com/RUCKBReasoning/SpreadsheetBench)（NeurIPS'24，verified_400） | 建设中 | 表格计算 → answer.json | 单元格/计算值 | CC-BY-SA-4.0 |

## 口径

- GSM8K：仅取整数答案题（剥离千分位逗号），排除分数/小数歧义；
- HumanEval：任务自带 `solution.py`（签名）+ `test_solution.py`（官方测试），
  正确性 = `acceptance` 型（测试通过即正确，独立于模型自评）；
- 全部任务冻结于 `tasks.json`（进入 data-freeze 清单），重跑不修改。

## 复现

```bash
cargo run --release -- bench --config experiments/configs/deepseek-e9-public.json
```
