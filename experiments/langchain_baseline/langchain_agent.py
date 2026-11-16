#!/usr/bin/env python3
"""E7: LangChain/LangGraph end-to-end baseline on sfai benchmark suites.

Fairness contract (identical to the sfai arm):
- model: deepseek-chat via DeepSeek OpenAI-compatible API
- tools carry the SAME information as sfai's fake tools (3 canned claims with
  source:// ref ids; file read/write in a per-task workspace)
- deliverable checks are framework-agnostic:
    dataset:  answer.json contains the exact computed total
    tool-use: report.md cites >= 3 distinct source:// refs
- metrics: success/correctness/latency/tokens/cost in sfai BenchTaskResult shape

Usage: DEEPSEEK_API_KEY=... python3 langchain_agent.py [reps]
"""
import json, os, re, sys, time
from pathlib import Path

from langchain_core.tools import tool
from langchain_openai import ChatOpenAI
from langgraph.prebuilt import create_react_agent

REPS = int(sys.argv[1]) if len(sys.argv) > 1 else 2
SIZES = 3
OUT = Path(__file__).resolve().parents[1] / "results" / "e7-langchain-baseline.json"

CANNED = [
    ("Paper A", "source://src_lg_a@1", "Sparse context activation reduces end-to-end cost by 30% while preserving task success on long-horizon work."),
    ("Paper B", "source://src_lg_b@1", "Index-native context retains hard constraints across long sessions; full-history replay is unnecessary for most tasks."),
    ("Paper C", "source://src_lg_c@1", "Model-native routing matches specialist routers on success rate at lower latency, though evidence is mixed on rare domains."),
]

WORKSPACE = Path("/tmp/e7-langchain-ws")
WORKSPACE.mkdir(exist_ok=True)

@tool
def mock_search(query: str) -> str:
    """Search the web; results come back with their source:// ref ids."""
    return "\n".join(f"{title} [{ref}]: {text}" for title, ref, text in CANNED)

@tool
def write_file(path: str, content: str) -> str:
    """Write a file into the task workspace."""
    p = WORKSPACE / path
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(content)
    return f"wrote {path} ({len(content)} chars)"

@tool
def read_file(path: str) -> str:
    """Read a file from the task workspace."""
    p = WORKSPACE / path
    return p.read_text() if p.exists() else f"no such file: {path}"

TOOLS = [mock_search, write_file, read_file]
SYSTEM = ("You are an autonomous agent. Use tools as needed. Cite sources with their "
          "source:// ref ids in your final deliverable. Write deliverables via write_file.")

def dataset_rows(i: int, seed: int) -> list[int]:
    return [((i * 7 + row * 13 + seed) % 97 + 10) for row in range(8)]

def run_task(llm, goal: str, rules: list[str], spec_id: str) -> dict:
    agent = create_react_agent(llm, TOOLS, prompt=SYSTEM)
    t0 = time.perf_counter()
    usage = {"input": 0, "output": 0}
    try:
        result = agent.invoke({"messages": [("user", goal)]}, config={"recursion_limit": 25})
        for m in result["messages"]:
            meta = getattr(m, "response_metadata", {}) or {}
            um = (meta.get("token_usage") or meta.get("usage") or {})
            usage["input"] += um.get("prompt_tokens", 0) or um.get("input_tokens", 0)
            usage["output"] += um.get("completion_tokens", 0) or um.get("output_tokens", 0)
        status = "completed"
    except Exception as e:  # noqa: BLE001
        status = "error"
        print(f"  [{spec_id}] ERROR: {e}", file=sys.stderr)
    latency = (time.perf_counter() - t0) * 1000

    # framework-agnostic checker (key-name agnostic: the expected value must
    # appear as a JSON value anywhere in answer.json)
    correct = False
    if rules[0] == "answer.json":
        try:
            data = json.loads((WORKSPACE / "answer.json").read_text())
            values = data.values() if isinstance(data, dict) else [data]
            exp_f = float(rules[1])
            correct = False
            for v in values:
                try:
                    if abs(float(v) - exp_f) < 1e-9:
                        correct = True
                        break
                except (TypeError, ValueError):
                    continue
        except Exception:  # noqa: BLE001
            correct = False
    else:
        # filename-agnostic: any deliverable the agent wrote counts
        best = 0
        for f in WORKSPACE.iterdir():
            if f.is_file():
                try:
                    refs = set(re.findall(r"source://[A-Za-z0-9_\-]+@[0-9]+", f.read_text()))
                    best = max(best, len(refs))
                except Exception:  # noqa: BLE001
                    continue
        correct = best >= 3
    acceptance = correct
    false_success = status == "completed" and not (acceptance and correct)
    cost = usage["input"] / 1e6 * 0.27 + usage["output"] / 1e6 * 1.10
    return {"spec_id": spec_id, "suite": "dataset" if rules[0] == "answer.json" else "tool-use",
            "ablation": "none", "status": status, "acceptance_passed": acceptance,
            "correct": correct, "false_success": false_success, "latency_ms": latency,
            "input_tokens": usage["input"], "output_tokens": usage["output"],
            "cost_usd": cost, "error": None if status == "completed" else "run failed"}

def main():
    key = os.environ.get("DEEPSEEK_API_KEY")
    if not key:
        print("DEEPSEEK_API_KEY required", file=sys.stderr)
        return 2
    llm = ChatOpenAI(model="deepseek-chat", base_url="https://api.deepseek.com/v1",
                     api_key=key, temperature=0, timeout=180, max_retries=3)
    tasks = []
    for rep in range(REPS):
        for i in range(SIZES):
            total = sum(dataset_rows(i, rep))
            csv = "row,amount\n" + "\n".join(f"{r},{a}" for r, a in enumerate(dataset_rows(i, rep)))
            (WORKSPACE / "sales.csv").write_text(csv)
            tasks.append((
                f"data-{i:02d}-r{rep}",
                f"Compute the total revenue from sales.csv and write answer.json (case {i}, seed {rep}).",
                ["answer.json", str(total)],
            ))
            tasks.append((
                f"tool-{i:02d}-r{rep}",
                f"Find three recent sources on sparse context activation and write a cited report (case {i}, seed {rep}).",
                ["report.md", ""],
            ))
    rows = []
    for spec_id, goal, rules in tasks:
        print(f"running {spec_id} ...", file=sys.stderr)
        for f in WORKSPACE.iterdir():
            if f.name != "sales.csv":
                f.unlink()
        rows.append(run_task(llm, goal, rules, spec_id))

    metrics = []
    for suite in ["dataset", "tool-use"]:
        group = [r for r in rows if r["suite"] == suite]
        n = max(1, len(group))
        suc = sum(1 for r in group if r["status"] == "completed")
        cor = sum(1 for r in group if r["correct"])
        fs = sum(1 for r in group if r["false_success"])
        lats = sorted(r["latency_ms"] for r in group)
        metrics.append({
            "suite": suite, "ablation": "none", "tasks": len(group),
            "success_rate": suc / n, "correctness": cor / n, "false_success_rate": fs / n,
            "p50_latency_ms": lats[len(lats) // 2] if lats else 0.0,
            "p95_latency_ms": lats[min(len(lats) - 1, int(0.95 * len(lats)))] if lats else 0.0,
            "mean_input_tokens": sum(r["input_tokens"] for r in group) / n,
            "mean_output_tokens": sum(r["output_tokens"] for r in group) / n,
            "mean_cost_usd": sum(r["cost_usd"] for r in group) / n,
        })
    report = {
        "version": "0.2.1", "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "suites": ["dataset", "tool-use"], "ablations": ["none"], "size": SIZES,
        "repetitions": REPS, "seed": 0,
        "config": {"name": "e7-langchain-baseline", "provider": "deepseek",
                   "model": "deepseek-chat", "note": "LangGraph ReAct + langchain-openai; same model/tools-info/checker as sfai arm"},
        "metrics": metrics, "tasks": rows,
        "notes": "Cross-framework E7 baseline; see docs/benchmark-spec.md fairness contract.",
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, indent=2))
    print(json.dumps(metrics, indent=2))
    print(f"written: {OUT}", file=sys.stderr)
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
