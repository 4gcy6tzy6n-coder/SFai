/* Capabilities workspace: registry, risk filters, skill activation. */
(function () {
  "use strict";

  var SFAI = (window.SFAI = window.SFAI || {});
  var U = SFAI.util;
  var views = (SFAI.views = SFAI.views || {});
  var icon = function (name, cls) { return SFAI.icons.icon(name, cls); };

  var risks = [
    { id: "all", label: "全部" },
    { id: "read", label: "读取" },
    { id: "write", label: "写入" },
    { id: "external", label: "外部" },
    { id: "high", label: "高影响" }
  ];

  function filteredCapabilities(state) {
    return state.capabilities.filter(function (cap) {
      return state.riskFilter === "all" || cap.risk === state.riskFilter;
    });
  }

  function riskBadge(risk) {
    var meta = U.riskMeta(risk);
    return '<span class="risk-badge ' + meta.className + '">' + meta.label + "</span>";
  }

  function capabilityItem(cap, selected) {
    return '<button class="cap-item' + (selected ? " is-selected" : "") + '" type="button" data-action="select-capability" data-id="' +
      U.escapeHtml(cap.id) + '" aria-current="' + (selected ? "true" : "false") + '">' +
      '<span class="cap-item-head"><span class="cap-item-name">' + U.escapeHtml(cap.name) + "</span>" + riskBadge(cap.risk) + "</span>" +
      '<span class="cap-item-id">' + U.escapeHtml(cap.id) + " · v" + U.escapeHtml(cap.version) + "</span>" +
      '<span class="cap-item-desc">' + U.escapeHtml(cap.description) + "</span>" +
    "</button>";
  }

  views.renderCapabilityList = function () {
    var state = SFAI.store.getState();
    var host = document.getElementById("cap-list");
    if (!host) return;
    var caps = filteredCapabilities(state);
    host.innerHTML = caps.length
      ? caps.map(function (cap) { return capabilityItem(cap, cap.id === state.selectedCapabilityId); }).join("")
      : '<div class="cap-empty">这个风险等级下没有能力。</div>';
  };

  views.renderCapabilityDetail = function () {
    var state = SFAI.store.getState();
    var host = document.getElementById("cap-detail");
    if (!host) return;
    var caps = filteredCapabilities(state);
    var cap = SFAI.store.getCapability(state.selectedCapabilityId) || caps[0];
    if (!cap) {
      host.innerHTML = '<div class="cap-empty">选择一个能力查看操作与风险说明。</div>';
      return;
    }
    host.innerHTML = '<div class="cap-detail-head">' +
        '<div><p class="eyebrow">CAPABILITY</p><h2>' + U.escapeHtml(cap.name) + "</h2>" +
          '<p class="cap-detail-id">' + U.escapeHtml(cap.id) + " · v" + U.escapeHtml(cap.version) + "</p></div>" +
        riskBadge(cap.risk) +
      "</div>" +
      '<p class="cap-detail-desc">' + U.escapeHtml(cap.description) + "</p>" +
      '<dl class="cap-meta-grid">' +
        "<div><dt>risk class</dt><dd>" + U.escapeHtml(U.riskMeta(cap.risk).label) + "</dd></div>" +
        "<div><dt>safety</dt><dd>" + U.escapeHtml(cap.safety) + "</dd></div>" +
        "<div><dt>operations</dt><dd>" + cap.operations.length + " 个</dd></div>" +
        "<div><dt>version</dt><dd>v" + U.escapeHtml(cap.version) + "</dd></div>" +
      "</dl>" +
      '<section class="cap-ops"><h3>operations</h3><ul>' + cap.operations.map(function (op) {
        return "<li><span>" + U.escapeHtml(op.name) + "()</span><span>" + U.escapeHtml(op.description) + "</span></li>";
      }).join("") + "</ul></section>" +
      '<section class="cap-code"><h3>EXAMPLE CALL</h3><pre>' + U.escapeHtml(cap.schema) + "</pre>" +
        '<div class="cap-code-actions"><button class="btn btn--sm" type="button" data-action="copy-capability" data-id="' + U.escapeHtml(cap.id) + '">' +
          icon("copy") + "复制调用示例</button><span class=\"cap-hint\">激活能力不会授予任何权限。</span></div>" +
      "</section>";
  };

  views.renderSkillsList = function (task) {
    var host = document.getElementById("skills-list");
    if (!host) return;
    var state = SFAI.store.getState();
    if (!state.skills.length) {
      host.innerHTML = '<div class="cap-empty">还没有发现 Skill 包。</div>';
      return;
    }
    host.innerHTML = state.skills.map(function (skill) {
      var active = task && task.activeSkills.indexOf(skill.id) >= 0;
      return '<div class="skill-row">' +
        '<div class="skill-main">' +
          '<div class="skill-title-row"><span class="skill-title">' + U.escapeHtml(skill.name) + "</span>" +
            '<span class="skill-version">v' + U.escapeHtml(skill.version) + "</span>" +
            '<span class="skill-trust">trust · ' + U.escapeHtml(skill.trust) + "</span>" +
            (active ? '<span class="skill-trust skill-active">已激活</span>' : "") +
          "</div>" +
          '<p class="skill-desc">' + U.escapeHtml(skill.description) + "</p>" +
          '<div class="skill-refs">' + skill.refs.map(function (ref) {
            return '<span class="skill-ref">' + U.escapeHtml(ref) + "</span>";
          }).join("") + "</div>" +
        "</div>" +
        '<div class="skill-action"><button class="btn btn--sm' + (active ? " btn--danger-ghost" : "") + '" type="button" data-action="toggle-skill" data-id="' +
          U.escapeHtml(skill.id) + '">' + (active ? icon("close") + "停用" : icon("plus") + "激活") + "</button></div>" +
      "</div>";
    }).join("");
  };

  views.renderCapabilitiesPanels = function () {
    var state = SFAI.store.getState();
    var caps = filteredCapabilities(state);
    if (caps.length && !caps.some(function (cap) { return cap.id === state.selectedCapabilityId; })) {
      SFAI.store.setSelectedCapability(caps[0].id);
    }
    views.renderCapabilityList();
    views.renderCapabilityDetail();
    views.renderSkillsList(SFAI.store.activeTask());
  };

  views.renderCapabilitiesView = function (task, state) {
    var caps = filteredCapabilities(state);
    return '<div class="page page--wide">' +
      '<header class="page-head">' +
        '<div class="page-head-main">' +
          '<div class="eyebrow-row"><span class="eyebrow">CAPABILITIES</span><span class="eyebrow-sep" aria-hidden="true"></span><span class="eyebrow">REGISTRY · METADATA ONLY</span></div>' +
          "<h2>能力与 Skill</h2>" +
          '<p class="page-sub">能力是可发现、可检查、可执行的操作；Skill 是按任务激活的说明包。发现能力不会加载正文，激活 Skill 也不会绕过权限。</p>' +
        "</div>" +
        '<div class="head-actions"><span class="skill-chip">' + state.capabilities.length + " 个能力 · " + state.skills.length + " 个 Skill</span></div>" +
      "</header>" +
      '<div class="cap-layout">' +
        '<div class="cap-list-pane">' +
          '<div class="cap-filter" role="group" aria-label="按风险等级筛选">' + risks.map(function (risk) {
            return '<button class="chip" type="button" data-action="risk-filter" data-risk="' + risk.id + '" aria-pressed="' + (state.riskFilter === risk.id ? "true" : "false") + '">' + risk.label + "</button>";
          }).join("") + "</div>" +
          '<div class="cap-list" id="cap-list">' + caps.map(function (cap) {
            return capabilityItem(cap, cap.id === state.selectedCapabilityId);
          }).join("") + "</div>" +
        "</div>" +
        '<div class="cap-detail" id="cap-detail"></div>' +
      "</div>" +
      '<section class="skills-panel" aria-labelledby="skills-title">' +
        '<div class="skills-head"><div><h2 id="skills-title">Skill 包</h2>' +
          '<p>当前任务：' + (task ? U.escapeHtml(task.id + " · " + task.title) : "未选择任务") + "。激活只影响这个任务的工作集。</p></div></div>" +
        '<div id="skills-list"></div>' +
      "</section>" +
    "</div>";
  };
})();
