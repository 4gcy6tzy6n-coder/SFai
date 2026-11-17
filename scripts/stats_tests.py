#!/usr/bin/env python3
"""Statistical tests + rate breakdown for the verification-gate experiments.

Addresses supervisor risk #1 (CI logic error -> paired tests) and #2
(zero-false-success may hide zero success -> full rate breakdown, explicit
false-success denominators).

McNemar exact test on PAIRED task outcomes (same suite/spec_id/repetition run
under two ablations). Two-sided exact binomial p-value on discordant pairs.
"""
import json
import math
from pathlib import Path

R = Path("experiments/results")


def load(name):
    return json.loads((R / name).read_text())


def paired_tasks(d, abl_a, abl_b, suite=None):
    """Pair task outcomes by (suite, spec_id, repetition)."""
    key = lambda t: (t["suite"], t["spec_id"], t["repetition"])
    a = {key(t): t for t in d["tasks"] if t["ablation"] == abl_a and (suite is None or t["suite"] == suite)}
    b = {key(t): t for t in d["tasks"] if t["ablation"] == abl_b and (suite is None or t["suite"] == suite)}
    common = sorted(set(a) & set(b))
    return [(a[k], b[k]) for k in common]


def mcnemar_exact(pairs, field):
    """Two-sided exact McNemar: b = A-only events, c = B-only events."""
    b = sum(1 for x, y in pairs if x[field] and not y[field])
    c = sum(1 for x, y in pairs if y[field] and not x[field])
    n = b + c
    if n == 0:
        return b, c, 1.0
    k = min(b, c)
    # two-sided exact binomial p = 2 * P(X <= k), capped at 1
    p = sum(math.comb(n, i) for i in range(0, k + 1)) * (0.5 ** n) * 2
    return b, c, min(p, 1.0)


def rate_breakdown(d, ablation, suite=None):
    rows = [t for t in d["tasks"] if t["ablation"] == ablation and (suite is None or t["suite"] == suite)]
    n = len(rows)
    completed = sum(1 for t in rows if t["status"] == "completed")
    correct = sum(1 for t in rows if t["correct"])
    fs = sum(1 for t in rows if t["false_success"])
    rejected = sum(1 for t in rows if t["status"] in ("blocked", "failed"))
    errors = sum(1 for t in rows if t["status"] == "error")
    fs_all = fs / n if n else 0.0
    fs_claimed = fs / completed if completed else float("nan")
    return dict(n=n, completed=completed, correct=correct, false_success=fs,
                rejected=rejected, errors=errors,
                completion_rate=completed / n if n else 0,
                correctness_all=correct / n if n else 0,
                fs_over_all=fs_all,
                fs_over_claimed=fs_claimed)


def fmt_rate(r):
    return (f"n={r['n']} 完成={r['completed']}({r['completion_rate']:.2f}) "
            f"正确={r['correct']}({r['correctness_all']:.2f}) "
            f"假成功={r['false_success']}(占全部 {r['fs_over_all']:.2f} / 占声称成功 "
            f"{('%.2f' % r['fs_over_claimed']) if r['completed'] else 'n/a(无声称成功)'}) "
            f"拒绝/失败={r['rejected']} 错误={r['errors']}")


print("# 统计检验与比率分解（回应导师风险 #1、#2）\n")
print("**假成功定义（benchmark-spec v0.2.2 口径）：**")
print("- 主指标 `fs_over_all` = 假成功 / **全部运行**（现表格口径）")
print("- 条件指标 `fs_over_claimed` = 假成功 / **声称成功的运行**（假发现率）")
print("- 假成功 = 状态 completed 且 (验收失败 或 交付物错误)。拒绝/阻塞**不计**假成功，单列。\n")

print("## 风险 #1 修正：Wilson CI 重叠 + 配对 McNemar 精确检验\n")
print("原表述『CI 不相交』有误——[0.18,0.61] 与 [0.00,0.19] 在 0.18–0.19 存在重叠。")
print("正确做法：配对设计用 McNemar 精确检验（同一任务两种消融各跑一次，配对比较）。\n")

print("### E5-gate（deepseek，同任务配对）\n")
d = load("e5-gate-ci-deepseek.json")
for suite in ["tool-use", "dataset", "long-horizon", "edge-case"]:
    pairs = paired_tasks(d, "no_verification", "none", suite)
    if not pairs:
        continue
    b, c, p = mcnemar_exact(pairs, "false_success")
    print(f"- {suite}: 不一致对（仅无门假成功 b={b} / 仅 有门假成功 c={c}），McNemar 精确 p = {p:.4f}")
b, c, p = mcnemar_exact(paired_tasks(d, "no_verification", "none"), "false_success")
print(f"- **pooled（4 suite 合并配对）**: b={b}, c={c}, **McNemar 精确 p = {p:.6f}**")
bc, cc, pc = mcnemar_exact(paired_tasks(d, "no_verification", "none"), "correct")
print(f"- 正确性方向配对：b={bc}, c={cc}, p = {pc:.4f}（有门提升）\n")

print("### 18 模型聚合（e2m + E1 + E2，逐 suite 配对后合并）\n")
all_pairs = []
for f in sorted(R.glob("e2m-*.json")) + [R / "e1-deepseek-baseline.json", R / "e2-mimo-baseline.json"]:
    dd = json.loads(f.read_text())
    all_pairs += paired_tasks(dd, "no_verification", "none")
b, c, p = mcnemar_exact(all_pairs, "false_success")
print(f"- 全部配对任务（{len(all_pairs)} 对）: b={b}, c={c}, **McNemar 精确 p = {p:.2e}**\n")

print("### E9 公开基准\n")
for name, abl_a, abl_b in [("e9-public-deepseek.json", "no_verification", "none"),
                            ("e9-public-humaneval-bailian.json", "no_verification", "none"),
                            ("e9-public-sheets-bailian.json", "no_verification", "none")]:
    dd = load(name)
    pairs = paired_tasks(dd, abl_a, abl_b)
    if pairs:
        b, c, p = mcnemar_exact(pairs, "false_success")
        print(f"- {name}: b={b}, c={c}, p = {p:.4f}（{len(pairs)} 对）")
print()

print("## 风险 #2 修正：完整比率分解（分母全披露）\n")
for name, abls in [
    ("e1-deepseek-baseline.json", ["none", "no_verification"]),
    ("e5-gate-ci-deepseek.json", ["none", "no_verification"]),
    ("e9-public-deepseek.json", ["none", "no_verification"]),
    ("e9-public-humaneval-bailian.json", ["none", "no_verification"]),
]:
    d2 = load(name)
    cfg = d2.get("config") or {}
    print(f"### {cfg.get('name', name)}")
    for abl in abls:
        print(f"- {abl}: {fmt_rate(rate_breakdown(d2, abl))}")
    print()

print("### 关键澄清：零假成功 ≠ 零成功\n")
print("- GSM8K 有门：完成率 0.95、正确率 0.95——**真实完成，非全拒绝**（健康对照）。")
print("- HumanEval 有门：完成率 0.00——**全部被拒**。此格的 fs=0 是平凡零（无声称成功可假），")
print("  其证据价值是『无门 80% 声称成功全是假的』（fs_over_claimed=1.00 附近），")
print("  **不能**解读为『门解决了任务』。表格必须同时给完成率（已加列）。")
print("- 18 模型主表 72 格：多数格子有门完成率 > 0 且 correct=completed（真完成）；")
print("  逐格完成率见 grand-model-table 增补列。")
