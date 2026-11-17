/* sfai real backend client — talks to `sfai serve` (Decide-Act runtime).
   Real mode activates when a backend base URL is configured:
   - desktop app injects window.__SFAI_BACKEND__
   - or settings.endpoint points at a running `sfai serve`.
   All calls are optional-layer: without a backend the UI keeps its demo mode. */
(function (global) {
  "use strict";

  function base() {
    if (global.__SFAI_BACKEND__) return String(global.__SFAI_BACKEND__).replace(/\/$/, "");
    var settings = (global.SFAIStore && global.SFAIStore.getState().settings) || {};
    return settings.endpoint ? String(settings.endpoint).replace(/\/$/, "") : "";
  }

  function enabled() {
    return !!base();
  }

  function headers() {
    return { "Content-Type": "application/json" };
  }

  async function json(method, path, body) {
    var res = await fetch(base() + path, {
      method: method,
      headers: headers(),
      body: body ? JSON.stringify(body) : undefined
    });
    var data = await res.json().catch(function () { return {}; });
    if (!res.ok) throw new Error(data.error || ("http " + res.status));
    return data;
  }

  /* Map a real ledger event to the UI event shape (defensive: unknown → SYSTEM). */
  function mapLedgerEvent(ev, turn) {
    var p = ev.payload || {};
    if (ev.event_type === "tool.committed") {
      return {
        kind: "TOOL",
        turn: turn,
        tool: p.operation_id || "tool",
        title: (p.status === "ok" ? "工具执行 · " : "工具回执 · ") + (p.operation_id || ""),
        status: p.status === "ok" ? "ok" : (p.status === "denied" ? "blocked" : "error"),
        args: p.args_hash ? "{\"args_hash\":\"" + p.args_hash + "\"}" : "{}",
        result: (p.result_preview || "") + (p.result_refs && p.result_refs.length
          ? "\n" + p.result_refs.map(function (r) { return "ref: " + r; }).join("\n") : "")
      };
    }
    if (ev.event_type === "verification.recorded") {
      return {
        kind: "VERIFY",
        turn: turn,
        title: "核验 · " + (p.criterion || "acceptance"),
        status: p.verdict === "pass" ? "ok" : "error",
        args: JSON.stringify({ criterion: p.criterion, target: p.target_ref }),
        result: p.findings || p.verdict || ""
      };
    }
    if (ev.event_type === "task.status_changed") {
      return {
        kind: "SYSTEM",
        turn: turn,
        title: "任务状态",
        text: "status → " + (p.status || "?") + (p.reason ? "（" + p.reason + "）" : "")
      };
    }
    if (ev.event_type === "task.output_added") {
      return { kind: "TOOL", turn: turn, tool: "artifact", title: "产出工件", status: "ok", args: "{}", result: String(p.ref || "") };
    }
    return null;
  }

  global.SFAIApi = {
    enabled: enabled,
    base: base,
    createTask: function (goal, constraints, acceptance) {
      return json("POST", "/api/tasks", {
        goal: goal, constraints: constraints || [], acceptance_rules: acceptance || []
      });
    },
    runTask: function (taskId, opts) {
      return json("POST", "/api/tasks/" + taskId + "/run", opts || {});
    },
    providers: function () { return json("GET", "/api/providers"); },
    events: function (taskId) { return json("GET", "/api/tasks/" + taskId + "/events"); },
    mapLedgerEvent: mapLedgerEvent
  };
})(window);
