/* Conversation-first workspace: user turns and grouped sfai replies. */
(function () {
  "use strict";

  var SFAI = (window.SFAI = window.SFAI || {});
  var U = SFAI.util;
  var views = (SFAI.views = SFAI.views || {});
  var icon = function (name, cls) { return SFAI.icons.icon(name, cls); };

  var CHAT_KINDS = ["USER", "ASSISTANT", "TOOL", "APPROVAL", "VERIFICATION"];

  function eventStatus(status) {
    if (status === "running") return '<span class="chat-live"><span class="dot dot--accent pulse" aria-hidden="true"></span>处理中</span>';
    if (status === "paused") return '<span class="chat-live"><span class="dot dot--warn" aria-hidden="true"></span>已暂停</span>';
    return "";
  }

  function refPills(refs) {
    if (!refs || !refs.length) return "";
    return '<div class="event-refs">' + refs.map(function (id) {
      var ref = SFAI.store.getRef(id);
      var label = ref ? ref.id + " · " + ref.title : id;
      return '<span class="ref-pill" title="' + U.escapeHtml(ref ? ref.path : id) + '">' + U.escapeHtml(label) + "</span>";
    }).join("") + "</div>";
  }

  function renderAssistantEvent(event) {
    if (event.kind === "TOOL") {
      return '<div class="chat-step chat-step--tool">' + views.renderToolEvent(event) + "</div>";
    }
    if (event.kind === "APPROVAL") {
      return '<div class="chat-step chat-step--approval">' + views.renderApprovalEvent(event) + "</div>";
    }
    if (event.kind === "VERIFICATION") {
      return '<div class="chat-step chat-step--verification">' + views.renderVerificationEvent(event) + "</div>";
    }
    var intent = event.intent ? '<span class="event-intent">' + U.escapeHtml(event.intent) + "</span>" : "";
    return '<div class="chat-step">' +
      '<div class="chat-step-meta">' + intent + eventStatus(event.status) +
        '<time class="chat-step-time chat-time" datetime="' + U.escapeHtml(event.time || "") + '">' + U.escapeHtml(event.time || "") + "</time></div>" +
      (event.title ? '<div class="chat-step-title">' + U.escapeHtml(event.title) + "</div>" : "") +
      (event.text ? '<p class="chat-step-text">' + U.escapeHtml(event.text) + "</p>" : "") +
      refPills(event.refs) +
    "</div>";
  }

  function renderAssistantGroup(events, task) {
    var first = events[0] || {};
    var body = events.map(renderAssistantEvent).join("");
    return '<article class="chat-assistant" aria-label="sfai 回复">' +
      '<header class="chat-assistant-head">' +
        '<span class="chat-agent-mark" aria-hidden="true">' + SFAI.icons.brandMark() + "</span>" +
        '<span class="chat-agent-name">sfai</span>' +
        '<span class="chat-agent-sep" aria-hidden="true"></span>' +
        '<span class="chat-agent-meta mono"><span class="chat-agent-time"><time class="chat-time" datetime="' +
          U.escapeHtml(first.time || "") + '">' + U.escapeHtml(first.time || "") + "</time> · </span>" +
          U.escapeHtml(task.model) + "</span>" +
      "</header>" +
      '<div class="chat-assistant-thread">' + body + "</div>" +
    "</article>";
  }

  function renderUser(event) {
    return '<article class="chat-user">' +
      '<div class="chat-user-meta mono"><span>你</span><span class="chat-user-time"> · <time class="chat-time" datetime="' +
        U.escapeHtml(event.time || "") + '">' + U.escapeHtml(event.time || "") + "</time></span></div>" +
      '<div class="chat-user-text">' + U.escapeHtml(event.text || event.title || "") + "</div>" +
    "</article>";
  }

  function renderEmpty() {
    return '<div class="chat-empty">' +
      '<p class="eyebrow">NEW CONVERSATION</p>' +
      "<h2>你想让 sfai 做什么？</h2>" +
      '<p class="chat-empty-sub">目标、约束和验收规则会固定在这次会话里；工具执行、审批与核验会内联出现。</p>' +
      '<div class="chat-suggestions">' +
        '<button class="chat-suggestion" type="button" data-action="suggest-prompt" data-prompt="检查 replay 回放一致性，并给出差异摘要。">检查 replay 一致性</button>' +
        '<button class="chat-suggestion" type="button" data-action="suggest-prompt" data-prompt="分析最近一次索引重建后的检索召回下降。">分析检索召回下降</button>' +
        '<button class="chat-suggestion" type="button" data-action="suggest-prompt" data-prompt="把这次迁移决策整理成一份可检索的记录。">整理决策记录</button>' +
      "</div>" +
    "</div>";
  }

  views.renderConversation = function (task, state) {
    if (!task) return renderEmpty();
    var events = SFAI.store.eventsFor(task.id).filter(function (event) {
      return CHAT_KINDS.indexOf(event.kind) >= 0;
    });
    var groups = [];
    events.forEach(function (event) {
      if (event.kind === "USER") {
        groups.push({ type: "user", event: event });
      } else {
        var last = groups[groups.length - 1];
        if (!last || last.type !== "assistant") {
          last = { type: "assistant", events: [] };
          groups.push(last);
        }
        last.events.push(event);
      }
    });
    var meta = '<div class="chat-session-meta"><span class="mono">SESSION ' + U.escapeHtml(task.id) + "</span>" +
      '<span class="chat-meta-sep" aria-hidden="true"></span>' +
      "<span>" + U.escapeHtml(task.mode) + "</span>" +
      '<span class="chat-session-time-wrap"><span class="chat-meta-sep" aria-hidden="true"></span>' +
        '<span class="chat-session-time">开始于 ' + U.escapeHtml(task.createdAt) + "</span></span>" +
      '<span class="chat-meta-sep" aria-hidden="true"></span>' +
      "<span>" + U.escapeHtml(task.model) + "</span></div>";
    if (!groups.length) {
      return '<div class="chat-column">' + meta + renderEmpty() + "</div>";
    }
    var stream = groups.map(function (group) {
      return group.type === "user" ? renderUser(group.event) : renderAssistantGroup(group.events, task);
    }).join("");
    return '<div class="chat-column">' + meta + '<div class="chat-stream">' + stream + "</div>" +
      '<div class="chat-tail" aria-hidden="true"><span class="dot dot--accent"></span>会话已同步到本地事件账本</div>' +
    "</div>";
  };

  views.renderChatEmpty = renderEmpty;
})();
