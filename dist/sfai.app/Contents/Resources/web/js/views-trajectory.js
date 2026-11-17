/* Trajectory workspace: summary, activity ruler, event ledger. */
(function () {
  "use strict";

  var SFAI = (window.SFAI = window.SFAI || {});
  var U = SFAI.util;
  var views = (SFAI.views = SFAI.views || {});
  var icon = function (name, cls) { return SFAI.icons.icon(name, cls); };

  var filters = [
    { id: "ALL", label: "全部" },
    { id: "INPUT", label: "输入" },
    { id: "ASSISTANT", label: "模型" },
    { id: "TOOL", label: "工具" },
    { id: "APPROVAL", label: "审批" },
    { id: "VERIFICATION", label: "核验" }
  ];

  function matchesFilter(event, filter) {
    if (filter === "ALL") return true;
    if (filter === "INPUT") return ["SYSTEM", "CONTEXT", "USER"].indexOf(event.kind) >= 0;
    return event.kind === filter;
  }

  function filterLabel(filter) {
    var found = filters.find(function (item) { return item.id === filter; });
    return found ? found.label : "全部";
  }

  function buildRuler(events) {
    var total = events.reduce(function (max, event) { return Math.max(max, event.seq); }, 1);
    var rows = [
      { key: "input", y: 4, kinds: ["SYSTEM", "CONTEXT", "USER"] },
      { key: "model", y: 26, kinds: ["ASSISTANT"] },
      { key: "tool", y: 48, kinds: ["TOOL", "APPROVAL", "VERIFICATION"] }
    ];
    var width = 744;
    var slot = width / total;
    var marks = rows.map(function (row) {
      var rects = events.filter(function (event) {
        return row.kinds.indexOf(event.kind) >= 0;
      }).map(function (event) {
        var x = 8 + (event.seq - 1) * slot;
        var w = Math.max(4, slot - 3);
        var cls = "ruler-seg--" + row.key;
        if (row.key === "tool") {
          if (event.kind === "APPROVAL") cls = "ruler-seg--approval";
          else if (event.verdict === "fail" || event.status === "fail") cls = "ruler-seg--failure";
        }
        var label = event.kind + " · " + (event.title || event.tool || "");
        return '<rect class="' + cls + '" x="' + x.toFixed(2) + '" y="' + row.y + '" width="' + w.toFixed(2) +
          '" height="10" rx="1"><title>' + U.escapeHtml(label) + "</title></rect>";
      }).join("");
      return '<line class="ruler-base" x1="8" y1="' + (row.y + 12) + '" x2="752" y2="' + (row.y + 12) + '" vector-effect="non-scaling-stroke"/>' + rects;
    }).join("");
    return '<svg class="ruler-chart" viewBox="0 0 760 64" preserveAspectRatio="none" role="img" aria-label="按步骤展开的输入、模型与工具活动">' +
      marks + "</svg>";
  }

  function traceMeta(event) {
    if (event.kind === "TOOL") {
      var status = U.toolStatusMeta(event.status);
      return status.label + (event.duration ? " · " + event.duration : "");
    }
    if (event.kind === "VERIFICATION") return event.verdict === "fail" ? "未通过" : "通过";
    if (event.kind === "APPROVAL") {
      var approval = SFAI.store.getApproval(event.approvalId);
      if (!approval) return "审批";
      return approval.status === "pending" ? "待审批" : approval.status === "approved" ? "已批准" : "已拒绝";
    }
    if (event.status === "running") return "运行中";
    if (event.status === "paused") return "已暂停";
    return "";
  }

  function traceTitle(event) {
    if (event.kind === "TOOL") {
      return "<strong>" + U.escapeHtml(event.tool || "tool") + "</strong><span>" + U.escapeHtml(event.summary || event.title || "") + "</span>";
    }
    if (event.kind === "ASSISTANT") {
      return (event.intent ? '<span class="event-intent">' + U.escapeHtml(event.intent) + "</span>" : "") +
        "<span>" + U.escapeHtml(event.title || event.text || "") + "</span>";
    }
    if (event.kind === "VERIFICATION") {
      return "<strong>" + U.escapeHtml(event.verdict === "fail" ? "verify.run" : "verify.run") + "</strong><span>" +
        U.escapeHtml(event.title || event.text || "") + "</span>";
    }
    return "<span>" + U.escapeHtml(event.title || event.text || event.kind) + "</span>";
  }

  function traceRow(event) {
    var role = U.roleMeta(event.kind);
    var dotClass = role.dotClass;
    if (event.verdict === "fail" || event.status === "fail" || event.status === "denied") dotClass = "dot--danger";
    return '<span class="trace-seq">' + String(event.seq).padStart(2, "0") + "</span>" +
      '<span class="trace-kind"><span class="dot ' + dotClass + '" aria-hidden="true"></span>' + U.escapeHtml(role.label) + "</span>" +
      '<span class="trace-title">' + traceTitle(event) + "</span>" +
      '<span class="trace-meta">' + U.escapeHtml(traceMeta(event)) + "</span>" +
      '<time class="trace-time" datetime="' + U.escapeHtml(event.time || "") + '">' + U.escapeHtml(event.time || "") + "</time>";
  }

  function traceEvent(event) {
    if (event.kind === "TOOL") {
      return '<details class="trace-details">' +
        '<summary><span class="trace-row">' + traceRow(event) + "</span></summary>" +
        '<div class="trace-detail-body">' +
          '<div><span class="tool-kv-label">arguments</span><pre>' + U.escapeHtml(event.args || "{}") + "</pre></div>" +
          '<div><span class="tool-kv-label">result</span><pre>' + U.escapeHtml(event.result || "—") + "</pre></div>" +
        "</div>" +
      "</details>";
    }
    return '<div class="trace-row">' + traceRow(event) + "</div>";
  }

  views.renderTrajectoryView = function (task, state) {
    if (!task) {
      return '<div class="page"><div class="empty"><div class="empty-icon">' + icon("trajectory") + "</div><h2>没有可查看的轨迹</h2><p>先创建一个任务，事件账本会在这里展开。</p></div></div>";
    }
    var events = SFAI.store.eventsFor(task.id);
    var filter = state.traceFilter || "ALL";
    var filtered = events.filter(function (event) { return matchesFilter(event, filter); });
    var groups = {};
    filtered.forEach(function (event) {
      var turn = event.turn || 1;
      groups[turn] = groups[turn] || [];
      groups[turn].push(event);
    });
    var turns = Object.keys(groups).sort(function (a, b) { return Number(a) - Number(b); });
    var list = turns.length
      ? turns.map(function (turn) {
        return '<section class="trace-turn" aria-label="第 ' + turn + ' 轮">' +
          '<div class="trace-turn-head"><span>TURN ' + String(turn).padStart(2, "0") + "</span><span>" + groups[turn].length + " steps</span></div>" +
          groups[turn].map(traceEvent).join("") +
        "</section>";
      }).join("")
      : '<div class="trace-empty">当前筛选下没有事件。选择“全部”恢复完整轨迹。</div>';

    return '<div class="page page--wide">' +
      '<header class="page-head">' +
        '<div class="page-head-main">' +
          '<div class="eyebrow-row"><span class="eyebrow">TRAJECTORY</span><span class="eyebrow-sep" aria-hidden="true"></span><span class="eyebrow">' + U.escapeHtml(task.id) + "</span></div>" +
          "<h2>执行轨迹</h2>" +
          '<p class="page-sub">每一次决策、工具调用与核验收据都来自同一份追加事件账本。轨迹是只读的。</p>' +
        "</div>" +
        '<div class="head-actions"><button class="btn" type="button" data-action="export-task" data-id="' + U.escapeHtml(task.id) + '">' + icon("download") + "导出轨迹</button></div>" +
      "</header>" +
      '<div class="stat-strip" aria-label="轨迹摘要">' +
        '<div class="stat"><span class="stat-value">' + U.escapeHtml(task.stats.duration) + '</span><span class="stat-label">duration</span></div>' +
        '<div class="stat"><span class="stat-value">' + task.stats.turns + '</span><span class="stat-label">turns</span></div>' +
        '<div class="stat"><span class="stat-value">' + task.stats.steps + '</span><span class="stat-label">steps</span></div>' +
        '<div class="stat"><span class="stat-value">' + U.money(task.budget.used) + '</span><span class="stat-label">cost</span></div>' +
      "</div>" +
      '<section class="ruler-block" aria-labelledby="ruler-title">' +
        '<div class="ruler-top"><h2 id="ruler-title">活动分布</h2><span>' + events.length + " events · " + U.escapeHtml(filterLabel(filter)) + "</span></div>" +
        '<div class="ruler">' +
          '<div class="ruler-labels"><span>INPUT</span><span>MODEL</span><span>TOOLS</span></div>' +
          buildRuler(events) +
        "</div>" +
        '<div class="ruler-legend">' +
          '<span class="legend-item"><span class="swatch" aria-hidden="true"></span>输入</span>' +
          '<span class="legend-item"><span class="swatch swatch--model" aria-hidden="true"></span>模型</span>' +
          '<span class="legend-item"><span class="swatch swatch--tool" aria-hidden="true"></span>工具</span>' +
          '<span class="legend-item"><span class="swatch swatch--approval" aria-hidden="true"></span>审批</span>' +
        "</div>" +
      "</section>" +
      '<div class="trace-toolbar">' +
        '<div class="filter-group" role="group" aria-label="筛选事件">' + filters.map(function (item) {
          return '<button class="chip" type="button" data-action="trace-filter" data-filter="' + item.id + '" aria-pressed="' + (filter === item.id ? "true" : "false") + '">' + item.label + "</button>";
        }).join("") + "</div>" +
        '<span class="trace-toolbar-note">' + filtered.length + " / " + events.length + " 条事件</span>" +
      "</div>" +
      '<div aria-live="polite">' + list + "</div>" +
    "</div>";
  };
})();
