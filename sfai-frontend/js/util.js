/* Shared formatting, safety, clipboard, and dialog helpers. */
(function () {
  "use strict";

  var SFAI = (window.SFAI = window.SFAI || {});
  var toastTimer = 0;

  function escapeHtml(value) {
    return String(value == null ? "" : value)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  function escapeRegExp(value) {
    return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }

  function highlight(text, query) {
    var raw = String(text == null ? "" : text);
    var q = String(query == null ? "" : query).trim();
    if (!q) return escapeHtml(raw);
    var pattern;
    try {
      pattern = new RegExp(escapeRegExp(q), "gi");
    } catch (error) {
      return escapeHtml(raw);
    }
    var out = "";
    var last = 0;
    var match;
    while ((match = pattern.exec(raw)) !== null) {
      if (match[0] === "") {
        pattern.lastIndex += 1;
        continue;
      }
      out += escapeHtml(raw.slice(last, match.index));
      out += "<mark>" + escapeHtml(match[0]) + "</mark>";
      last = match.index + match[0].length;
    }
    out += escapeHtml(raw.slice(last));
    return out;
  }

  function truncate(value, max) {
    var text = String(value == null ? "" : value);
    if (text.length <= max) return text;
    return text.slice(0, Math.max(0, max - 1)).trimEnd() + "…";
  }

  function money(value) {
    var number = Number(value);
    if (!Number.isFinite(number)) number = 0;
    return "¥" + number.toFixed(2);
  }

  function number(value) {
    var parsed = Number(value);
    if (!Number.isFinite(parsed)) parsed = 0;
    return parsed.toLocaleString("zh-CN");
  }

  function tokens(value) {
    return number(value) + " tok";
  }

  function clamp(value, min, max) {
    return Math.min(max, Math.max(min, value));
  }

  function uid(prefix) {
    return String(prefix || "id") + "_" + Date.now().toString(36) + "_" + Math.random().toString(36).slice(2, 7);
  }

  function timeNow() {
    var now = new Date();
    return String(now.getHours()).padStart(2, "0") + ":" +
      String(now.getMinutes()).padStart(2, "0") + ":" +
      String(now.getSeconds()).padStart(2, "0");
  }

  function statusMeta(status) {
    var map = {
      active: { label: "进行中", className: "status-badge--active", dotClass: "dot--accent" },
      approval: { label: "待审批", className: "status-badge--approval", dotClass: "dot--warn" },
      done: { label: "已完成", className: "status-badge--done", dotClass: "dot--ok" },
      blocked: { label: "已阻塞", className: "status-badge--blocked", dotClass: "dot--danger" },
      paused: { label: "已暂停", className: "status-badge--paused", dotClass: "dot--warn" }
    };
    return map[status] || map.active;
  }

  function roleMeta(kind) {
    var map = {
      USER: { label: "USER", dotClass: "" },
      SYSTEM: { label: "SYSTEM", dotClass: "" },
      CONTEXT: { label: "CONTEXT", dotClass: "" },
      ASSISTANT: { label: "ASSISTANT", dotClass: "dot--accent" },
      TOOL: { label: "TOOL", dotClass: "dot--ok" },
      APPROVAL: { label: "APPROVAL", dotClass: "dot--warn" },
      VERIFICATION: { label: "VERIFICATION", dotClass: "dot--ok" }
    };
    return map[kind] || { label: kind || "EVENT", dotClass: "" };
  }

  function toolStatusMeta(status) {
    var map = {
      ok: { label: "成功", className: "tool-status--ok", dotClass: "dot--ok" },
      running: { label: "运行中", className: "tool-status--running", dotClass: "dot--accent" },
      paused: { label: "已暂停", className: "", dotClass: "dot--warn" },
      fail: { label: "失败", className: "tool-status--fail", dotClass: "dot--danger" },
      denied: { label: "已拒绝", className: "tool-status--fail", dotClass: "dot--danger" }
    };
    return map[status] || map.ok;
  }

  function approvalPolicyLabel(policy) {
    var map = {
      ask: "每次询问",
      auto: "自动批准",
      deny: "默认拒绝"
    };
    return map[policy] || "每次询问";
  }

  function riskMeta(risk) {
    var map = {
      read: { label: "读取", className: "" },
      write: { label: "写入", className: "risk-badge--write" },
      external: { label: "外部", className: "risk-badge--external" },
      high: { label: "高影响", className: "risk-badge--high" }
    };
    return map[risk] || { label: risk || "未知", className: "" };
  }

  function refKindLabel(kind) {
    var map = {
      source: "源码",
      doc: "文档",
      decision: "决策",
      derived: "派生",
      skill: "Skill"
    };
    return map[kind] || "引用";
  }

  function resolveTheme(pref) {
    if (pref === "light" || pref === "dark") return pref;
    return window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }

  function copyText(text) {
    if (navigator.clipboard && window.isSecureContext) {
      return navigator.clipboard.writeText(String(text)).then(function () {
        return true;
      }).catch(function () {
        return legacyCopy(text);
      });
    }
    return Promise.resolve(legacyCopy(text));
  }

  function legacyCopy(text) {
    var area = document.createElement("textarea");
    area.value = String(text);
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.appendChild(area);
    area.select();
    var ok = false;
    try {
      ok = document.execCommand("copy");
    } catch (error) {
      ok = false;
    }
    area.remove();
    return ok;
  }

  function downloadJson(filename, payload) {
    var blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
    var url = URL.createObjectURL(blob);
    var link = document.createElement("a");
    link.href = url;
    link.download = filename;
    link.rel = "noopener";
    document.body.appendChild(link);
    link.click();
    link.remove();
    window.setTimeout(function () {
      URL.revokeObjectURL(url);
    }, 2000);
  }

  function toast(message) {
    var el = document.getElementById("toast");
    if (!el) return;
    el.textContent = message;
    el.hidden = false;
    requestAnimationFrame(function () {
      el.classList.add("is-visible");
    });
    window.clearTimeout(toastTimer);
    toastTimer = window.setTimeout(function () {
      el.classList.remove("is-visible");
      toastTimer = window.setTimeout(function () {
        el.hidden = true;
      }, 240);
    }, 2600);
  }

  function openDialog(id) {
    var dialog = document.getElementById(id);
    if (!dialog) return null;
    if (typeof dialog.showModal === "function") {
      if (!dialog.open) dialog.showModal();
    } else {
      dialog.setAttribute("open", "");
    }
    return dialog;
  }

  function closeDialog(id) {
    var dialog = document.getElementById(id);
    if (!dialog) return;
    if (typeof dialog.close === "function" && dialog.open) {
      dialog.close();
    } else {
      dialog.removeAttribute("open");
    }
  }

  function focusMain() {
    var main = document.getElementById("main");
    if (main) main.focus({ preventScroll: true });
  }

  SFAI.util = {
    escapeHtml: escapeHtml,
    escapeRegExp: escapeRegExp,
    highlight: highlight,
    truncate: truncate,
    money: money,
    number: number,
    tokens: tokens,
    clamp: clamp,
    uid: uid,
    timeNow: timeNow,
    statusMeta: statusMeta,
    roleMeta: roleMeta,
    toolStatusMeta: toolStatusMeta,
    approvalPolicyLabel: approvalPolicyLabel,
    riskMeta: riskMeta,
    refKindLabel: refKindLabel,
    resolveTheme: resolveTheme,
    copyText: copyText,
    downloadJson: downloadJson,
    toast: toast,
    openDialog: openDialog,
    closeDialog: closeDialog,
    focusMain: focusMain
  };
})();
