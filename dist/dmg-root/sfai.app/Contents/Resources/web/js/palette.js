/* Command palette for a conversation-first workspace. */
(function () {
  "use strict";

  var SFAI = (window.SFAI = window.SFAI || {});
  var U = SFAI.util;
  var icon = function (name, cls) { return SFAI.icons.icon(name, cls); };
  var api = null;
  var inputEl = null;
  var listEl = null;
  var items = [];
  var index = -1;

  function actions() {
    var state = SFAI.store.getState();
    var task = SFAI.store.activeTask();
    var result = [
      { kind: "action", label: "新建对话", meta: "N", iconName: "plus", run: api.openNewChat },
      { kind: "action", label: "打开设置：任务", meta: "设置", iconName: "tasks", run: function () { api.openSettings("task"); } },
      { kind: "action", label: "打开设置：轨迹", meta: "设置", iconName: "trajectory", run: function () { api.openSettings("trajectory"); } },
      { kind: "action", label: "打开设置：上下文", meta: "设置", iconName: "context", run: function () { api.openSettings("context"); } },
      { kind: "action", label: "打开设置：能力", meta: "设置", iconName: "capabilities", run: function () { api.openSettings("capabilities"); } },
      { kind: "action", label: "打开设置：界面与运行", meta: "设置", iconName: "settings", run: function () { api.openSettings("run"); } },
      { kind: "action", label: "切换浅色/深色", meta: "", iconName: "moon", run: function () { SFAI.store.toggleTheme(); } }
    ];
    if (task) {
      result.push({ kind: "action", label: "导出当前会话轨迹", meta: task.id, iconName: "download", run: function () { api.exportTask(task.id); } });
      result.push({ kind: "action", label: "复制当前会话 ID", meta: task.id, iconName: "copy", run: function () { api.copy(task.id, "会话 ID 已复制"); } });
    }
    state.tasks.forEach(function (item) {
      result.push({
        kind: "session",
        label: "打开 " + item.id + " · " + item.title,
        meta: U.statusMeta(item.status).label,
        iconName: "tasks",
        run: function () { api.selectTask(item.id); }
      });
    });
    return result;
  }

  function updateSelection() {
    Array.prototype.slice.call(listEl.querySelectorAll("[data-palette-index]")).forEach(function (button) {
      var selected = Number(button.dataset.paletteIndex) === index;
      button.classList.toggle("is-selected", selected);
      button.setAttribute("aria-selected", selected ? "true" : "false");
    });
    var selected = listEl.querySelector('[data-palette-index="' + index + '"]');
    if (selected) {
      selected.scrollIntoView({ block: "nearest" });
      inputEl.setAttribute("aria-activedescendant", selected.id);
    }
  }

  function render(query) {
    var q = String(query || "").trim().toLowerCase();
    var all = actions();
    items = all.filter(function (item) {
      return !q || (item.label + " " + item.meta).toLowerCase().indexOf(q) >= 0;
    });
    if (!items.length) {
      listEl.innerHTML = '<div class="palette-empty">没有匹配的命令。</div>';
      index = -1;
      inputEl.removeAttribute("aria-activedescendant");
      return;
    }
    var groups = { action: [], session: [] };
    items.forEach(function (item, itemIndex) {
      item.index = itemIndex;
      groups[item.kind].push(item);
    });
    var html = "";
    ["action", "session"].forEach(function (kind) {
      if (!groups[kind].length) return;
      html += '<div class="palette-group">' + (kind === "action" ? "动作" : "会话") + "</div>";
      html += groups[kind].map(function (item) {
        return '<button class="palette-item" type="button" id="palette-item-' + item.index + '" data-palette-index="' + item.index +
          '" role="option" aria-selected="false" tabindex="-1">' +
          icon(item.iconName) + '<span class="palette-item-name">' + U.escapeHtml(item.label) +
          '</span><span class="palette-meta">' + U.escapeHtml(item.meta) + "</span></button>";
      }).join("");
    });
    listEl.innerHTML = html;
    index = 0;
    updateSelection();
  }

  function open() {
    U.openDialog("palette-dialog");
    inputEl.value = "";
    render("");
    window.requestAnimationFrame(function () {
      inputEl.focus();
    });
  }

  function execute(itemIndex) {
    var item = items[itemIndex];
    if (!item) return;
    U.closeDialog("palette-dialog");
    window.setTimeout(function () { item.run(); }, 0);
  }

  function handleKeydown(event) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (items.length) {
        index = (index + 1) % items.length;
        updateSelection();
      }
      return true;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      if (items.length) {
        index = (index - 1 + items.length) % items.length;
        updateSelection();
      }
      return true;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      if (index >= 0) execute(index);
      return true;
    }
    return false;
  }

  function init(options) {
    api = options;
    inputEl = document.getElementById("palette-input");
    listEl = document.getElementById("palette-list");
  }

  SFAI.palette = {
    init: init,
    open: open,
    execute: execute,
    handleInput: function (value) { render(value); },
    handleKeydown: handleKeydown
  };
})();
