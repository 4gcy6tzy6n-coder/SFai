#!/usr/bin/env python3
"""Grand cross-model table: all providers' models in one comparison.

Merges: Bailian e2m-*.json (16 text models), MiMo e2-mimo-baseline.json,
DeepSeek e1-deepseek-baseline.json. Emits Markdown with per-suite gate-on
success/correctness/false-success + gate-off false-success + tokens.
"""
import json, glob
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "experiments" / "results"

SUITES = ["tool-use", "dataset", "long-horizon", "edge-case"]

def load_models():
    out = []
    # DeepSeek chat (E1) and MiMo (E2) carry provider labels in config.
    for name in ["e1-deepseek-baseline", "e2-mimo-baseline"]:
        p = RESULTS / f"{name}.json"
        if p.exists():
            d = json.loads(p.read_text())
            cfg = d.get("config") or {}
            out.append((f"{cfg.get('provider','?')}/{cfg.get('model','?')}", d))
    for p in sorted(RESULTS.glob("e2m-*.json")):
        d = json.loads(p.read_text())
        cfg = d.get("config") or {}
        out.append((f"bailian/{cfg.get('model', p.stem)}", d))
    return out

def cell(d, suite, ablation, key):
    for m in d.get("metrics", []):
        if m["suite"] == suite and m["ablation"] == ablation:
            return m.get(key, 0.0)
    return None

def main():
    models = load_models()
    print("| model | suite | suc(gate) | cor(gate) | **fs(gate)** | **fs(no gate)** | in tokens (gate) |")
    print("|---|---|---|---|---|---|---|")
    for label, d in models:
        for suite in SUITES:
            suc = cell(d, suite, "none", "success_rate")
            cor = cell(d, suite, "none", "correctness")
            fsg = cell(d, suite, "none", "false_success_rate")
            fsn = cell(d, suite, "no_verification", "false_success_rate")
            tok = cell(d, suite, "none", "mean_input_tokens")
            if suc is None:
                continue
            def f(x):
                return "—" if x is None else f"{x:.2f}"
            print(f"| {label} | {suite} | {f(suc)} | {f(cor)} | **{f(fsg)}** | **{f(fsn)}** | {tok or 0:.0f} |")
    # aggregate headline
    tot_on = tot_off = n_on = n_off = 0
    for _, d in models:
        for suite in SUITES:
            a = cell(d, suite, "none", "false_success_rate")
            b = cell(d, suite, "no_verification", "false_success_rate")
            if a is not None:
                tot_on += a; n_on += 1
            if b is not None:
                tot_off += b; n_off += 1
    print(f"\n**Aggregate:** gate-on mean false-success = {tot_on/max(1,n_on):.3f} ({n_on} cells) "
          f"| gate-off mean false-success = {tot_off/max(1,n_off):.3f} ({n_off} cells)")

if __name__ == "__main__":
    main()
