/* Settings sheet: task detail, trajectory, context, capabilities, run settings. */
(function () {
  "use strict";

  var SFAI = (window.SFAI = window.SFAI || {});
  var U = SFAI.util;
  var views = (SFAI.views = SFAI.views || {});
  var icon = function (name, cls) { return SFAI.icons.icon(name, cls); };

  function acceptanceIcon(state) {
    if (state === "pass") return icon("check");
    if (state === "fail") return icon("alert");
    return icon("circle");
  }

  function taskPane(task) {
    if (!task) return '<div class="settings-empty">还没有会话。</div>';
    var state = SFAI.store.getState();
    var events = SFAI.store.eventsFor(task.id);
    var pending = SFAI.store.pendingApproval(task);
    var verificationEvent = events.filter(function (event) { return event.kind === "VERIFICATION"; }).pop();
    var refs = SFAI.store.workingRefs(task);
    var skills = task.activeSkills.map(function (id) {
      var skill = SFAI.store.getSkill(id);
      return skill ? skill.name : id;
    });
    var percent = task.budget.cap > 0 ? U.clamp((task.budget.used / task.budget.cap) * 100, 0, 100) : 0;
    var meterClass = percent >= 100 ? " meter-fill--danger" : percent >= 80 ? " meter-fill--warn" : "";

    var constraints = task.constraints.length
      ? '<div class="settings-rule-list">' + task.constraints.map(function (item, index) {
        return '<div class="settings-rule"><span class="mono">' + String(index + 1).padStart(2, "0") + "</span><span>" + U.escapeHtml(item) + "</span></div>";
      }).join("") + "</div>"
      : '<p class="settings-muted">没有额外约束。</p>';
    var acceptance = '<div class="settings-rule-list">' + task.acceptance.map(function (rule) {
      return '<div class="settings-rule settings-rule--' + U.escapeHtml(rule.state) + '">' + acceptanceIcon(rule.state) + "<span>" + U.escapeHtml(rule.label) + "</span></div>";
    }).join("") + "</div>";
    var skillsBlock = skills.length
      ? '<div class="skill-chips">' + skills.map(function (name) { return '<span class="skill-chip">' + U.escapeHtml(name) + "</span>"; }).join("") + "</div>"
      : '<p class="settings-muted">当前会话没有激活 Skill。</p>';
    var verification = task.verification
      ? '<div class="settings-receipt settings-receipt--' + (task.verification.verdict === "pass" ? "pass" : "fail") + '">' +
        '<div class="settings-receipt-head">' + (task.verification.verdict === "pass" ? icon("check") : icon("alert")) +
        "<span>" + U.escapeHtml(task.verification.label) + '</span><span class="mono">' + U.escapeHtml(task.verification.time || "") + "</span></div>" +
        (verificationEvent && verificationEvent.findings ? '<div class="settings-rule-list">' + verificationEvent.findings.map(function (finding) {
          return '<div class="settings-rule settings-rule--' + U.escapeHtml(finding.state) + '">' + acceptanceIcon(finding.state) + "<span>" + U.escapeHtml(finding.label) + "</span></div>";
        }).join("") + "</div>" : "") +
      "</div>"
      : '<p class="settings-muted">尚未核验。高影响操作完成后必须运行独立核验。</p>';
    var workingSet = refs.length
      ? '<div class="settings-ref-list">' + refs.map(function (ref) {
        return '<div class="settings-ref"><div><span class="settings-ref-title">' + U.escapeHtml(ref.title) + "</span>" +
          '<span class="settings-ref-path mono">' + U.escapeHtml(ref.path) + "</span></div>" +
          '<span class="settings-ref-tokens mono">' + U.tokens(ref.tokens) + "</span></div>";
      }).join("") + "</div>" +
        '<button class="btn btn--sm settings-card-action" type="button" data-action="settings-tab" data-tab="context">在上下文中管理</button>'
      : '<p class="settings-muted">工作集为空。</p>';
    var approvalNotice = pending
      ? '<div class="settings-approval-note"><div><span class="dot dot--warn" aria-hidden="true"></span><strong>等待审批：' +
        U.escapeHtml(pending.action) + "</strong><p>" + U.escapeHtml(pending.command) + "</p></div>" +
        '<button class="btn btn--sm" type="button" data-action="focus-approval">回到对话处理</button></div>'
      : "";
    var status = U.statusMeta(task.status);
    var recent = events.slice().reverse().slice(0, 5).map(function (event) {
      var role = U.roleMeta(event.kind);
      var title = event.title || event.summary || event.tool || event.text || event.kind;
      return '<div class="settings-activity-row"><time class="mono">' + U.escapeHtml(event.time || "") + "</time>" +
        '<span class="settings-activity-kind">' + U.escapeHtml(role.label) + "</span>" +
        '<span class="settings-activity-title">' + U.escapeHtml(title) + "</span></div>";
    }).join("");

    return '<div class="settings-pane">' +
      '<header class="settings-pane-head">' +
        '<div><p class="eyebrow">TASK ' + U.escapeHtml(task.id) + '</p><h2>' + U.escapeHtml(task.title) + "</h2>" +
          '<p class="settings-pane-goal">' + U.escapeHtml(task.goal) + "</p></div>" +
        '<div class="settings-pane-actions">' +
          '<button class="btn btn--sm" type="button" data-action="copy-id" data-id="' + U.escapeHtml(task.id) + '">' + icon("copy") + "复制 ID</button>" +
          '<button class="btn btn--sm" type="button" data-action="export-task" data-id="' + U.escapeHtml(task.id) + '">' + icon("download") + "导出</button>" +
        "</div>" +
      "</header>" +
      '<div class="settings-stat-row">' +
        '<div class="settings-stat"><span class="settings-stat-value">' + U.escapeHtml(status.label) + '</span><span class="settings-stat-label">状态</span></div>' +
        '<div class="settings-stat"><span class="settings-stat-value mono">' + U.escapeHtml(task.model) + '</span><span class="settings-stat-label">模型</span></div>' +
        '<div class="settings-stat"><span class="settings-stat-value mono">' + events.length + '</span><span class="settings-stat-label">事件</span></div>' +
        '<div class="settings-stat"><span class="settings-stat-value mono">' + U.money(task.budget.used) + '</span><span class="settings-stat-label">已用 / ' + U.money(task.budget.cap) + "</span></div>" +
      "</div>" +
      approvalNotice +
      '<div class="settings-grid">' +
        '<section class="settings-card"><h3>固定约束</h3>' + constraints + "</section>" +
        '<section class="settings-card"><h3>验收规则</h3>' + acceptance + "</section>" +
        '<section class="settings-card"><h3>当前 Skill</h3>' + skillsBlock + "</section>" +
        '<section class="settings-card"><h3>最近核验</h3>' + verification + "</section>" +
        '<section class="settings-card settings-card--wide"><h3>最近活动</h3>' +
          '<div class="settings-activity">' + recent + "</div>" +
          '<button class="btn btn--sm settings-card-action" type="button" data-action="settings-tab" data-tab="trajectory">查看完整轨迹</button>' +
        "</section>" +
        '<section class="settings-card settings-card--wide"><h3>工作集</h3>' + workingSet + "</section>" +
        '<section class="settings-card"><h3>预算</h3>' +
          '<div class="settings-cost-value mono">' + U.money(task.budget.used) + " / " + U.money(task.budget.cap) + "</div>" +
          '<div class="meter" role="progressbar" aria-label="预算使用" aria-valuemin="0" aria-valuemax="' + task.budget.cap + '" aria-valuenow="' + task.budget.used + '"><div class="meter-fill' + meterClass + '" style="width:' + percent.toFixed(1) + '%"></div></div>' +
          '<div class="settings-cost-meta"><span>tokens <b class="mono">' + U.number(task.budget.tokens) + "</b></span>" +
            "<span>调用 <b class=\"mono\">" + U.number(task.budget.calls) + "</b></span>" +
            "<span>墙钟 <b class=\"mono\">" + U.escapeHtml(task.budget.wall) + "</b></span></div>" +
        "</section>" +
        '<section class="settings-card"><h3>会话信息</h3>' +
          '<div class="settings-kv"><span>创建</span><b class="mono">' + U.escapeHtml(task.createdAt) + "</b></div>" +
          '<div class="settings-kv"><span>模式</span><b class="mono">' + U.escapeHtml(task.mode) + "</b></div>" +
          '<div class="settings-kv"><span>审批</span><b class="mono">' + U.escapeHtml(U.approvalPolicyLabel(state.settings.approvalPolicy)) + "</b></div>" +
        "</section>" +
      "</div>" +
    "</div>";
  }

  function switchRow(id, name, checked, label, hint) {
    return '<label class="settings-switch" for="' + id + '">' +
      '<span class="settings-switch-copy"><span class="field-label">' + U.escapeHtml(label) + '</span>' +
        '<span class="field-hint">' + U.escapeHtml(hint) + "</span></span>" +
      '<span class="settings-switch-control">' +
        '<input id="' + id + '" name="' + name + '" type="checkbox"' + (checked ? " checked" : "") + ">" +
        '<span class="settings-switch-track" aria-hidden="true"><span class="settings-switch-thumb"></span></span>' +
      "</span>" +
    "</label>";
  }

  function runPane(state) {
    var settings = state.settings;
    var density = settings.density === "compact" ? "compact" : "comfortable";
    var sendKey = settings.sendKey === "enter" ? "enter" : "meta";
    var timestamps = settings.timestamps !== false;
    var autoFollow = settings.autoFollow !== false;

    // ---- real provider/model switching (from /api/providers when connected) ----
    var PROVIDER_PRESETS = {
      deepseek: { label: "DeepSeek", models: ["deepseek-chat", "deepseek-reasoner"] },
      bailian: { label: "百炼（多模型）", models: ["deepseek-v4.1-flash", "qwen4-max", "qwen3.8-flash", "glm-5.3"] },
      mimo: { label: "Xiaomi MiMo", models: ["mimo-v2.6-pro", "mimo-v2.6-flash"] },
      qwen: { label: "Qwen（DashScope）", models: ["qwen-plus", "qwen-max"] },
      custom: { label: "自定义 OpenAI 兼容", models: [] }
    };
    var live = (window.SFAIStore && window.SFAIStore.getState().providers) || [];
    var currentProvider = settings.provider || "deepseek";
    var preset = PROVIDER_PRESETS[currentProvider] || PROVIDER_PRESETS.deepseek;
    var providerOptions = "";
    var providerHint = "";
    var modelPool = preset.models.slice();
    if (live.length) {
      providerOptions = live.map(function (p) {
        return '<option value="' + U.escapeHtml(p.name) + '"' + (p.name === currentProvider ? " selected" : "") + ">" +
          U.escapeHtml(p.name + " · " + (p.model || "")) + (p.configured ? " ✓" : "（未配置 key）") + "</option>";
      }).join("");
      var liveCurrent = live.find(function (p) { return p.name === currentProvider; });
      providerHint = liveCurrent
        ? "来自运行时 /api/providers · " + (liveCurrent.note || "")
        : "来自运行时 /api/providers";
      if (liveCurrent && liveCurrent.model) modelPool = [liveCurrent.model].concat(modelPool);
    } else {
      providerOptions = Object.keys(PROVIDER_PRESETS).map(function (key) {
        return '<option value="' + key + '"' + (key === currentProvider ? " selected" : "") + ">" + PROVIDER_PRESETS[key].label + "</option>";
      }).join("");
      providerHint = "连接运行时后显示各 Provider 的配置状态。";
    }
    var modelOptions = Array.from(new Set(modelPool)).map(function (m) {
      return '<option value="' + U.escapeHtml(m) + '">';
    }).join("");

    return '<div class="settings-pane settings-pane--run">' +
      '<header class="settings-pane-head"><div><p class="eyebrow">SETTINGS</p><h2>运行与界面</h2>' +
        '<p class="settings-pane-goal">连接、审批与界面偏好只保存在本机。配置 endpoint（sfai serve 地址）后进入真实模式，向真实模型发起请求。</p></div></header>' +
      '<form id="settings-form" class="settings-form" novalidate>' +
        '<section class="settings-section">' +
          '<div class="settings-section-head"><h3>连接</h3><p>真实接入：运行时地址、Provider 与模型切换。</p></div>' +
          '<div class="settings-section-body">' +
            '<div class="field"><label class="field-label" for="settings-endpoint">运行时地址（sfai serve）</label>' +
              '<input id="settings-endpoint" name="endpoint" type="url" value="' + U.escapeHtml(settings.endpoint) + '" placeholder="http://127.0.0.1:8787" spellcheck="false" aria-describedby="settings-endpoint-hint settings-endpoint-error">' +
              '<p class="field-hint" id="settings-endpoint-hint">桌面 App 内置运行时可留空（自动注入）。指向任意 sfai serve 即可远程接入。</p>' +
              '<p class="field-error" id="settings-endpoint-error" hidden></p></div>' +
            '<div class="field"><label class="field-label" for="settings-provider">Provider（可切换）</label>' +
              '<select id="settings-provider" name="provider">' + providerOptions + "</select>" +
              '<p class="field-hint" id="settings-provider-hint">' + providerHint + "</p></div>" +
            '<div class="field"><label class="field-label" for="settings-model">模型（可切换）</label>' +
              '<input id="settings-model" name="model" list="settings-model-list" value="' + U.escapeHtml(settings.model || "") + '" placeholder="deepseek-chat" spellcheck="false">' +
              '<datalist id="settings-model-list">' + modelOptions + "</datalist>" +
              '<p class="field-hint">按 Provider 预填默认模型；可输入该 Provider 的任意模型名（如 deepseek-v4.1-flash / qwen4-max / mimo-v2.6-pro）。</p></div>' +
          "</div>" +
        "</section>" +
        '<section class="settings-section">' +
          '<div class="settings-section-head"><h3>审批</h3><p>高影响操作在对话里如何请求确认。</p></div>' +
          '<div class="settings-section-body">' +
            '<div class="field"><label class="field-label" for="settings-approval">高影响操作</label><select id="settings-approval" name="approvalPolicy">' +
              '<option value="ask"' + (settings.approvalPolicy === "ask" ? " selected" : "") + ">每次询问</option>" +
              '<option value="deny"' + (settings.approvalPolicy === "deny" ? " selected" : "") + ">默认拒绝</option>" +
              '<option value="auto"' + (settings.approvalPolicy === "auto" ? " selected" : "") + ">自动批准</option>" +
            '</select><p class="field-hint">策略只影响后续审批；已提交的审批需要单独处理。</p></div>' +
          "</div>" +
        "</section>" +
        '<section class="settings-section">' +
          '<div class="settings-section-head"><h3>外观</h3><p>主题、信息密度与消息时间的显示方式。</p></div>' +
          '<div class="settings-section-body">' +
            '<fieldset class="field"><legend class="field-label">主题</legend><div class="segmented" role="radiogroup" aria-label="主题">' +
              '<label class="segment"><input type="radio" name="theme" value="light"' + (settings.theme === "light" ? " checked" : "") + '><span class="theme-choice"><span class="theme-swatch theme-swatch--light" aria-hidden="true"></span><span>浅色</span></span></label>' +
              '<label class="segment"><input type="radio" name="theme" value="dark"' + (settings.theme === "dark" ? " checked" : "") + '><span class="theme-choice"><span class="theme-swatch theme-swatch--dark" aria-hidden="true"></span><span>深色</span></span></label>' +
              '<label class="segment"><input type="radio" name="theme" value="system"' + (settings.theme === "system" ? " checked" : "") + '><span class="theme-choice"><span class="theme-swatch theme-swatch--system" aria-hidden="true"></span><span>跟随系统</span></span></label>' +
            "</div></fieldset>" +
            '<div class="form-grid">' +
              '<div class="field"><label class="field-label" for="settings-density">信息密度</label><select id="settings-density" name="density">' +
                '<option value="comfortable"' + (density === "comfortable" ? " selected" : "") + ">标准</option>" +
                '<option value="compact"' + (density === "compact" ? " selected" : "") + ">紧凑</option>" +
              "</select></div>" +
              '<div class="field"><label class="field-label" for="settings-motion">界面动效</label><select id="settings-motion" name="motion">' +
                '<option value="system"' + (settings.motion === "system" ? " selected" : "") + ">跟随系统</option>" +
                '<option value="reduced"' + (settings.motion === "reduced" ? " selected" : "") + ">减少动效</option>" +
              "</select></div>" +
            "</div>" +
            switchRow("settings-timestamps", "timestamps", timestamps, "显示消息时间", "关闭后隐藏对话中的发送与步骤时间。") +
          "</div>" +
        "</section>" +
        '<section class="settings-section">' +
          '<div class="settings-section-head"><h3>对话</h3><p>输入区发送方式与自动跟随。</p></div>' +
          '<div class="settings-section-body">' +
            '<div class="field"><label class="field-label" for="settings-send-key">发送方式</label><select id="settings-send-key" name="sendKey">' +
              '<option value="meta"' + (sendKey === "meta" ? " selected" : "") + ">⌘↵ 发送 · Enter 换行</option>" +
              '<option value="enter"' + (sendKey === "enter" ? " selected" : "") + ">Enter 发送 · Shift+Enter 换行</option>" +
            "</select></div>" +
            switchRow("settings-autofollow", "autoFollow", autoFollow, "自动跟随新消息", "对话靠近底部时，收到新步骤自动滚动到最新。") +
          "</div>" +
        "</section>" +
        '<section class="settings-section settings-section--last">' +
          '<div class="settings-section-head"><h3>数据</h3><p>真实数据来自 Decide-Act 运行时；本地仅缓存会话状态。</p></div>' +
          '<div class="settings-section-body">' +
            '<div class="settings-reset-zone"><div><p class="field-label">本地状态</p><p class="field-hint">清除本地缓存的会话状态（不影响运行时数据）。</p></div>' +
              '<button class="btn btn--danger-ghost" type="button" data-action="reset-demo">重置本地状态</button></div>' +
            '<div class="reset-confirm" id="reset-confirm" hidden><p>这会清除本地缓存状态，无法撤销。</p>' +
              '<div class="dialog-actions"><button class="btn" type="button" data-action="cancel-reset">取消</button>' +
              '<button class="btn btn--danger" type="button" data-action="confirm-reset">确认重置</button></div></div>' +
          "</div>" +
        "</section>" +
        '<footer class="settings-form-foot"><p class="field-hint">设置保存在浏览器本地</p>' +
          '<button class="btn btn--primary" type="submit">保存设置</button></footer>' +
      "</form>" +
    "</div>";
  }

  views.renderSettingsContent = function (state, task, tab) {
    if (tab === "trajectory") {
      return '<div class="settings-pane settings-pane--embed">' + views.renderTrajectoryView(task, state) + "</div>";
    }
    if (tab === "context") {
      return '<div class="settings-pane settings-pane--embed">' + views.renderContextView(task, state) + "</div>";
    }
    if (tab === "capabilities") {
      return '<div class="settings-pane settings-pane--embed">' + views.renderCapabilitiesView(task, state) + "</div>";
    }
    if (tab === "run") return runPane(state);
    return taskPane(task);
  };
})();
