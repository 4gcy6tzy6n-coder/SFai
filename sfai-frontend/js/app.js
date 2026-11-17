/* Conversation-first shell: sessions, chat stream, composer, and settings sheet. */
(function () {
  "use strict";

  var SFAI = window.SFAI;
  var U = SFAI.util;
  var store = SFAI.store;
  var views = SFAI.views;
  var els = {};
  var lastChatHtml = "";
  var pendingScroll = true;
  var railOpen = false;
  var settingsTab = "task";
  var lastLiveEvent = "";
  var resizeTimer = 0;
  function isMobile() {
    return window.matchMedia("(max-width: 960px)").matches;
  }

  function $(selector, root) {
    return (root || document).querySelector(selector);
  }

  function $$(selector, root) {
    return Array.prototype.slice.call((root || document).querySelectorAll(selector));
  }

  function renderRail(state) {
    var list = document.getElementById("session-list");
    if (list) list.innerHTML = views.renderTaskList(state.taskQuery);
    var count = document.getElementById("rail-count");
    if (count) count.textContent = state.tasks.length;
    var search = document.getElementById("task-search");
    if (search && search.value !== state.taskQuery) search.value = state.taskQuery;
  }

  function renderTopbar(state) {
    var task = store.activeTask();
    els.chatTitle.textContent = task ? task.title : "新的对话";
    els.chatStatus.innerHTML = task ? views.statusBadge(task.status) : "";
    els.chatSub.textContent = task
      ? task.id + " · " + task.mode + " · " + U.money(task.budget.used) + " / " + U.money(task.budget.cap)
      : "";
    var model = $(".model-name", els.modelChip);
    if (model) model.textContent = state.settings.model;
    document.title = "sfai · " + (task ? task.title : "对话");
  }

  function announceLatest(task) {
    if (!task) return;
    var events = store.eventsFor(task.id);
    var last = events[events.length - 1];
    if (!last || last.id === lastLiveEvent) return;
    lastLiveEvent = last.id;
    var text = "会话已更新";
    if (last.kind === "ASSISTANT") text = "sfai：" + U.truncate(last.text || last.title || "", 90);
    else if (last.kind === "TOOL") text = "sfai " + (last.status === "running" ? "正在运行 " : "调用了 ") + (last.tool || "工具");
    else if (last.kind === "APPROVAL") text = last.title || "有操作等待审批";
    else if (last.kind === "VERIFICATION") text = "核验" + (last.verdict === "fail" ? "未通过" : "通过");
    else if (last.kind === "USER") text = "你发送了一条消息";
    els.liveStatus.textContent = text;
  }

  function renderChat(state, task) {
    var html = views.renderConversation(task, state);
    var changed = html !== lastChatHtml;
    var saved = els.main.scrollTop;
    var nearBottom = (els.main.scrollHeight - saved - els.main.clientHeight) < 180;
    var follow = store.getState().settings.autoFollow !== false;
    var openTools = $$("details.tool[open]", els.main).map(function (node) { return node.dataset.eventId; }).filter(Boolean);
    if (changed) {
      els.main.innerHTML = html;
      lastChatHtml = html;
      SFAI.icons.hydrate(els.main);
      openTools.forEach(function (eventId) {
        var node = els.main.querySelector('details.tool[data-event-id="' + eventId + '"]');
        if (node) node.open = true;
      });
      if (pendingScroll || (follow && nearBottom)) els.main.scrollTop = els.main.scrollHeight;
      else els.main.scrollTop = saved;
      pendingScroll = false;
    } else if (pendingScroll) {
      els.main.scrollTop = els.main.scrollHeight;
      pendingScroll = false;
    }
    announceLatest(task);
  }

  function renderComposer(state, task) {
    if (!task) return;
    if (els.composerForm.dataset.taskId !== task.id) els.composerForm.dataset.taskId = task.id;
    els.composerNote.textContent = (window.SFAIApi && window.SFAIApi.enabled() ? "真实模式" : "未连接") + " · 审批：" + U.approvalPolicyLabel(state.settings.approvalPolicy);
    if (els.composerHint) {
      els.composerHint.textContent = state.settings.sendKey === "enter"
        ? "⇧↵ 换行 · ↵ 发送"
        : "Enter 换行 · ⌘↵ 发送";
    }
  }

  function renderRailState() {
    var mobile = isMobile();
    var open = mobile && railOpen;
    els.app.classList.toggle("rail-open", open);
    els.railScrim.hidden = !open;
    if (els.chatMain) els.chatMain.inert = open;
    var toggle = $('[data-action="toggle-rail"]');
    if (toggle) {
      toggle.setAttribute("aria-expanded", open ? "true" : "false");
      toggle.setAttribute("aria-label", open ? "关闭会话栏" : "打开会话栏");
    }
  }

  function captureSettingsFocus() {
    var el = document.activeElement;
    if (!el || !els.settingsBody.contains(el)) return null;
    if (el.id) {
      return { kind: "id", value: el.id, start: el.selectionStart, end: el.selectionEnd };
    }
    if (el.dataset && el.dataset.action) {
      return {
        kind: "action",
        action: el.dataset.action,
        id: el.dataset.id || "",
        filter: el.dataset.filter || "",
        risk: el.dataset.risk || "",
        tab: el.dataset.tab || ""
      };
    }
    return null;
  }

  function restoreSettingsFocus(key) {
    if (!key) return false;
    var el = null;
    if (key.kind === "id") {
      el = document.getElementById(key.value);
    } else if (key.kind === "action") {
      var selector = '[data-action="' + key.action + '"]';
      if (key.id) selector += '[data-id="' + key.id + '"]';
      if (key.filter) selector += '[data-filter="' + key.filter + '"]';
      if (key.risk) selector += '[data-risk="' + key.risk + '"]';
      if (key.tab) selector += '[data-tab="' + key.tab + '"]';
      el = els.settingsBody.querySelector(selector);
    }
    if (!el) return false;
    el.focus({ preventScroll: true });
    if (key.start != null && el.setSelectionRange) {
      try { el.setSelectionRange(key.start, key.end); } catch (error) { /* non-text control */ }
    }
    return true;
  }

  function renderSettings(options) {
    var settings = options || {};
    var state = store.getState();
    var task = store.activeTask();
    if (settingsTab === "run" && !settings.force) return;

    $$(".settings-tab").forEach(function (button) {
      var selected = button.dataset.tab === settingsTab;
      button.setAttribute("aria-selected", selected ? "true" : "false");
      button.tabIndex = selected ? 0 : -1;
    });
    els.settingsTaskLabel.textContent = task ? task.id + " · " + U.statusMeta(task.status).label : "";
    els.settingsBody.setAttribute("aria-labelledby", "settings-tab-" + settingsTab);

    var focusKey = settings.reset ? null : captureSettingsFocus();
    var scroll = settings.reset ? 0 : els.settingsBody.scrollTop;
    var subScrolls = {};
    if (!settings.reset) {
      ["context-results", "context-preview", "cap-list", "cap-detail"].forEach(function (id) {
        var pane = els.settingsBody.querySelector("#" + id);
        if (pane) subScrolls[id] = pane.scrollTop;
      });
    }

    els.settingsBody.innerHTML = views.renderSettingsContent(state, task, settingsTab);
    els.settingsBody.dataset.tab = settingsTab;
    SFAI.icons.hydrate(els.settingsBody);
    if (settingsTab === "capabilities") views.renderCapabilitiesPanels();

    Object.keys(subScrolls).forEach(function (id) {
      var pane = els.settingsBody.querySelector("#" + id);
      if (pane) pane.scrollTop = subScrolls[id];
    });
    els.settingsBody.scrollTop = scroll;
    restoreSettingsFocus(focusKey);
  }

  function render() {
    var state = store.getState();
    var task = store.activeTask();
    renderRail(state);
    renderTopbar(state);
    renderChat(state, task);
    renderComposer(state, task);
    renderRailState();
    if (els.settingsDialog.open) renderSettings({ force: false, reset: false });
  }

  function clearComposer() {
    els.composerInput.value = "";
    autoGrow(els.composerInput);
  }

  function autoGrow(el) {
    if (!el) return;
    el.style.height = "auto";
    el.style.height = Math.min(el.scrollHeight, 160) + "px";
  }

  function setGoalError(message) {
    var input = document.getElementById("new-chat-goal");
    var error = document.getElementById("new-chat-goal-error");
    if (!input || !error) return;
    error.textContent = message;
    error.hidden = !message;
    if (message) input.setAttribute("aria-invalid", "true");
    else input.removeAttribute("aria-invalid");
  }

  function setEndpointError(message) {
    var input = document.getElementById("settings-endpoint");
    var error = document.getElementById("settings-endpoint-error");
    if (!input || !error) return;
    error.textContent = message;
    error.hidden = !message;
    if (message) input.setAttribute("aria-invalid", "true");
    else input.removeAttribute("aria-invalid");
  }

  function splitList(value) {
    return String(value || "")
      .split(/[,，;；\n]/)
      .map(function (item) { return item.trim(); })
      .filter(Boolean);
  }

  function openNewChat() {
    if (railOpen) {
      railOpen = false;
      renderRailState();
    }
    els.newChatForm.reset();
    els.newChatForm.querySelector("details").open = false;
    var model = els.newChatForm.elements.model;
    var preferred = store.getState().settings.model;
    if (model && Array.prototype.some.call(model.options, function (option) { return option.value === preferred; })) {
      model.value = preferred;
    }
    setGoalError("");
    U.openDialog("new-chat-dialog");
    window.requestAnimationFrame(function () {
      var goal = document.getElementById("new-chat-goal");
      if (goal) goal.focus();
    });
  }

  function openSettings(tab) {
    if (railOpen) {
      railOpen = false;
      renderRailState();
    }
    settingsTab = tab || "task";
    U.openDialog("settings-dialog");
    renderSettings({ force: true, reset: true });
    window.requestAnimationFrame(function () {
      var active = document.getElementById("settings-tab-" + settingsTab);
      if (active) active.focus();
    });
  }

  function openPalette() {
    if (railOpen) {
      railOpen = false;
      renderRailState();
    }
    SFAI.palette.open();
  }

  function selectTask(id) {
    if (!store.getTask(id)) return;
    clearComposer();
    railOpen = false;
    pendingScroll = true;
    store.selectTask(id);
    renderRailState();
    els.main.focus({ preventScroll: true });
  }

  function exportTask(taskId) {
    var id = taskId || (store.activeTask() && store.activeTask().id);
    var payload = store.exportPayload(id);
    if (!payload) {
      U.toast("没有可导出的会话。");
      return;
    }
    U.downloadJson("sfai-" + id + "-trace.json", payload);
    U.toast("会话轨迹已导出");
  }

  function copy(text, message) {
    U.copyText(text).then(function (ok) {
      U.toast(ok ? message : "复制失败，请手动选择内容。");
    }).catch(function () {
      U.toast("复制失败，请手动选择内容。");
    });
  }

  function copyInPlace(button, text) {
    if (!button) return;
    U.copyText(text).then(function (ok) {
      if (!ok) {
        U.toast("复制失败，请手动选择内容。");
        return;
      }
      var original = button.innerHTML;
      var originalLabel = button.getAttribute("aria-label");
      button.textContent = "已复制";
      button.setAttribute("aria-label", "已复制");
      window.setTimeout(function () {
        button.innerHTML = original;
        if (originalLabel) button.setAttribute("aria-label", originalLabel);
        else button.removeAttribute("aria-label");
      }, 1200);
    }).catch(function () {
      U.toast("复制失败，请手动选择内容。");
    });
  }

  function focusApprovalCard() {
    if (els.settingsDialog.open) U.closeDialog("settings-dialog");
    var card = els.main.querySelector(".approval");
    if (!card) {
      U.toast("当前对话没有待审批操作。");
      return;
    }
    card.scrollIntoView({ block: "center" });
    var title = card.querySelector(".approval-title");
    if (title) {
      title.setAttribute("tabindex", "-1");
      title.focus({ preventScroll: true });
    }
  }

  function onAction(actionEl, event) {
    var action = actionEl.dataset.action;
    var id = actionEl.dataset.id;
    var task = store.activeTask();

    switch (action) {
      case "focus-composer":
        event.preventDefault();
        if (railOpen) {
          railOpen = false;
          renderRailState();
        }
        els.composerInput.focus();
        break;
      case "open-new-chat":
        openNewChat();
        break;
      case "toggle-rail":
        railOpen = !railOpen;
        renderRailState();
        if (isMobile()) {
          window.requestAnimationFrame(function () {
            var target = railOpen
              ? $("#session-rail .rail-close")
              : $('[data-action="toggle-rail"]');
            if (target) target.focus();
          });
        }
        break;
      case "close-rail":
        railOpen = false;
        renderRailState();
        var toggle = $('[data-action="toggle-rail"]');
        if (toggle) toggle.focus();
        break;
      case "select-task":
        selectTask(id);
        break;
      case "clear-task-search":
        store.setTaskQuery("");
        els.taskSearch.focus();
        break;
      case "open-settings":
        if (window.SFAIStore && window.SFAIStore.refreshFromBackend) {
          window.SFAIStore.refreshFromBackend();
        }
        openSettings(actionEl.dataset.tab || "task");
        break;
      case "open-palette":
        openPalette();
        break;
      case "settings-tab":
        settingsTab = actionEl.dataset.tab || "task";
        renderSettings({ force: true, reset: true });
        var tab = document.getElementById("settings-tab-" + settingsTab);
        if (tab) tab.focus();
        break;
      case "close-settings":
        U.closeDialog("settings-dialog");
        break;
      case "trace-filter":
        store.setTraceFilter(actionEl.dataset.filter);
        renderSettings({ force: true, reset: false });
        break;
      case "risk-filter":
        store.setRiskFilter(actionEl.dataset.risk);
        $$('[data-action="risk-filter"]', els.settingsBody).forEach(function (button) {
          button.setAttribute("aria-pressed", button.dataset.risk === actionEl.dataset.risk ? "true" : "false");
        });
        views.renderCapabilitiesPanels();
        var riskButton = $('[data-action="risk-filter"][data-risk="' + actionEl.dataset.risk + '"]', els.settingsBody);
        if (riskButton) riskButton.focus({ preventScroll: true });
        break;
      case "select-capability":
        store.setSelectedCapability(id);
        views.renderCapabilityList();
        views.renderCapabilityDetail();
        var capButton = $('[data-action="select-capability"][data-id="' + id + '"]', els.settingsBody);
        if (capButton) capButton.focus({ preventScroll: true });
        break;
      case "copy-capability":
        var cap = store.getCapability(id);
        if (cap) copyInPlace(actionEl, cap.schema);
        break;
      case "toggle-skill":
        var activated = store.toggleSkill(id);
        if (activated != null) U.toast(activated ? "Skill 已激活，只影响当前会话" : "Skill 已停用");
        break;
      case "select-ref":
        store.setSelectedRef(id);
        views.renderContextResults();
        var refButton = $('[data-action="select-ref"][data-id="' + id + '"]', els.settingsBody);
        if (refButton) refButton.focus({ preventScroll: true });
        break;
      case "toggle-working-ref":
        if (!task) break;
        if (task.workingSet.indexOf(id) >= 0) {
          store.removeRefFromWorkingSet(id);
          U.toast("已从工作集移除");
        } else {
          store.addRefToWorkingSet(id);
          U.toast("已加入工作集");
        }
        break;
      case "remove-working-ref":
        store.removeRefFromWorkingSet(id);
        U.toast("已从工作集移除");
        break;
      case "copy-ref":
        copyInPlace(actionEl, id);
        break;
      case "clear-context-search":
        store.setContextQuery("");
        views.renderContextResults();
        var contextSearch = document.getElementById("context-search-input");
        if (contextSearch) contextSearch.focus();
        break;
      case "export-task":
        exportTask(id);
        break;
      case "approve":
        pendingScroll = true;
        store.approve(id, actionEl.dataset.mode);
        els.main.focus({ preventScroll: true });
        break;
      case "deny":
        pendingScroll = true;
        store.deny(id);
        els.main.focus({ preventScroll: true });
        break;
      case "retry-approval":
        pendingScroll = true;
        store.retryApproval(id);
        var approveButton = $('[data-action="approve"][data-id="' + id + '"]', els.main);
        if (approveButton) approveButton.focus({ preventScroll: true });
        break;
      case "focus-approval":
        focusApprovalCard();
        break;
      case "suggest-prompt":
        els.composerInput.value = actionEl.dataset.prompt || "";
        autoGrow(els.composerInput);
        els.composerInput.focus();
        break;
      case "reset-demo":
        var resetConfirm = document.getElementById("reset-confirm");
        if (resetConfirm) resetConfirm.hidden = false;
        var confirmButton = $('[data-action="confirm-reset"]', els.settingsBody);
        if (confirmButton) confirmButton.focus();
        break;
      case "cancel-reset":
        var resetBlock = document.getElementById("reset-confirm");
        if (resetBlock) resetBlock.hidden = true;
        var resetTrigger = $('[data-action="reset-demo"]', els.settingsBody);
        if (resetTrigger) resetTrigger.focus();
        break;
      case "confirm-reset":
        U.closeDialog("settings-dialog");
        store.reset();
        settingsTab = "task";
        railOpen = false;
        clearComposer();
        pendingScroll = true;
        render();
        U.toast("本地状态已重置");
        break;
      case "copy-id":
        copyInPlace(actionEl, id);
        break;
      default:
        break;
    }
  }

  function handleClick(event) {
    var close = event.target.closest("[data-close-dialog]");
    if (close) {
      U.closeDialog(close.dataset.closeDialog);
      return;
    }
    var paletteItem = event.target.closest("[data-palette-index]");
    if (paletteItem) {
      SFAI.palette.execute(Number(paletteItem.dataset.paletteIndex));
      return;
    }
    var actionEl = event.target.closest("[data-action]");
    if (!actionEl || actionEl.disabled) return;
    onAction(actionEl, event);
  }

  function handleSubmit(event) {
    var form = event.target;

    if (form.id === "composer-form") {
      event.preventDefault();
      var text = els.composerInput.value.trim();
      if (!text) {
        U.toast("先写下要补充的内容。");
        els.composerInput.focus();
        return;
      }
      var taskId = form.dataset.taskId;
      if (!taskId) return;
      pendingScroll = true;
      var result = store.sendMessage(taskId, text);
      if (result) {
        els.composerInput.value = "";
        autoGrow(els.composerInput);
        els.composerInput.focus();
        U.toast("已写入会话");
      }
      return;
    }

    if (form.id === "new-chat-form") {
      event.preventDefault();
      var goal = form.elements.goal.value.trim();
      if (!goal) {
        setGoalError("写下要完成的结果，再开始对话。");
        form.elements.goal.focus();
        return;
      }
      var budget = Number(form.elements.budget.value);
      if (!Number.isFinite(budget) || budget < 0.1) {
        U.toast("预算需大于等于 0.1 元。");
        form.elements.budget.focus();
        return;
      }
      var payload = {
        goal: goal,
        constraints: splitList(form.elements.constraints.value),
        acceptance: splitList(form.elements.acceptance.value),
        model: form.elements.model.value,
        budget: budget
      };
      U.closeDialog("new-chat-dialog");
      setGoalError("");
      clearComposer();
      pendingScroll = true;
      var task = store.createTask(payload);
      els.composerInput.focus();
      U.toast("新对话 " + task.id + " 已开始");
      return;
    }

    if (form.id === "settings-form") {
      event.preventDefault();
      var endpoint = form.elements.endpoint.value.trim();
      if (endpoint && !/^https?:\/\/\S+$/i.test(endpoint)) {
        setEndpointError("请输入以 http:// 或 https:// 开头的完整地址。");
        form.elements.endpoint.focus();
        return;
      }
      var theme = form.querySelector('input[name="theme"]:checked');
      var settingsPayload = {
        endpoint: endpoint,
        provider: form.elements.provider.value,
        model: form.elements.model.value,
        approvalPolicy: form.elements.approvalPolicy.value,
        theme: theme ? theme.value : "system",
        motion: form.elements.motion.value,
        density: form.elements.density.value,
        timestamps: form.elements.timestamps.checked,
        sendKey: form.elements.sendKey.value,
        autoFollow: form.elements.autoFollow.checked
      };
      U.closeDialog("settings-dialog");
      store.setSettings(settingsPayload);
      U.toast("设置已保存");
    }
  }

  function handleInput(event) {
    var target = event.target;
    if (target.id === "task-search") {
      store.setTaskQuery(target.value);
      return;
    }
    if (target.id === "composer-input") {
      autoGrow(target);
      return;
    }
    if (target.id === "palette-input") {
      SFAI.palette.handleInput(target.value);
      return;
    }
    if (target.id === "context-search-input") {
      store.setContextQuery(target.value);
      views.renderContextResults();
      return;
    }
    if (target.id === "new-chat-goal") {
      setGoalError("");
      return;
    }
    if (target.id === "settings-endpoint") {
      setEndpointError("");
    }
  }

  function handleKeydown(event) {
    var target = event.target;

    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      if (!document.querySelector("dialog[open]")) openPalette();
      return;
    }

    var sendKey = store.getState().settings.sendKey;
    var sendWithEnter = sendKey === "enter"
      ? event.key === "Enter" && !event.shiftKey
      : event.key === "Enter" && (event.metaKey || event.ctrlKey);
    if (target.id === "composer-input" && sendWithEnter && !event.isComposing) {
      event.preventDefault();
      var form = target.closest("form");
      if (form && form.requestSubmit) form.requestSubmit();
      else if (form) form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
      return;
    }

    if (target.id === "palette-input" && SFAI.palette.handleKeydown(event)) return;

    if (event.key === "Escape") {
      if (!document.querySelector("dialog[open]") && railOpen) {
        railOpen = false;
        renderRailState();
        var toggle = $('[data-action="toggle-rail"]');
        if (toggle) toggle.focus();
      }
      return;
    }

    if (target.getAttribute && target.getAttribute("role") === "tab" && target.closest(".settings-nav") &&
      ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].indexOf(event.key) >= 0) {
      event.preventDefault();
      var tabs = $$(".settings-tab");
      var index = tabs.indexOf(target);
      var forward = event.key === "ArrowDown" || event.key === "ArrowRight";
      if (index >= 0) {
        var next = forward ? (index + 1) % tabs.length : (index - 1 + tabs.length) % tabs.length;
        tabs[next].focus();
        tabs[next].click();
      }
      return;
    }

    if (!event.metaKey && !event.ctrlKey && !event.altKey && event.key.toLowerCase() === "n") {
      var tag = (target.tagName || "").toLowerCase();
      if ((target === document.body || target === els.main) && !document.querySelector("dialog[open]") &&
        ["input", "textarea", "select"].indexOf(tag) < 0) {
        event.preventDefault();
        openNewChat();
      }
    }
  }

  function handleResize() {
    window.clearTimeout(resizeTimer);
    resizeTimer = window.setTimeout(function () {
      if (!isMobile() && railOpen) railOpen = false;
      renderRailState();
    }, 120);
  }

  function init() {
    els.app = document.getElementById("app");
    els.railScrim = document.getElementById("rail-scrim");
    els.chatMain = $(".chat-main");
    els.chatTitle = document.getElementById("chat-title");
    els.chatStatus = document.getElementById("chat-status");
    els.chatSub = document.getElementById("chat-sub");
    els.modelChip = document.getElementById("model-chip");
    els.main = document.getElementById("main");
    els.composerForm = document.getElementById("composer-form");
    els.composerInput = document.getElementById("composer-input");
    els.composerNote = document.getElementById("composer-note");
    els.composerHint = document.getElementById("composer-hint");
    els.taskSearch = document.getElementById("task-search");
    els.settingsDialog = document.getElementById("settings-dialog");
    els.settingsBody = document.getElementById("settings-body");
    els.settingsTaskLabel = document.getElementById("settings-task-label");
    els.newChatForm = document.getElementById("new-chat-form");
    els.liveStatus = document.getElementById("live-status");

    SFAI.icons.hydrate(document);
    SFAI.palette.init({
      openNewChat: openNewChat,
      openSettings: openSettings,
      selectTask: selectTask,
      exportTask: exportTask,
      copy: copy
    });

    store.subscribe(render);
    pendingScroll = true;
    render();

    document.addEventListener("click", handleClick);
    document.addEventListener("submit", handleSubmit);
    document.addEventListener("input", handleInput);
    document.addEventListener("keydown", handleKeydown);
    window.addEventListener("resize", handleResize);

    els.railScrim.addEventListener("click", function () {
      railOpen = false;
      renderRailState();
      var toggle = $('[data-action="toggle-rail"]');
      if (toggle) toggle.focus();
    });

    var newChatDialog = document.getElementById("new-chat-dialog");
    if (newChatDialog) {
      newChatDialog.addEventListener("close", function () {
        setGoalError("");
        if (els.composerInput) els.composerInput.focus();
      });
    }

    els.settingsDialog.addEventListener("close", function () {
      var active = document.activeElement;
      if (active && active !== document.body && !els.settingsDialog.contains(active)) return;
      var trigger = document.getElementById("settings-trigger");
      if (trigger && trigger.offsetParent !== null) {
        trigger.focus();
      } else {
        var railToggle = $('[data-action="toggle-rail"]');
        if (railToggle) railToggle.focus();
      }
    });

    var paletteDialog = document.getElementById("palette-dialog");
    if (paletteDialog) {
      paletteDialog.addEventListener("close", function () {
        var input = document.getElementById("palette-input");
        if (input) input.value = "";
      });
    }

    if (window.matchMedia) {
      var darkQuery = window.matchMedia("(prefers-color-scheme: dark)");
      var onThemeChange = function () {
        if (store.getState().settings.theme === "system") store.applyPreferences();
      };
      if (darkQuery.addEventListener) darkQuery.addEventListener("change", onThemeChange);
      else if (darkQuery.addListener) darkQuery.addListener(onThemeChange);
    }

    if (!store.isStorageAvailable()) {
      U.toast("本地存储不可用，修改只会保留到刷新前。");
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", function () {
    init();
    // 真实测评模式：启动即从 Decide-Act 运行时拉取能力/Skill 注册表。
    if (window.SFAIStore && window.SFAIStore.refreshFromBackend) {
      window.SFAIStore.refreshFromBackend();
    }
  });
  } else {
    init();
  }
})();
