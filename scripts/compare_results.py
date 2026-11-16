#!/usr/bin/env python3
"""Merge benchmark result JSONs into a comparison table (E2 provider compare).

Usage: python3 scripts/compare_results.py experiments/results/*.json
Prints a Markdown table: rows = suite/ablation, columns = one result file each.
"""

import json
import sys
from pathlib import Path


def load(path: str):
    data = json.loads(Path(path).read_text())
    config = data.get("config") or {}
    label = config.get("name") or Path(path).stem
    rows = {}
    for m in data.get("metrics", []):
        key = (m["suite"], m["ablation"])
        rows[key] = m
    return label, rows, data


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    loaded = [load(p) for p in argv[1:]]
    labels = [label for label, _, _ in loaded]
    keys = sorted({k for _, rows, _ in loaded for k in rows})

    print("| suite | ablation | " + " | ".join(labels) + " |")
    print("|---|---|" + "---|" * len(labels))
    for key in keys:
        suite, ablation = key
        cells = []
        for _, rows, _ in loaded:
            m = rows.get(key)
            if m is None:
                cells.append("—")
            else:
                cells.append(
                    f"acc {m['correctness']:.2f} | suc {m['success_rate']:.2f} "
                    f"| fs {m['false_success_rate']:.2f} | ${m['mean_cost_usd']:.5f} "
                    f"| {m['p50_latency_ms']:.0f}ms"
                )
        print(f"| {suite} | {ablation} | " + " | ".join(cells) + " |")

    print("\n## Run metadata\n")
    for label, _, data in loaded:
        config = data.get("config") or {}
        print(
            f"- **{label}**: provider={config.get('provider') or 'scripted'} "
            f"model={config.get('model') or '-'} seed={data.get('seed')} "
            f"repetitions={data.get('repetitions')} at {data.get('generated_at')}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
