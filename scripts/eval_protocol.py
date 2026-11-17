#!/usr/bin/env python3
"""Y/C/A evaluation protocol — six reliability metrics (Paper A, A1).

Formal semantics (docs/paper/eval-protocol.md):
  Y = independent environment ground truth (deliverable correct)
  C = agent claimed completion (Respond/Finish processed; older data: status==completed)
  A = verification gate allowed delivery (final status == completed)

Metrics:
  True Success            P(Y=1)
  Claim False Success     P(C=1, Y=0)
  Delivered False Success P(A=1, Y=0)
  Accepted Error Rate     P(Y=0 | A=1)
  Valid Success Recall    P(A=1 | Y=1)
  False Rejection         P(A=0 | Y=1)

Wilson 95% CIs on every rate; trivial-zero detection (a zero with zero claims is
flagged, not celebrated); paired McNemar for gate comparisons.

Usage:
  python3 scripts/eval_protocol.py [results.json ...]
"""
import json
import math
import sys
from pathlib import Path

R = Path(__file__).resolve().parents[1] / "experiments" / "results"

DEFAULT_FILES = [
    "e1-deepseek-baseline.json",
    "e5-gate-ci-deepseek.json",
    "e9-public-deepseek.json",
    "e9-public-humaneval-fixed.json",
    "e9-public-sheets-bailian.json",
    "e10-sfai.json",
]


def wilson(k, n, z=1.96):
    if n == 0:
        return (0.0, 0.0, 0.0)
    p = k / n
    d = 1 + z * z / n
    c = p + z * z / (2 * n)
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return (p, max(0.0, (c - h) / d), min(1.0, (c + h) / d))


def claimed_of(t):
    if "claimed" in t:
        return bool(t["claimed"])
    # legacy rows: without the flag, the final status is the best proxy
    return t["status"] == "completed"


def metrics(rows):
    n = len(rows)
    if n == 0:
        return None
    y = [bool(t["correct"]) for t in rows]
    c = [claimed_of(t) for t in rows]
    a = [t["status"] == "completed" for t in rows]
    n_y = sum(y)
    n_c = sum(c)
    n_a = sum(a)
    claim_fs = sum(1 for ci, yi in zip(c, y) if ci and not yi)
    deliv_fs = sum(1 for ai, yi in zip(a, y) if ai and not yi)
    accepted_err = sum(1 for ai, yi in zip(a, y) if ai and not yi)
    valid_rec = sum(1 for ai, yi in zip(a, y) if ai and yi)
    false_rej = sum(1 for ai, yi in zip(a, y) if not ai and yi)
    return {
        "n": n,
        "claims": n_c,
        "accepted": n_a,
        "true_success": wilson(n_y, n),
        "claim_false_success": wilson(claim_fs, n),
        "delivered_false_success": wilson(deliv_fs, n),
        "accepted_error_rate": wilson(accepted_err, n_a),
        "valid_success_recall": wilson(valid_rec, n_y),
        "false_rejection": wilson(false_rej, n_y),
        "trivial_zero": n_c == 0 or n_a == 0,
    }


def fmt(w):
    p, lo, hi = w
    return f"{p:.2f} [{lo:.2f},{hi:.2f}]"


def main():
    files = [Path(f) for f in sys.argv[1:]] or [R / f for f in DEFAULT_FILES]
    print("# Y/C/A 六指标评价协议 — 应用于冻结数据（Paper A 表 1 草稿）\n")
    print("**语义**: Y=独立真值(交付物正确) · C=模型声称完成 · A=验证门放行(最终 completed)")
    print("**口径**: 全部为任务级比率 + Wilson 95% CI；**平凡零标记**：当声称/放行为 0 时零值不作证据")
    print("**旧数据注**: 无 claimed 字段的历史行以 status 代理 C（协议 v1 起新跑均有精确 claimed）\n")
    for f in files:
        if not f.exists():
            continue
        d = json.loads(f.read_text())
        cfg = d.get("config") or {}
        print(f"## {cfg.get('name', f.stem)}（{cfg.get('provider','')}/{cfg.get('model','')}）\n")
        by_abl = {}
        for t in d.get("tasks", []):
            by_abl.setdefault(t["ablation"], []).append(t)
        print("| 消融 | n | P(Y=1) True Success | P(C=1,Y=0) Claim FS | P(A=1,Y=0) Delivered FS | P(Y=0\\|A=1) Accepted Err | P(A=1\\|Y=1) Valid Recall | P(A=0\\|Y=1) False Rej | 平凡零 |")
        print("|---|---|---|---|---|---|---|---|---|")
        for abl, rows in sorted(by_abl.items()):
            m = metrics(rows)
            if not m:
                continue
            flag = "⚠️（claims=0）" if m["claims"] == 0 else ("⚠️（accepted=0）" if m["accepted"] == 0 else "—")
            print(f"| {abl} | {m['n']} | {fmt(m['true_success'])} | {fmt(m['claim_false_success'])} | "
                  f"{fmt(m['delivered_false_success'])} | {fmt(m['accepted_error_rate'])} | "
                  f"{fmt(m['valid_success_recall'])} | {fmt(m['false_rejection'])} | {flag} |")
        print()
    print("## 解读规则（写作时执行）")
    print()
    print("1. **零假成功必须与 P(A=1) 联读**：accepted=0 的零是平凡零（HumanEval 教训）；")
    print("2. Claim FS 与 Delivered FS 的差 = 门的拦截贡献；")
    print("3. Accepted Error Rate 是'交付可信度'（False Discovery）；")
    print("4. Valid Success Recall + False Rejection = 门的覆盖率代价（风险-覆盖权衡图轴）；")
    print("5. 配对比较用 McNemar（scripts/stats_tests.py），不做 CI 相交推断。")


if __name__ == "__main__":
    main()
