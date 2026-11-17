#!/usr/bin/env python3
"""B0–B4 causal decomposition of verification & repair (Paper B, B3).

Design (all arms judge the SAME base runs — perfect pairing):
  B0  no gate            — bench `no_verification` arm (claim = success)
  B1  gate, no repair    — deterministic acceptance applied post-hoc to B0 runs
  B2  gate + repair      — bench `none` arm (kernel gate + 2 repair retries)
  B3  self-check+repair  — the SAME model judges its own deliverable (post-hoc)
  B4  judge + repair     — independent judge call (neutral prompt, post-hoc)

Inputs:
  --bench <bench.json>       bench run with no_verification + none arms
                             (run with SFAI_BENCH_PERSIST=1 so workspaces persist)
  --ws <dir>                 SFAI_BENCH_WS root
  --tasks <tasks.json>       acceptance rules per spec_id
  --model-id                 model used in the base runs (self-check = same model)

Output: B0–B4 table with Y/C/A six metrics + accepted-error-rate comparison.
"""
import argparse
import json
import os
import re
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

BASE = os.environ.get("SFAI_E10_BASE_URL", "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1")
KEY = os.environ.get("SFAI_E10_API_KEY", "")


def llm_judge(prompt: str, model: str) -> str:
    body = json.dumps({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0,
        "max_tokens": 200,
    }).encode()
    req = urllib.request.Request(
        BASE.rstrip("/") + "/chat/completions", data=body,
        headers={"Content-Type": "application/json", "Authorization": f"Bearer {KEY}"},
    )
    try:
        with urllib.request.urlopen(req, timeout=120) as r:
            data = json.loads(r.read())
        return data["choices"][0]["message"]["content"]
    except Exception as e:  # noqa: BLE001
        return f"JUDGE_ERROR: {e}"


def check_acceptance(rules, ws: Path):
    """Deterministic gate — same semantics as sfai verify engine (B1)."""
    for rule in rules:
        name, _, arg = rule.partition(":")
        if name == "artifact_exists":
            if not (ws / arg).exists():
                return False
        elif name == "json_valid":
            try:
                json.loads((ws / arg).read_text())
            except Exception:  # noqa: BLE001
                return False
        elif name == "artifact_contains":
            path, _, needle = arg.partition(":")
            try:
                if needle not in (ws / path).read_text():
                    return False
            except Exception:  # noqa: BLE001
                return False
        elif name == "source_count_at_least":
            path, _, k = arg.partition(":")
            try:
                refs = set(re.findall(r"source://[A-Za-z0-9_\\-]+@[0-9]+", (ws / path).read_text()))
                if len(refs) < int(k):
                    return False
            except Exception:  # noqa: BLE001
                return False
        elif name == "code_tests_pass":
            import subprocess
            r = subprocess.run(arg, shell=True, cwd=ws, capture_output=True, timeout=120)
            if r.returncode != 0:
                return False
    return True


def judge_verdict(rules, ws: Path, model: str, mode: str) -> bool:
    """B3 self-check (same model, first-person) / B4 independent judge."""
    files = {}
    for f in ws.iterdir():
        if f.is_file() and f.stat().st_size < 20000:
            try:
                files[f.name] = f.read_text()[:2000]
            except Exception:  # noqa: BLE001
                continue
    if mode == "self":
        head = "You just completed a task. Judge YOUR OWN deliverable strictly."
    else:
        head = "You are an independent verification judge. Judge strictly."
    prompt = (
        f"{head}\\n\\nAcceptance rules:\\n- " + "\\n- ".join(rules) +
        "\\n\\nWorkspace files:\\n" +
        "\\n\\n".join(f"--- {k} ---\\n{v}" for k, v in files.items()) +
        '\\n\\nReply with exactly one word: PASS (all rules satisfied) or FAIL.'
    )
    out = llm_judge(prompt, model)
    return "PASS" in out.upper().split("FAIL")[0]


def wilson(k, n, z=1.96):
    import math
    if n == 0:
        return (0.0, 0.0, 0.0)
    p = k / n
    d = 1 + z * z / n
    c = p + z * z / (2 * n)
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return (p, max(0.0, (c - h) / d), min(1.0, (c + h) / d))


def fmt(w):
    return f"{w[0]:.2f} [{w[1]:.2f},{w[2]:.2f}]"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bench", required=True)
    ap.add_argument("--ws", default="/tmp/sfai-bench-ws")
    ap.add_argument("--tasks", default=str(ROOT / "evals/public/tasks.json"))
    ap.add_argument("--model", required=True)
    args = ap.parse_args()

    bench = json.loads(Path(args.bench).read_text())
    tasks = {t["id"]: t for t in json.loads(Path(args.tasks).read_text())}
    ws_root = Path(args.ws)

    b0 = [t for t in bench["tasks"] if t["ablation"] == "no_verification"]
    b2_rows = [t for t in bench["tasks"] if t["ablation"] == "none"]

    rows = []
    for t in b0:
        spec = tasks.get(t["spec_id"], {})
        rules = spec.get("acceptance_rules", [])
        ws = ws_root / f"{t['suite']}-no_verification-{t['spec_id']}-{t['repetition']}"
        y = bool(t["correct"])
        c = bool(t.get("claimed", t["status"] == "completed"))
        b1 = check_acceptance(rules, ws) if ws.exists() and rules else False
        b3 = judge_verdict(rules, ws, args.model, "self") if ws.exists() and rules and KEY else None
        b4 = judge_verdict(rules, ws, args.model, "judge") if ws.exists() and rules and KEY else None
        rows.append({"spec": t["spec_id"], "Y": y, "C": c, "B1": b1, "B3": b3, "B4": b4})

    def arm_metrics(verdicts, name, claims):
        accepted = [v for v in verdicts if v is not None]
        n = len(rows)
        n_a = sum(1 for v in accepted if v)
        deliv_fs = sum(1 for r, v in zip(rows, verdicts) if v and not r["Y"])
        valid_rec_num = sum(1 for r, v in zip(rows, verdicts) if v and r["Y"])
        n_y = sum(1 for r in rows if r["Y"])
        false_rej = sum(1 for r, v in zip(rows, verdicts) if (not v) and r["Y"])
        return (name, n,
                fmt(wilson(sum(1 for r in rows if r["Y"]), n)),
                fmt(wilson(deliv_fs, n)),
                fmt(wilson(deliv_fs, n_a)) if n_a else "n/a",
                fmt(wilson(valid_rec_num, n_y)) if n_y else "n/a",
                fmt(wilson(false_rej, n_y)) if n_y else "n/a")

    print("# B0–B4 验证/修复因果消融（同基线运行，完美配对）\n")
    print("| 臂 | n | True Success P(Y=1) | Delivered FS P(A=1,Y=0) | Accepted Err P(Y=0|A=1) | Valid Recall P(A=1|Y=1) | False Rej P(A=0|Y=1) |")
    print("|---|---|---|---|---|---|---|---|")
    # B0: no gate — claim becomes acceptance
    n = len(rows)
    n_y = sum(1 for r in rows if r["Y"])
    b0_fs = sum(1 for r in rows if r["C"] and not r["Y"])
    b0_acc = sum(1 for r in rows if r["C"])
    print(f"| B0 无门（claim=交付） | {n} | {fmt(wilson(n_y, n))} | {fmt(wilson(b0_fs, n))} | "
          f"{fmt(wilson(b0_fs, b0_acc)) if b0_acc else 'n/a'} | {fmt(wilson(sum(1 for r in rows if r['C'] and r['Y']), n_y)) if n_y else 'n/a'} | "
          f"{fmt(wilson(0, n_y)) if n_y else 'n/a'} |")
    print("| " + " | ".join(str(x) for x in arm_metrics([r["B1"] for r in rows], "B1 门·无修复", None)) + " |")
    # B2 from bench none arm
    if b2_rows:
        b2n = len(b2_rows)
        b2y = sum(1 for t in b2_rows if t["correct"])
        b2a = sum(1 for t in b2_rows if t["status"] == "completed")
        b2fs = sum(1 for t in b2_rows if t["status"] == "completed" and not t["correct"])
        b2fr = sum(1 for t in b2_rows if t["status"] != "completed" and t["correct"])
        print(f"| B2 门+修复（kernel，2 轮） | {b2n} | {fmt(wilson(b2y, b2n))} | {fmt(wilson(b2fs, b2n))} | "
              f"{fmt(wilson(b2fs, b2a)) if b2a else 'n/a'} | {fmt(wilson(b2y - b2fr, b2y)) if b2y else 'n/a'} | "
              f"{fmt(wilson(b2fr, b2y)) if b2y else 'n/a'} |")
    b3v = [r["B3"] for r in rows]
    if any(v is not None for v in b3v):
        print("| " + " | ".join(str(x) for x in arm_metrics(b3v, "B3 LLM 自检", None)) + " |")
    b4v = [r["B4"] for r in rows]
    if any(v is not None for v in b4v):
        print("| " + " | ".join(str(x) for x in arm_metrics(b4v, "B4 独立判官", None)) + " |")

    print("\n## 判分一致性（验证策略对比的核心量）")
    for name, key in [("B1 确定性门", "B1"), ("B3 LLM 自检", "B3"), ("B4 独立判官", "B4")]:
        judged = [r for r in rows if r[key] is not None]
        if not judged:
            continue
        agree = sum(1 for r in judged if bool(r[key]) == bool(r["Y"]))
        accept_wrong = sum(1 for r in judged if r[key] and not r["Y"])
        reject_right = sum(1 for r in judged if (not r[key]) and r["Y"])
        print(f"- {name}: 与真值一致 {agree}/{len(judged)}；误放（接受错交付）{accept_wrong}；误杀（拒绝对交付）{reject_right}")


if __name__ == "__main__":
    main()
