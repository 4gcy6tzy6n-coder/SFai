/* Local state + persistence. Real runtime data only (Decide-Act backend). */
(function () {
  "use strict";

  var SFAI = (window.SFAI = window.SFAI || {});
  var U = SFAI.util;
  var KEY = "sfai.console.state.v3";
  var VALID_ROUTES = ["tasks", "trajectory", "context", "capabilities"];
  var listeners = [];
  var verifyEvents = {};
  var memoryStore = {};
  var storageAvailable = true;

  var state;

  function clone(value) {
    return JSON.parse(JSON.stringify(value));
  }

  function readRaw() {
    try {
      return window.localStorage.getItem(KEY);
    } catch (error) {
      storageAvailable = false;
      return memoryStore[KEY] || null;
    }
  }

  function writeRaw(value) {
    try {
      window.localStorage.setItem(KEY, value);
    } catch (error) {
      storageAvailable = false;
      memoryStore[KEY] = value;
    }
  }

  function removeRaw() {
    try {
      window.localStorage.removeItem(KEY);
    } catch (error) {
      storageAvailable = false;
    }
    delete memoryStore[KEY];
  }

  function initialTransient(activeTaskId) {
    return {
      activeTaskId: activeTaskId,
      route: "tasks",
      taskQuery: "",
      contextQuery: "",
      traceFilter: "ALL",
      riskFilter: "all",
      selectedRefId: "ctx_8f21",
      selectedCapabilityId: "context.read",
      inspectorTab: "state",
      inspectorOpen: true,
      mobileSidebarOpen: false,
      mobileInspectorOpen: false
    };
  }

  function load() {
    var seed = SFAI.seed();
    var saved = null;
    var raw = readRaw();
    if (raw) {
      try {
        saved = JSON.parse(raw);
      } catch (error) {
        saved = null;
      }
    }

    var tasks = saved && Array.isArray(saved.tasks) && saved.tasks.length ? saved.tasks : seed.tasks;
    var activeId = saved && tasks.some(function (task) { return task.id === saved.activeTaskId; })
      ? saved.activeTaskId
      : (tasks.length ? tasks[0].id : null);
    var route = saved && VALID_ROUTES.indexOf(saved.route) >= 0 ? saved.route : "tasks";

    state = {
      meta: seed.meta,
      tasks: tasks,
      events: saved && Array.isArray(saved.events) ? saved.events : seed.events,
      approvals: saved && Array.isArray(saved.approvals) ? saved.approvals : seed.approvals,
      contextRefs: saved && Array.isArray(saved.contextRefs) ? saved.contextRefs : seed.contextRefs,
      capabilities: seed.capabilities,
      skills: saved && Array.isArray(saved.skills) ? saved.skills : seed.skills,
      settings: Object.assign({}, seed.settings, (saved && saved.settings) || {})
    };

    Object.assign(state, initialTransient(activeId));
    state.route = route;
    applyPreferences();
  }

  function applyPreferences() {
    var root = document.documentElement;
    var theme = U.resolveTheme(state.settings.theme);
    root.dataset.theme = theme;
    root.dataset.motion = state.settings.motion === "reduced" ? "reduced" : "system";
    root.dataset.density = state.settings.density === "compact" ? "compact" : "comfortable";
    root.dataset.timestamps = state.settings.timestamps === false ? "hidden" : "visible";
    var meta = document.querySelector('meta[name="theme-color"]');
    if (meta) meta.setAttribute("content", theme === "dark" ? "#0b0e11" : "#f5f6f7");
  }

  function persist() {
    var snapshot = {
      savedAt: new Date().toISOString(),
      tasks: state.tasks,
      events: state.events,
      approvals: state.approvals,
      contextRefs: state.contextRefs,
      skills: state.skills,
      settings: state.settings,
      activeTaskId: state.activeTaskId,
      route: state.route
    };
    writeRaw(JSON.stringify(snapshot));
  }

  function subscribe(fn) {
    listeners.push(fn);
    return function () {
      listeners = listeners.filter(function (listener) { return listener !== fn; });
    };
  }

  function notify() {
    listeners.slice().forEach(function (fn) {
      try {
        fn(state);
      } catch (error) {
        if (window.console && window.console.error) window.console.error("sfai render failed", error);
      }
    });
  }

  async function refreshFromBackend() {
    if (!(global.SFAIApi && global.SFAIApi.enabled())) return;
    try {
      var caps = await global.SFAIApi.json("GET", "/api/capabilities");
      var skills = await global.SFAIApi.json("GET", "/api/skills");
      var providers = await global.SFAIApi.json("GET", "/api/providers");
      state = Object.assign({}, state, {
        providers: providers.providers || [],
        capabilities: (caps.capabilities || []).map(function (c) {
          var ops = (c.operations || []).map(function (o) {
            return { id: o.id, description: o.description, risk: o.risk_class, sample: JSON.stringify(o.input_schema || {}) };
          });
          return {
            id: c.id, name: c.id, description: c.description, risk: c.risk_class,
            operations: ops, permissions: Object.keys(c.permissions || {})
          };
        }),
        skills: (skills.skills || []).map(function (sk) {
          return { id: sk.name, name: sk.name, description: sk.description, version: sk.version, active: false, files: [] };
        })
      });
      commit();
    } catch (e) { /* backend may still be booting */ }
  }

  function commit() {
    persist();
    notify();
  }

  function getState() {
    return state;
  }

  function getTask(id) {
    return state.tasks.find(function (task) { return task.id === id; }) || null;
  }

  function activeTask() {
    return getTask(state.activeTaskId) || state.tasks[0] || null;
  }

  function getApproval(id) {
    return state.approvals.find(function (approval) { return approval.id === id; }) || null;
  }

  function getRef(id) {
    return state.contextRefs.find(function (ref) { return ref.id === id; }) || null;
  }

  function getSkill(id) {
    return state.skills.find(function (skill) { return skill.id === id; }) || null;
  }

  function getCapability(id) {
    return state.capabilities.find(function (cap) { return cap.id === id; }) || null;
  }

  function eventsFor(taskId) {
    return state.events
      .filter(function (event) { return event.taskId === taskId; })
      .slice()
      .sort(function (a, b) { return a.seq - b.seq; });
  }

  function workingRefs(task) {
    if (!task) return [];
    return task.workingSet.map(getRef).filter(Boolean);
  }

  function tokenTotal(task) {
    return workingRefs(task).reduce(function (sum, ref) { return sum + (Number(ref.tokens) || 0); }, 0);
  }

  function pendingApproval(task) {
    if (!task) return null;
    return state.approvals.find(function (approval) {
      return approval.taskId === task.id && approval.status === "pending";
    }) || null;
  }

  function updateTask(id, patch) {
    state = Object.assign({}, state, {
      tasks: state.tasks.map(function (task) {
        if (task.id !== id) return task;
        var next = typeof patch === "function" ? patch(task) : patch;
        return Object.assign({}, task, next);
      })
    });
  }

  function updateEvent(id, patch) {
    state = Object.assign({}, state, {
      events: state.events.map(function (event) {
        if (event.id !== id) return event;
        var next = typeof patch === "function" ? patch(event) : patch;
        return Object.assign({}, event, next);
      })
    });
  }

  function updateApproval(id, patch) {
    state = Object.assign({}, state, {
      approvals: state.approvals.map(function (approval) {
        if (approval.id !== id) return approval;
        var next = typeof patch === "function" ? patch(approval) : patch;
        return Object.assign({}, approval, next);
      })
    });
  }

  function nextSeq(taskId) {
    return state.events
      .filter(function (event) { return event.taskId === taskId; })
      .reduce(function (max, event) { return Math.max(max, event.seq); }, 0) + 1;
  }

  function maxTurn(taskId) {
    return state.events
      .filter(function (event) { return event.taskId === taskId; })
      .reduce(function (max, event) { return Math.max(max, event.turn || 0); }, 0);
  }

  function addEvent(taskId, raw) {
    var task = getTask(taskId);
    if (!task) return null;
    var seq = nextSeq(taskId);
    var event = Object.assign({
      id: U.uid("e"),
      taskId: taskId,
      seq: seq,
      turn: raw.turn || maxTurn(taskId) || 1,
      time: U.timeNow(),
      status: "ok"
    }, raw);
    state = Object.assign({}, state, { events: state.events.concat([event]) });
    updateTask(taskId, function (current) {
      return {
        updatedAt: "刚刚",
        stats: Object.assign({}, current.stats, {
          steps: Math.max(current.stats.steps || 0, seq),
          turns: Math.max(current.stats.turns || 0, event.turn)
        })
      };
    });
    if (raw.cost) {
      updateTask(taskId, function (current) {
        return {
          budget: Object.assign({}, current.budget, {
            used: Math.round((current.budget.used + raw.cost) * 100) / 100,
            calls: current.budget.calls + 1
          })
        };
      });
    }
    return event;
  }

  function setStatus(taskId, status, note) {
    updateTask(taskId, { status: status, statusNote: note });
  }

  function pauseRunningEvents(taskId) {
    state.events.forEach(function (event) {
      if (event.taskId === taskId && event.status === "running") {
        updateEvent(event.id, { status: "paused" });
      }
    });
  }

  function selectTask(id) {
    if (!getTask(id)) return;
    state = Object.assign({}, state, {
      activeTaskId: id,
      mobileSidebarOpen: false,
      mobileInspectorOpen: false
    });
    commit();
  }

  function navigate(route) {
    if (VALID_ROUTES.indexOf(route) < 0) return;
    state = Object.assign({}, state, {
      route: route,
      mobileSidebarOpen: false,
      mobileInspectorOpen: false
    });
    persist();
    notify();
  }

  function setTaskQuery(value) {
    state = Object.assign({}, state, { taskQuery: String(value || "") });
    notify();
  }

  function setContextQuery(value) {
    state = Object.assign({}, state, { contextQuery: String(value || "") });
  }

  function setTraceFilter(value) {
    state = Object.assign({}, state, { traceFilter: value || "ALL" });
  }

  function setRiskFilter(value) {
    state = Object.assign({}, state, { riskFilter: value || "all" });
  }

  function setSelectedRef(id) {
    if (!getRef(id)) return;
    state = Object.assign({}, state, { selectedRefId: id });
  }

  function setSelectedCapability(id) {
    if (!getCapability(id)) return;
    state = Object.assign({}, state, { selectedCapabilityId: id });
  }

  function setInspectorTab(tab) {
    if (["state", "context", "cost"].indexOf(tab) < 0) return;
    state = Object.assign({}, state, { inspectorTab: tab, inspectorOpen: true });
    commit();
  }

  function toggleInspector() {
    state = Object.assign({}, state, { inspectorOpen: !state.inspectorOpen });
    persist();
    notify();
  }

  function setMobileSidebar(open) {
    state = Object.assign({}, state, { mobileSidebarOpen: !!open });
    notify();
  }

  function setMobileInspector(open) {
    state = Object.assign({}, state, { mobileInspectorOpen: !!open, inspectorOpen: true });
    notify();
  }

  function toggleTheme() {
    var current = U.resolveTheme(state.settings.theme);
    state = Object.assign({}, state, {
      settings: Object.assign({}, state.settings, { theme: current === "dark" ? "light" : "dark" })
    });
    applyPreferences();
    persist();
    notify();
  }

  function saveSettings(payload) {
    state = Object.assign({}, state, {
      settings: Object.assign({}, state.settings, payload)
    });
    applyPreferences();
    persist();
    notify();
  }

  // ---- real mode (Decide-Act runtime via `sfai serve`) ----
  var realTaskMap = {};

  function realStatus(status) {
    if (status === "completed") return "done";
    if (status === "waiting_approval") return "approval";
    if (status === "blocked" || status === "failed" || status === "cancelled") return "blocked";
    return "active";
  }

  function realSend(taskId, text, turn) {
    var api = global.SFAIApi;
    var task = getTask(taskId);
    if (!task) return null;
    addEvent(taskId, { kind: "USER", turn: turn, title: "补充目标", text: text });
    addEvent(taskId, {
      kind: "SYSTEM", turn: turn, title: "真实模型运行",
      text: "已提交至 Decide-Act 运行时（" + (api.base() || "local") + "）。"
    });
    (async function () {
      try {
        if (!realTaskMap[taskId]) {
          var created = await api.createTask(
            task.goal, task.constraints, task.acceptance.map(function (a) { return a.label; })
          );
          realTaskMap[taskId] = created.task.task_id;
        }
        var realId = realTaskMap[taskId];
        var seen = 0;
        var poll = window.setInterval(async function () {
          try {
            var ev = await api.events(realId);
            for (; seen < ev.events.length; seen += 1) {
              var mapped = api.mapLedgerEvent(ev.events[seen], turn);
              if (mapped) addEvent(taskId, mapped);
            }
          } catch (e) { /* transient */ }
        }, 500);
        var result;
        try {
          var runOpts = {};
          if (state.settings.provider) runOpts.provider = state.settings.provider;
          if (state.settings.model) runOpts.model = state.settings.model;
          result = await api.runTask(realId, runOpts);
        } finally {
          window.clearInterval(poll);
        }
        // drain any remaining events
        try {
          var ev2 = await api.events(realId);
          for (; seen < ev2.events.length; seen += 1) {
            var mapped2 = api.mapLedgerEvent(ev2.events[seen], turn);
            if (mapped2) addEvent(taskId, mapped2);
          }
        } catch (e) {}
        addEvent(taskId, {
          kind: "ASSISTANT", turn: turn, intent: result.status === "completed" ? "finish" : "respond",
          title: "运行结束 · " + result.status,
          text: result.answer || "（本轮没有文本回答；详见工具回执与轨迹。）"
        });
        updateTask(taskId, {
          status: realStatus(result.status),
          statusNote: "steps " + (result.steps || 0) + " · cost $" + ((result.cost && result.cost.cost_usd) || 0).toFixed(4)
        });
      } catch (err) {
        addEvent(taskId, { kind: "SYSTEM", turn: turn, title: "运行错误", text: String(err && err.message || err) });
        updateTask(taskId, { status: "blocked", statusNote: "运行错误" });
      }
    })();
    return task;
  }

  function createTask(payload) {
    var maxId = state.tasks.reduce(function (max, task) {
      var match = /^T-(\d+)$/.exec(task.id);
      return match ? Math.max(max, Number(match[1])) : max;
    }, 1000);
    var id = "T-" + (maxId + 1);
    var now = U.timeNow();
    var constraints = payload.constraints || [];
    var acceptance = payload.acceptance && payload.acceptance.length
      ? payload.acceptance
      : ["目标结果可验证"];
    var task = {
      id: id,
      title: U.truncate(payload.goal, 42),
      goal: payload.goal,
      status: "active",
      statusNote: "等待下一步决策",
      model: payload.model || state.settings.model,
      mode: "standard",
      updatedAt: "刚刚",
      createdAt: now.slice(0, 5),
      constraints: constraints,
      acceptance: acceptance.map(function (label, index) {
        return { id: "a" + (index + 1), label: label, state: "pending" };
      }),
      budget: { used: 0, cap: Number(payload.budget) || 5, tokens: 0, calls: 0, wall: "0m 00s" },
      stats: { turns: 1, steps: 2, duration: "0m 00s", costSeries: [0, 0] },
      workingSet: [],
      activeSkills: [],
      pendingApprovalId: null,
      verification: null
    };
    state = Object.assign({}, state, {
      tasks: state.tasks.concat([task]),
      activeTaskId: id,
      route: "tasks",
      taskQuery: "",
      selectedRefId: state.selectedRefId,
      mobileSidebarOpen: false
    });
    addEvent(id, {
      kind: "SYSTEM",
      turn: 1,
      title: "任务创建",
      text: "预算上限 " + U.money(task.budget.cap) + " · 验收规则 " + task.acceptance.length + " 条。"
    });
    addEvent(id, {
      kind: "USER",
      turn: 1,
      title: "任务目标",
      text: payload.goal
    });
    addEvent(id, {
      kind: "ASSISTANT",
      turn: 1,
      intent: "respond",
      title: "任务已建立",
      text: (global.SFAIApi && global.SFAIApi.enabled())
        ? "目标与约束已固定在 resident context。真实模型已接入（Decide-Act 运行时）；发送下一步即可开始执行。"
        : "目标与约束已固定在 resident context。未连接真实运行时——请在设置中配置 sfai serve 地址后开始真实测评。"
    });
    commit();
    return task;
  }

  function sendMessage(taskId, text) {
    var task = getTask(taskId);
    var body = String(text || "").trim();
    if (!task || !body) return null;
    if (!(global.SFAIApi && global.SFAIApi.enabled())) {
      addEvent(taskId, {
        kind: "SYSTEM",
        turn: maxTurn(taskId) + 1,
        title: "未连接真实运行时",
        text: "真实测评模式：请在设置中配置 sfai serve 地址（如 http://127.0.0.1:8787）后重试。"
      });
      commit();
      return task;
    }
    var turn = maxTurn(taskId) + 1;
    pauseRunningEvents(taskId);
    return realSend(taskId, body, turn);
  }

  function approve(approvalId, mode) {
    var approval = getApproval(approvalId);
    if (!approval || approval.status !== "pending") return;
    var taskId = approval.taskId;
    var decidedAt = U.timeNow();
    updateApproval(approvalId, {
      status: "approved",
      decided: decidedAt,
      mode: mode === "always" ? "always" : "once"
    });
    addEvent(taskId, {
      kind: "SYSTEM",
      turn: maxTurn(taskId) + 1,
      title: "审批决定 · " + (mode === "always" ? "始终允许" : "允许一次"),
      text: "真实测评模式：执行授权由后端策略（--permissions）裁决，此决定已写入本地会话记录。"
    });
    commit();
  }

  function deny(approvalId) {
    var approval = getApproval(approvalId);
    if (!approval || approval.status !== "pending") return;
    var taskId = approval.taskId;
    updateApproval(approvalId, { status: "denied", decided: U.timeNow(), mode: "deny" });
    addEvent(taskId, {
      kind: "SYSTEM",
      turn: maxTurn(taskId) + 1,
      title: "审批决定 · 拒绝",
      text: "真实测评模式：拒绝决定已记录；外部高影响动作不会执行。"
    });
    commit();
  }

  function retryApproval(approvalId) {
    var approval = getApproval(approvalId);
    if (!approval) return;
    var taskId = approval.taskId;
    updateApproval(approvalId, { status: "pending", decided: null, mode: null });
    addEvent(taskId, {
      kind: "SYSTEM",
      turn: maxTurn(taskId) + 1,
      title: "重新请求授权",
      text: "真实测评模式：重新提交给运行时（请在后端使用 --permissions ask 以获得交互审批）。"
    });
    commit();
  }

  function togglePause(taskId) {
    var task = getTask(taskId);
    if (!task) return;
    clearTimers(taskId);
    if (task.status === "paused") {
      setStatus(taskId, task.pendingApprovalId ? "approval" : "active", task.pendingApprovalId ? "等待高影响执行审批" : "任务已继续");
      addEvent(taskId, { kind: "SYSTEM", turn: maxTurn(taskId), title: "任务已继续", text: "未完成的步骤需要重新发起。" });
    } else if (task.status === "active" || task.status === "approval") {
      pauseRunningEvents(taskId);
      setStatus(taskId, "paused", "已暂停");
      addEvent(taskId, { kind: "SYSTEM", turn: maxTurn(taskId), title: "任务已暂停", text: "当前步骤停在安全边界；继续后需要重新发起未完成的执行。" });
    }
    commit();
  }

  function addRefToWorkingSet(refId) {
    var task = activeTask();
    if (!task || !getRef(refId)) return false;
    if (task.workingSet.indexOf(refId) >= 0) return false;
    updateTask(task.id, { workingSet: task.workingSet.concat([refId]) });
    commit();
    return true;
  }

  function removeRefFromWorkingSet(refId) {
    var task = activeTask();
    if (!task) return false;
    updateTask(task.id, {
      workingSet: task.workingSet.filter(function (id) { return id !== refId; })
    });
    commit();
    return true;
  }

  var skillRefMap = {
    "debug-session": "ctx_a1f0",
    "verify-receipt": "ctx_5c09",
    "wiki-compiler": "ctx_9d10",
    "context-audit": "ctx_15f7"
  };

  function toggleSkill(skillId) {
    var task = activeTask();
    var skill = getSkill(skillId);
    if (!task || !skill) return null;
    var active = task.activeSkills.indexOf(skillId) >= 0;
    var nextSkills = active
      ? task.activeSkills.filter(function (id) { return id !== skillId; })
      : task.activeSkills.concat([skillId]);
    var nextWorkingSet = task.workingSet.slice();
    var refId = skillRefMap[skillId];
    if (refId) {
      if (active) {
        nextWorkingSet = nextWorkingSet.filter(function (id) { return id !== refId; });
      } else if (nextWorkingSet.indexOf(refId) < 0) {
        nextWorkingSet.push(refId);
      }
    }
    updateTask(task.id, { activeSkills: nextSkills, workingSet: nextWorkingSet });
    addEvent(task.id, {
      kind: "SYSTEM",
      turn: maxTurn(task.id),
      title: active ? "Skill 已停用" : "Skill 已激活",
      text: skill.name + " · 仅对当前任务生效；激活 Skill 不授予任何权限。"
    });
    commit();
    return !active;
  }

  function setSettings(payload) {
    saveSettings({
      endpoint: payload.endpoint,
      model: payload.model,
      approvalPolicy: payload.approvalPolicy,
      theme: payload.theme,
      motion: payload.motion
    });
  }

  function reset() {
    Object.keys(timers).forEach(function (taskId) { clearTimers(taskId); });
    verifyEvents = {};
    removeRaw();
    state = null;
    load();
    commit();
  }

  function exportPayload(taskId) {
    var task = getTask(taskId);
    if (!task) return null;
    var events = eventsFor(taskId);
    return {
      schema: "sfai.trace/v2",
      exportedAt: new Date().toISOString(),
      workspace: state.meta.workspace,
      task: task,
      events: events,
      approvals: state.approvals.filter(function (approval) { return approval.taskId === taskId; }),
      context_refs: workingRefs(task),
      verification: task.verification
    };
  }

  function getTaskCounts() {
    return state.tasks.reduce(function (counts, task) {
      counts.total += 1;
      if (task.status === "done") counts.done += 1;
      else counts.open += 1;
      return counts;
    }, { total: 0, open: 0, done: 0 });
  }

  load();

  SFAI.store = {
    getState: getState,
    subscribe: subscribe,
    getTask: getTask,
    activeTask: activeTask,
    getApproval: getApproval,
    getRef: getRef,
    getSkill: getSkill,
    getCapability: getCapability,
    eventsFor: eventsFor,
    workingRefs: workingRefs,
    tokenTotal: tokenTotal,
    pendingApproval: pendingApproval,
    selectTask: selectTask,
    navigate: navigate,
    setTaskQuery: setTaskQuery,
    setContextQuery: setContextQuery,
    setTraceFilter: setTraceFilter,
    setRiskFilter: setRiskFilter,
    setSelectedRef: setSelectedRef,
    setSelectedCapability: setSelectedCapability,
    setInspectorTab: setInspectorTab,
    toggleInspector: toggleInspector,
    setMobileSidebar: setMobileSidebar,
    setMobileInspector: setMobileInspector,
    toggleTheme: toggleTheme,
    setSettings: setSettings,
    refreshFromBackend: refreshFromBackend,
    createTask: createTask,
    sendMessage: sendMessage,
    approve: approve,
    deny: deny,
    retryApproval: retryApproval,
    togglePause: togglePause,
    addRefToWorkingSet: addRefToWorkingSet,
    removeRefFromWorkingSet: removeRefFromWorkingSet,
    toggleSkill: toggleSkill,
    reset: reset,
    exportPayload: exportPayload,
    getTaskCounts: getTaskCounts,
    applyPreferences: applyPreferences,
    isStorageAvailable: function () { return storageAvailable; }
  };
})();
