/* Task workspace: goal, decision spine, approvals, composer. */
(function () {
  "use strict";

  var SFAI = (window.SFAI = window.SFAI || {});
  var U = SFAI.util;
  var views = (SFAI.views = SFAI.views || {});
  var icon = function (name, cls) { return SFAI.icons.icon(name, cls); };

  function statusBadge(status) {
    var meta = U.statusMeta(status);
    return '<span class="status-badge ' + meta.className + '">' +
      '<span class="dot ' + meta.dotClass + '" aria-hidden="true"></span>' + meta.label + "</span>";
  }

  function fact(label, value) {
    return '<div class="fact"><span class="fact-k">' + U.escapeHtml(label) + '</span><span class="fact-v">' + U.escapeHtml(value) + "</span></div>";
  }

  function refPills(refs) {
    if (!refs || !refs.length) return "";
    return '<div class="event-refs">' + refs.map(function (id) {
      var ref = SFAI.store.getRef(id);
      var label = ref ? ref.id + " · " + ref.title : id;
      return '<span class="ref-pill" title="' + U.escapeHtml(ref ? ref.path : id) + '">' + U.escapeHtml(label) + "</span>";
    }).join("") + "</div>";
  }

  function eventStatus(status) {
    if (!status || status === "ok") return "";
    var map = {
      running: { label: "运行中", dot: "dot--accent pulse" },
      paused: { label: "已暂停", dot: "dot--warn" },
      fail: { label: "失败", dot: "dot--danger" },
      denied: { label: "已拒绝", dot: "dot--danger" }
    };
    var meta = map[status] || map.running;
    return '<span class="event-status"><span class="dot ' + meta.dot + '" aria-hidden="true"></span>' + meta.label + "</span>";
  }

  function renderTool(event) {
    var status = U.toolStatusMeta(event.status);
    var open = event.status === "fail" ? " open" : "";
    var running = event.status === "running" ? " pulse" : "";
    return '<details class="tool" data-event-id="' + U.escapeHtml(event.id) + '"' + open + ">" +
      '<summary class="tool-summary">' +
        '<span class="tool-name">' + U.escapeHtml(event.tool || "tool") + "</span>" +
        '<span class="tool-desc">' + U.escapeHtml(event.summary || event.title || "") + "</span>" +
        '<span class="tool-status ' + status.className + '"><span class="dot ' + status.dotClass + running + '" aria-hidden="true"></span> ' + status.label +
          (event.duration ? " · " + U.escapeHtml(event.duration) : "") + "</span>" +
        '<span class="tool-chevron">' + icon("chevron-right") + "</span>" +
      "</summary>" +
      '<div class="tool-detail">' +
        '<div><span class="tool-kv-label">arguments</span><pre>' + U.escapeHtml(event.args || "{}") + "</pre></div>" +
        '<div><span class="tool-kv-label">result</span><pre>' + U.escapeHtml(event.result || "—") + "</pre></div>" +
      "</div>" +
    "</details>";
  }

  function renderApproval(event) {
    var approval = SFAI.store.getApproval(event.approvalId);
    if (!approval) {
      return '<div class="approval"><div class="approval-body"><p class="approval-desc">审批记录不可用。</p></div></div>';
    }
    var risk = U.riskMeta(approval.risk);
    var stateClass = approval.status === "denied" ? " approval--denied" : approval.status === "approved" ? " approval--approved" : "";
    var header = approval.status === "approved"
      ? '<span class="approval-head-left"><span class="dot dot--ok" aria-hidden="true"></span>已批准</span>'
      : approval.status === "denied"
        ? '<span class="approval-head-left"><span class="dot dot--danger" aria-hidden="true"></span>已拒绝</span>'
        : '<span class="approval-head-left"><span class="dot dot--warn" aria-hidden="true"></span>需要审批</span>';
    var outcome = "";
    if (approval.status === "approved") {
      outcome = '<div class="approval-outcome"><span class="dot dot--ok" aria-hidden="true"></span>已批准 · ' +
        U.escapeHtml(approval.decided || "") + (approval.mode === "always" ? " · 后续自动批准" : " · 仅本次") + "</div>";
    } else if (approval.status === "denied") {
      outcome = '<div class="approval-outcome"><span class="dot dot--danger" aria-hidden="true"></span>已拒绝 · ' +
        U.escapeHtml(approval.decided || "") + ' · 未执行任何外部操作</div>' +
        '<div class="approval-actions"><button class="btn" type="button" data-action="retry-approval" data-id="' + U.escapeHtml(approval.id) + '">' + icon("refresh") + "重新发起审批</button></div>";
    } else {
      outcome = '<div class="approval-actions">' +
        '<button class="btn btn--primary" type="button" data-action="approve" data-id="' + U.escapeHtml(approval.id) + '" data-mode="once">' + icon("check") + "批准一次</button>" +
        '<button class="btn" type="button" data-action="approve" data-id="' + U.escapeHtml(approval.id) + '" data-mode="always">始终允许</button>' +
        '<button class="btn btn--ghost" type="button" data-action="deny" data-id="' + U.escapeHtml(approval.id) + '">拒绝</button>' +
        "</div>" +
        '<p class="approval-note">批准后由 Decide-Act 运行时按策略执行；高影响操作需授权收据，全程写入事件账本。</p>';
    }
    return '<div class="approval' + stateClass + '" role="group" aria-labelledby="ap-title-' + U.escapeHtml(event.id) + '">' +
      '<div class="approval-head">' + header + '<span class="approval-id">' + U.escapeHtml(approval.id) + "</span></div>" +
      '<div class="approval-body">' +
        '<h3 class="approval-title" id="ap-title-' + U.escapeHtml(event.id) + '">' + U.escapeHtml(approval.title) + "</h3>" +
        '<p class="approval-desc">' + U.escapeHtml(approval.summary) + "</p>" +
        '<pre><code>' + U.escapeHtml(approval.command) + "</code></pre>" +
        '<dl class="approval-kv">' +
          "<div><dt>action</dt><dd>" + U.escapeHtml(approval.action) + "</dd></div>" +
          "<div><dt>risk</dt><dd>" + U.escapeHtml(risk.label) + "</dd></div>" +
          "<div><dt>scope</dt><dd>" + U.escapeHtml(approval.scope) + "</dd></div>" +
          "<div><dt>expires</dt><dd>" + U.escapeHtml(approval.expires) + "</dd></div>" +
        "</dl>" +
      "</div>" + outcome +
    "</div>";
  }

  function renderVerification(event) {
    var fail = event.verdict === "fail";
    var findings = event.findings || [];
    return '<div class="verification' + (fail ? " verification--fail" : "") + '">' +
      '<div class="verification-head"><span>' + U.escapeHtml(event.title || (fail ? "核验未通过" : "核验通过")) + "</span><span>" +
        (fail ? icon("alert") : icon("shield")) + "</span></div>" +
      '<p class="approval-desc">' + U.escapeHtml(event.text || "") + "</p>" +
      (findings.length ? '<ul class="verification-list">' + findings.map(function (finding) {
        var isFail = finding.state === "fail";
        var isPending = finding.state === "pending";
        var mark = isFail ? icon("alert") : isPending ? icon("circle") : icon("check");
        return '<li class="' + (isFail ? "is-fail" : "") + '">' + mark + "<span>" + U.escapeHtml(finding.label) + "</span></li>";
      }).join("") + "</ul>" : "") +
    "</div>";
  }

  function renderEvent(event) {
    var role = U.roleMeta(event.kind);
    var body = "";
    if (event.kind === "TOOL") {
      body = renderTool(event);
    } else if (event.kind === "APPROVAL") {
      body = renderApproval(event);
    } else if (event.kind === "VERIFICATION") {
      body = renderVerification(event);
    } else {
      body = '<p class="event-text">' + U.escapeHtml(event.text || "") + "</p>" + refPills(event.refs);
    }
    var intent = event.intent ? '<span class="event-intent">' + U.escapeHtml(event.intent) + "</span>" : "";
    return '<li class="event event--' + event.kind.toLowerCase() + (event.verdict === "fail" ? " is-fail" : "") + '">' +
      '<span class="event-marker" aria-hidden="true"></span>' +
      '<div class="event-body">' +
        '<div class="event-meta">' +
          '<span class="event-role">' + U.escapeHtml(role.label) + "</span>" + intent + eventStatus(event.status) +
          '<time class="event-time" datetime="' + U.escapeHtml(event.time || "") + '">' + U.escapeHtml(event.time || "") + "</time>" +
        "</div>" +
        (event.title ? '<div class="event-title">' + U.escapeHtml(event.title) + "</div>" : "") +
        body +
      "</div>" +
    "</li>";
  }

  function renderTimeline(task) {
    var events = SFAI.store.eventsFor(task.id);
    if (!events.length) {
      return '<p class="ins-empty">还没有事件。发送一条消息开始记录。</p>';
    }
    return '<ol class="timeline" aria-label="任务事件时间线">' + events.map(renderEvent).join("") + "</ol>";
  }

  function renderComposer(task, state) {
    var policy = state.settings.approvalPolicy;
    var policyLabel = U.approvalPolicyLabel(policy);
    return '<div class="composer-wrap">' +
      '<label class="composer-label" for="composer-input"><span>补充目标或约束</span><span>Enter 换行 · ⌘↵ 发送</span></label>' +
      '<form class="composer" id="composer-form" data-task-id="' + U.escapeHtml(task.id) + '">' +
        '<textarea id="composer-input" name="message" rows="1" placeholder="例如：先只读取 diagnostics 契约，不运行测试。"></textarea>' +
        '<div class="composer-bar">' +
          '<div class="composer-tools">' +
            '<span class="composer-mode"><span class="dot dot--accent" aria-hidden="true"></span>' + U.escapeHtml(task.mode) + "</span>" +
            '<span class="composer-hint">审批：' + policyLabel + "</span>" +
          "</div>" +
          '<button class="btn btn--primary btn--sm composer-send" type="submit">' + icon("send") + "发送</button>" +
        "</div>" +
      "</form>" +
    "</div>";
  }

  function renderNotice(task) {
    if (task.status === "approval") {
      return '<div class="task-notice">' + icon("alert") + '<div><strong>高影响执行等待审批。</strong> 批准前不会运行工作区命令；请在下方审批卡中确认。</div></div>';
    }
    if (task.status === "blocked") {
      return '<div class="task-notice task-notice--danger">' + icon("alert") + "<div><strong>" +
        U.escapeHtml(task.statusNote || "任务已阻塞") + "。</strong> 查看轨迹中的失败诊断，或补充信息后继续。</div></div>";
    }
    if (task.status === "paused") {
      return '<div class="task-notice">' + icon("pause") + '<div><strong>任务已暂停。</strong> 当前步骤停在安全边界；继续后需要重新发起未完成的执行。</div></div>';
    }
    return "";
  }

  views.renderTaskView = function (task, state) {
    if (!task) {
      return '<div class="page"><div class="empty"><div class="empty-icon">' + icon("tasks") + "</div><h2>还没有任务</h2><p>新建一个目标，开始真实测评。</p>" +
        '<div class="empty-actions"><button class="btn btn--primary" type="button" data-action="open-new-task">' + icon("plus") + "新建任务</button></div></div></div>";
    }
    var pauseLabel = task.status === "paused" ? "继续" : "暂停";
    var pauseIcon = task.status === "paused" ? "play" : "pause";
    var pauseDisabled = task.status === "done" || task.status === "blocked";
    var events = SFAI.store.eventsFor(task.id);
    return '<div class="page task-page">' +
      '<article class="task-head">' +
        '<div class="eyebrow-row"><span class="eyebrow">TASK ' + U.escapeHtml(task.id) + "</span><span class=\"eyebrow-sep\" aria-hidden=\"true\"></span>" +
          statusBadge(task.status) + '<span class="eyebrow-sep" aria-hidden="true"></span>' +
          '<span class="eyebrow">' + U.escapeHtml(task.model) + "</span></div>" +
        '<h1 class="task-title">' + U.escapeHtml(task.title) + "</h1>" +
        '<p class="task-goal">' + U.escapeHtml(task.goal) + "</p>" +
        '<div class="task-facts">' +
          fact("模式", task.mode) +
          fact("预算", U.money(task.budget.used) + " / " + U.money(task.budget.cap)) +
          fact("事件", String(events.length)) +
          fact("最近更新", task.updatedAt) +
        "</div>" +
        '<div class="task-actions">' +
          '<button class="btn" type="button" data-action="toggle-pause" data-id="' + U.escapeHtml(task.id) + '"' + (pauseDisabled ? " disabled" : "") + ">" + icon(pauseIcon) + pauseLabel + "</button>" +
          '<button class="btn" type="button" data-action="copy-id" data-id="' + U.escapeHtml(task.id) + '">' + icon("copy") + "复制 ID</button>" +
          '<button class="btn" type="button" data-action="export-task" data-id="' + U.escapeHtml(task.id) + '">' + icon("download") + "导出轨迹</button>" +
        "</div>" +
        renderNotice(task) +
      "</article>" +
      renderTimeline(task) +
      renderComposer(task, state) +
    "</div>";
  };

  views.renderTaskList = function (query) {
    var state = SFAI.store.getState();
    var normalized = String(query || "").trim().toLowerCase();
    var tasks = state.tasks.filter(function (task) {
      if (!normalized) return true;
      return (task.id + " " + task.title + " " + task.goal).toLowerCase().indexOf(normalized) >= 0;
    });
    if (!tasks.length) {
      return '<div class="task-empty">没有匹配的任务。<button class="btn btn--sm" type="button" data-action="clear-task-search">清除搜索</button></div>';
    }
    var open = tasks.filter(function (task) { return task.status !== "done"; });
    var done = tasks.filter(function (task) { return task.status === "done"; });
    function group(label, list) {
      if (!list.length) return "";
      return '<div class="task-group-label">' + label + "</div>" + list.map(function (task) {
        var meta = U.statusMeta(task.status);
        return '<button class="task-item" type="button" data-action="select-task" data-id="' + U.escapeHtml(task.id) + '" aria-current="' +
          (task.id === state.activeTaskId ? "true" : "false") + '">' +
          '<span class="dot ' + meta.dotClass + '" aria-hidden="true"></span>' +
          '<span><span class="task-title">' + U.escapeHtml(task.title) + '</span><span class="task-meta">' +
            U.escapeHtml(task.id) + " · " + meta.label + " · " + U.escapeHtml(task.updatedAt) + "</span></span>" +
        "</button>";
      }).join("");
    }
    return group("进行中", open) + group("已完成", done);
  };

  /* Shared conversation components, reused by the chat stream and settings. */
  views.statusBadge = statusBadge;
  views.renderToolEvent = renderTool;
  views.renderApprovalEvent = renderApproval;
  views.renderVerificationEvent = renderVerification;
})();
