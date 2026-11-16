#!/usr/bin/env python3
"""E6: pure-Python orchestration overhead baseline.

Simulates the same per-step work the sfai kernel does for one scripted task:
state load, envelope JSON build + pydantic-style validation, tool dispatch,
SQLite event append (WAL), state fold. Measures ms/task and ms/step.
Run: python3 scripts/bench_python_overhead.py [tasks] [steps]
"""
import json
import random
import sqlite3
import string
import sys
import time
from dataclasses import dataclass, asdict, field

TASKS = int(sys.argv[1]) if len(sys.argv) > 1 else 200
STEPS = int(sys.argv[2]) if len(sys.argv) > 2 else 5


@dataclass
class ActionIntent:
    kind: str
    task_id: str
    reason_code: str = "unspecified"
    payload: dict = field(default_factory=dict)
    expected_refs: list = field(default_factory=list)
    idempotency_key: str | None = None

    def validate(self):
        assert self.kind in {"discover", "retrieve", "activate", "execute",
                             "verify", "respond", "ask_user", "pause", "finish"}
        assert self.task_id


@dataclass
class TaskState:
    task_id: str
    status: str = "active"
    completed_outputs: list = field(default_factory=list)
    tool_calls_made: int = 0
    model_calls_made: int = 0


def make_envelope(goal: str, refs: list[str]) -> str:
    env = {
        "goal": goal,
        "pinned_constraints": ["final output must cite sources"],
        "task_state": {"current_status": "active", "tool_calls_made": 2},
        "root_index": {"task": f"task://t/index@1", "sources": "task://t/sources@1"},
        "active_refs": refs,
        "recent_results": [{"operation_id": "web.search", "result_preview": "x" * 400,
                            "result_refs": refs[:2], "status": "ok"} for _ in range(5)],
        "budget": {"active_context_tokens": 10000, "tool_calls_remaining": 15},
    }
    return json.dumps(env)


def fold_event(state: TaskState, ev: dict):
    if ev["event_type"] == "tool.committed":
        state.tool_calls_made += 1
    elif ev["event_type"] == "task.intent_started":
        state.model_calls_made += 1
    elif ev["event_type"] == "task.output_added":
        state.completed_outputs.append(ev["payload"]["ref"])


def main():
    conn = sqlite3.connect(":memory:")
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("""CREATE TABLE events (seq INTEGER PRIMARY KEY AUTOINCREMENT,
        idempotency_key TEXT UNIQUE, task_id TEXT, actor TEXT, event_type TEXT,
        timestamp TEXT, payload TEXT)""")
    conn.commit()

    goal = "Find three recent sources and write a cited report"
    refs = [f"source://src_{i:04d}@1" for i in range(8)]
    random.seed(0)

    t0 = time.perf_counter()
    for t in range(TASKS):
        task_id = f"task_{t:04d}"
        state = TaskState(task_id)
        for step in range(STEPS):
            # 1) envelope build + serialize
            env_text = make_envelope(goal, refs)
            # 2) "model" response validation (pydantic-equivalent work)
            raw = {"kind": "execute", "task_id": task_id,
                   "payload": {"operation_id": "web.search",
                               "arguments": {"query": "sparse context"}}}
            intent = ActionIntent(**raw)
            intent.validate()
            # 3) dispatch + ledger append
            conn.execute(
                "INSERT INTO events(idempotency_key, task_id, actor, event_type, timestamp, payload)"
                " VALUES(?,?,?,?,?,?)",
                (f"{task_id}:s{step}", task_id, "tool:web.search", "tool.committed",
                 "2026-01-01T00:00:00Z",
                 json.dumps({"operation_id": "web.search", "status": "ok",
                             "result_refs": random.sample(refs, 2)})))
            # 4) state fold
            fold_event(state, {"event_type": "tool.committed", "payload": {}})
            fold_event(state, {"event_type": "task.intent_started", "payload": {}})
        conn.commit()
    dt = time.perf_counter() - t0

    ms_task = dt * 1000 / TASKS
    ms_step = dt * 1000 / (TASKS * STEPS)
    print(json.dumps({
        "runtime": f"python ({sys.version.split()[0]})",
        "tasks": TASKS, "steps_per_task": STEPS,
        "total_ms": round(dt * 1000, 1),
        "ms_per_task": round(ms_task, 3),
        "ms_per_step": round(ms_step, 3),
        "tasks_per_sec": round(TASKS / dt, 1),
    }, indent=2))


if __name__ == "__main__":
    main()
