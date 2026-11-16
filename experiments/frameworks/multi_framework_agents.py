#!/usr/bin/env python3
"""E7-ext: recent-framework baselines (smolagents / pydantic-ai) on sfai suites.

Same fairness contract as langchain_agent.py: deepseek-chat, same goals/seeds,
same tool information (3 canned claims with source:// ref ids), same
framework-agnostic checker (answer-value match / >=3 distinct source:// refs).

Usage: DEEPSEEK_API_KEY=... python3 multi_framework_agents.py [framework]
       framework: smolagents | pydantic-ai | all
"""
import json, os, re, sys, time
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
OUT_DIR = REPO / "experiments" / "results"
WORKSPACE = Path("/tmp/e7-frameworks-ws")
WORKSPACE.mkdir(exist_ok=True)

CANNED = [
    ("Paper A", "source://src_lg_a@1", "Sparse context activation reduces end-to-end cost by 30% while preserving task success on long-horizon work."),
    ("Paper B", "source://src_lg_b@1", "Index-native context retains hard constraints across long sessions; full-history replay is unnecessary for most tasks."),
    ("Paper C", "source://src_lg_c@1", "Model-native routing matches specialist routers on success rate at lower latency, though evidence is mixed on rare domains."),
]

def dataset_rows(i: int, seed: int):
    return [((i * 7 + row * 13 + seed) % 97 + 10) for row in range(8)]

def check(rules):
    expected, kind = rules[0], rules[1]
    try:
        if kind == "dataset":
            data = json.loads((WORKSPACE / "answer.json").read_text())
            values = data.values() if isinstance(data, dict) else [data]
            exp_f = float(expected)
            for v in values:
                try:
                    if abs(float(v) - exp_f) < 1e-9:
                        return True
                except (TypeError, ValueError):
                    continue
            return False
        text = (WORKSPACE / "report.md").read_text()
        refs = set(re.findall(r"source://[A-Za-z0-9_\-]+@[0-9]+", text))
        return len(refs) >= 3
    except Exception:  # noqa: BLE001
        return False

def row_for(spec_id, suite, status, correct, latency, usage_in, usage_out, err=None):
    acceptance = correct
    return {"spec_id": spec_id, "suite": suite, "ablation": "none",
            "status": status, "acceptance_passed": acceptance, "correct": correct,
            "false_success": status == "completed" and not (acceptance and correct),
            "latency_ms": latency, "input_tokens": usage_in, "output_tokens": usage_out,
            "cost_usd": usage_in / 1e6 * 0.27 + usage_out / 1e6 * 1.10, "error": err}

def run_smolagents(goal, rules, spec_id):
    from smolagents import CodeAgent, OpenAIServerModel, tool

    @tool
    def mock_search(query: str) -> str:
        """Search the web; results come back with their source:// ref ids.

        Args:
            query: search query string.
        """
        return "\n".join(f"{t} [{r}]: {x}" for t, r, x in CANNED)

    @tool
    def write_file(path: str, content: str) -> str:
        """Write a file into the task workspace.

        Args:
            path: relative file path.
            content: full file content.
        """
        p = WORKSPACE / path
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(content)
        return f"wrote {path} ({len(content)} chars)"

    @tool
    def read_file(path: str) -> str:
        """Read a file from the task workspace.

        Args:
            path: relative file path.
        """
        p = WORKSPACE / path
        return p.read_text() if p.exists() else f"no such file: {path}"

    model = OpenAIServerModel(
        model_id="deepseek-chat",
        api_base="https://api.deepseek.com/v1",
        api_key=os.environ["DEEPSEEK_API_KEY"],
    )
    agent = CodeAgent(tools=[mock_search, write_file, read_file], model=model, max_steps=12)
    t0 = time.perf_counter()
    status, err = "completed", None
    try:
        agent.run(goal)
    except Exception as e:  # noqa: BLE001
        status, err = "error", str(e)[:200]
    latency = (time.perf_counter() - t0) * 1000
    mon = getattr(agent, "monitor", None)
    usage_in = int(getattr(mon, "total_input_token_count", 0) or 0)
    usage_out = int(getattr(mon, "total_output_token_count", 0) or 0)
    correct = check(rules)
    return row_for(spec_id, rules[2], status, correct, latency, usage_in, usage_out, err)

def run_pydantic_ai(goal, rules, spec_id):
    from pydantic_ai import Agent
    from pydantic_ai.models.openai import OpenAIChatModel
    from pydantic_ai.providers.openai import OpenAIProvider

    model = OpenAIChatModel(
        "deepseek-chat",
        provider=OpenAIProvider(
            base_url="https://api.deepseek.com/v1",
            api_key=os.environ["DEEPSEEK_API_KEY"],
        ),
    )
    agent = Agent(model, instructions=(
        "You are an autonomous agent. Use tools as needed. Cite sources with their "
        "source:// ref ids in your final deliverable. Write deliverables via write_file."))

    @agent.tool_plain
    def mock_search(query: str) -> str:
        """Search the web; results come back with their source:// ref ids."""
        return "\n".join(f"{t} [{r}]: {x}" for t, r, x in CANNED)

    @agent.tool_plain
    def write_file(path: str, content: str) -> str:
        """Write a file into the task workspace."""
        p = WORKSPACE / path
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(content)
        return f"wrote {path} ({len(content)} chars)"

    @agent.tool_plain
    def read_file(path: str) -> str:
        """Read a file from the task workspace."""
        p = WORKSPACE / path
        return p.read_text() if p.exists() else f"no such file: {path}"

    t0 = time.perf_counter()
    status, err, usage_in, usage_out = "completed", None, 0, 0
    try:
        result = agent.run_sync(goal)
        u = getattr(result, "usage", None)
        if callable(u):
            u = u()
        usage_in = int(getattr(u, "input_tokens", 0) or getattr(u, "request_tokens", 0) or 0)
        usage_out = int(getattr(u, "output_tokens", 0) or getattr(u, "response_tokens", 0) or 0)
    except Exception as e:  # noqa: BLE001
        status, err = "error", str(e)[:200]
    latency = (time.perf_counter() - t0) * 1000
    correct = check(rules)
    return row_for(spec_id, rules[2], status, correct, latency, usage_in, usage_out, err)

RUNNERS = {"smolagents": run_smolagents, "pydantic-ai": run_pydantic_ai}

def tasks():
    out = []
    for rep in range(2):
        for i in range(3):
            total = sum(dataset_rows(i, rep))
            csv = "row,amount\n" + "\n".join(f"{r},{a}" for r, a in enumerate(dataset_rows(i, rep)))
            (WORKSPACE / "sales.csv").write_text(csv)
            out.append((f"data-{i:02d}-r{rep}",
                        f"Compute the total revenue from sales.csv and write answer.json (case {i}, seed {rep}).",
                        (str(total), "dataset", "dataset")))
            out.append((f"tool-{i:02d}-r{rep}",
                        f"Find three recent sources on sparse context activation and write a cited report (case {i}, seed {rep}).",
                        ("3-refs", "tool-use", "tool-use")))
    return out

def main():
    which = sys.argv[1] if len(sys.argv) > 1 else "all"
    names = list(RUNNERS) if which == "all" else [which]
    for name in names:
        rows = []
        for spec_id, goal, rules in tasks():
            print(f"[{name}] {spec_id} ...", file=sys.stderr)
            rows.append(RUNNERS[name](goal, rules, spec_id))
        metrics = []
        for suite in ["dataset", "tool-use"]:
            group = [r for r in rows if r["suite"] == suite]
            n = max(1, len(group))
            lats = sorted(r["latency_ms"] for r in group)
            metrics.append({
                "suite": suite, "ablation": "none", "tasks": len(group),
                "success_rate": sum(1 for r in group if r["status"] == "completed") / n,
                "correctness": sum(1 for r in group if r["correct"]) / n,
                "false_success_rate": sum(1 for r in group if r["false_success"]) / n,
                "p50_latency_ms": lats[len(lats) // 2] if lats else 0.0,
                "mean_input_tokens": sum(r["input_tokens"] for r in group) / n,
                "mean_output_tokens": sum(r["output_tokens"] for r in group) / n,
                "mean_cost_usd": sum(r["cost_usd"] for r in group) / n,
            })
        report = {"version": "0.2.1",
                  "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                  "suites": ["dataset", "tool-use"], "ablations": ["none"], "size": 3,
                  "repetitions": 2, "seed": 0,
                  "config": {"name": f"e7ext-{name}", "provider": "deepseek",
                             "model": "deepseek-chat",
                             "note": f"{name} baseline; same fairness contract as E7"},
                  "metrics": metrics, "tasks": rows,
                  "notes": "E7-ext framework baseline; see e7-cross-framework-headtohead.md contract."}
        path = OUT_DIR / f"e7ext-{name}.json"
        path.write_text(json.dumps(report, indent=2))
        print(json.dumps(metrics, indent=2))
        print(f"written: {path}", file=sys.stderr)

if __name__ == "__main__":
    main()
