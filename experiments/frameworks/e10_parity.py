#!/usr/bin/env python3
"""E10 parity experiment (supervisor risk #3): LangGraph + EQUAL verifier.

Compares, on identical tasks (evals/public/tasks.json suite public-e10parity),
identical model, identical checker, identical repair budget (2 rounds, matching
sfai MAX_ACCEPTANCE_RETRIES):

  1. langgraph nogate  — ReAct agent, claims success unconditionally
  2. langgraph gate    — same agent + sfai-identical acceptance rules + up to
                         2 repair rounds; claims success only on acceptance pass

Usage:
  python3 e10_parity.py nogate [out.json]
  python3 e10_parity.py gate   [out.json]

Provider via env: SFAI_E10_BASE_URL, SFAI_E10_API_KEY, SFAI_E10_MODEL.
"""
import json
import os
import re
import sys
import time
from pathlib import Path

from langchain_core.messages import HumanMessage, SystemMessage
from langchain_openai import ChatOpenAI
from langgraph.prebuilt import create_react_agent

ROOT = Path(__file__).resolve().parents[2]
TASKS = json.loads((ROOT / "evals" / "public" / "tasks.json").read_text())
SUITE = [t for t in TASKS if t["suite"] == "public-e10parity"]
REPAIR_BUDGET = 2  # == sfai MAX_ACCEPTANCE_RETRIES

BASE_URL = os.environ.get("SFAI_E10_BASE_URL", "https://api.deepseek.com/v1")
API_KEY = os.environ.get("SFAI_E10_API_KEY") or os.environ.get("DEEPSEEK_API_KEY", "")
MODEL = os.environ.get("SFAI_E10_MODEL", "deepseek-chat")
PRICE_IN = float(os.environ.get("SFAI_E10_PRICE_IN", "0"))
PRICE_OUT = float(os.environ.get("SFAI_E10_PRICE_OUT", "0"))

WORKSPACE = Path("/tmp/e10-parity-ws")

CANNED = [
    ("Paper A", "source://src_lg_a@1", "Sparse context activation reduces end-to-end cost by 30% while preserving task success on long-horizon work."),
    ("Paper B", "source://src_lg_b@1", "Index-native context retains hard constraints across long sessions; full-history replay is unnecessary for most tasks."),
    ("Paper C", "source://src_lg_c@1", "Model-native routing matches specialist routers on success rate at lower latency, though evidence is mixed on rare domains."),
]


def make_tools():
    from langchain_core.tools import tool

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

    return [mock_search, write_file, read_file]


SYSTEM = ("You are an autonomous agent. Use tools as needed. Cite sources with their "
          "source:// ref ids in your final deliverable. Write deliverables via write_file.")


def seed_files(files):
    WORKSPACE.mkdir(exist_ok=True)
    for f in WORKSPACE.iterdir():
        f.unlink()
    for path, content in (files or {}).items():
        (WORKSPACE / path).write_text(content)


# ---- acceptance rules: same semantics as sfai verify engine criteria ----
def check_acceptance(rules):
    """Return (passed, failed_rule). Mirrors sfai's criteria for these tasks."""
    for rule in rules:
        name, _, arg = rule.partition(":")
        if name == "artifact_exists":
            if not (WORKSPACE / arg).exists():
                return False, rule
        elif name == "json_valid":
            try:
                json.loads((WORKSPACE / arg).read_text())
            except Exception:  # noqa: BLE001
                return False, rule
        elif name == "artifact_contains":
            path, _, needle = arg.partition(":")
            try:
                if needle not in (WORKSPACE / path).read_text():
                    return False, rule
            except Exception:  # noqa: BLE001
                return False, rule
        elif name == "source_count_at_least":
            path, _, k = arg.partition(":")
            try:
                refs = set(re.findall(r"source://[A-Za-z0-9_\-]+@[0-9]+", (WORKSPACE / path).read_text()))
                if len(refs) < int(k):
                    return False, rule
            except Exception:  # noqa: BLE001
                return False, rule
    return True, None


def shared_correctness(task):
    """Framework-agnostic checker — identical for all arms."""
    gt = task["ground_truth"]
    if task["id"].startswith("e10-data"):
        try:
            data = json.loads((WORKSPACE / "answer.json").read_text())
            values = data.values() if isinstance(data, dict) else [data]
            exp_f = float(gt)
            for v in values:
                try:
                    if abs(float(v) - exp_f) < 1e-9:
                        return True
                except (TypeError, ValueError):
                    continue
        except Exception:  # noqa: BLE001
            return False
        return False
    best = 0
    for f in WORKSPACE.iterdir():
        if f.is_file():
            try:
                refs = set(re.findall(r"source://[A-Za-z0-9_\-]+@[0-9]+", f.read_text()))
                best = max(best, len(refs))
            except Exception:  # noqa: BLE001
                continue
    return best >= 3


def run_one(llm, task, mode):
    seed_files(task.get("files"))
    agent = create_react_agent(llm, make_tools(), prompt=SYSTEM)
    usage = {"input": 0, "output": 0}
    t0 = time.perf_counter()
    repairs = 0
    status = "completed"
    try:
        result = agent.invoke({"messages": [("user", task["goal"])]}, config={"recursion_limit": 25})
        msgs = list(result["messages"])
    except Exception as e:  # noqa: BLE001
        msgs = []
        status = "error"
        print(f"  [{task['id']}] ERROR: {e}", file=sys.stderr)

    def harvest(messages):
        for m in messages:
            meta = getattr(m, "response_metadata", {}) or {}
            um = meta.get("token_usage") or meta.get("usage") or {}
            usage["input"] += um.get("prompt_tokens", 0) or um.get("input_tokens", 0)
            usage["output"] += um.get("completion_tokens", 0) or um.get("output_tokens", 0)

    harvest(msgs)

    claimed = status == "completed"
    acceptance, failed_rule = check_acceptance(task["acceptance_rules"]) if claimed else (False, "run-error")

    if mode == "gate":
        while claimed and not acceptance and repairs < REPAIR_BUDGET:
            repairs += 1
            feedback = (f"Your deliverable failed verification: {failed_rule}. "
                        "Inspect the workspace and fix the deliverable.")
            try:
                result = agent.invoke(
                    {"messages": [("user", task["goal"])] +
                                 [(m.type, m.content) for m in msgs if m.type in ("ai", "tool", "human")] +
                                 [("user", feedback)]},
                    config={"recursion_limit": 25},
                )
                msgs = list(result["messages"])
                harvest(msgs)
            except Exception as e:  # noqa: BLE001
                print(f"  [{task['id']}] repair {repairs} ERROR: {e}", file=sys.stderr)
                break
            acceptance, failed_rule = check_acceptance(task["acceptance_rules"])

        # gated claim: success only if acceptance passes
        final_status = "completed" if acceptance else "blocked"
    else:
        final_status = status

    correct = shared_correctness(task)
    false_success = final_status == "completed" and not correct
    cost = usage["input"] / 1e6 * PRICE_IN + usage["output"] / 1e6 * PRICE_OUT
    return {
        "task_id": task["id"], "suite": "dataset" if task["id"].startswith("e10-data") else "tool-use",
        "ablation": f"langgraph_{mode}", "status": final_status,
        "acceptance_passed": acceptance, "correct": correct,
        "false_success": false_success, "repairs": repairs,
        "latency_ms": (time.perf_counter() - t0) * 1000,
        "input_tokens": usage["input"], "output_tokens": usage["output"], "cost_usd": cost,
    }


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "nogate"
    out = Path(sys.argv[2]) if len(sys.argv) > 2 else ROOT / "experiments" / "results" / f"e10-langgraph-{mode}.json"
    if not API_KEY:
        print("API key required (SFAI_E10_API_KEY or DEEPSEEK_API_KEY)", file=sys.stderr)
        return 2
    llm = ChatOpenAI(model=MODEL, base_url=BASE_URL, api_key=API_KEY,
                     temperature=0, timeout=240, max_retries=3)
    rows = []
    for task in SUITE:
        print(f"[langgraph-{mode}] {task['id']} ...", file=sys.stderr)
        rows.append(run_one(llm, task, mode))

    n = len(rows)
    completed = sum(1 for r in rows if r["status"] == "completed")
    correct = sum(1 for r in rows if r["correct"])
    fs = sum(1 for r in rows if r["false_success"])
    report = {
        "config": {"name": f"e10-langgraph-{mode}", "provider": BASE_URL, "model": MODEL,
                   "repair_budget": REPAIR_BUDGET, "note": "E10 parity: equal verifier/repair budget/permissions"},
        "tasks": rows,
        "summary": {
            "n": n, "completion_rate": completed / n, "correctness": correct / n,
            "fs_over_all": fs / n,
            "fs_over_claimed": (fs / completed) if completed else None,
            "mean_repairs": sum(r["repairs"] for r in rows) / n,
            "repair_success": sum(1 for r in rows if r["repairs"] > 0 and r["acceptance_passed"]),
        },
    }
    out.write_text(json.dumps(report, indent=1))
    print(json.dumps(report["summary"], indent=1))
    print(f"written: {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
