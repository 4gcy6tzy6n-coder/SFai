/* Context workspace: lexical search, bounded excerpt, working set. */
(function () {
  "use strict";

  var SFAI = (window.SFAI = window.SFAI || {});
  var U = SFAI.util;
  var views = (SFAI.views = SFAI.views || {});
  var icon = function (name, cls) { return SFAI.icons.icon(name, cls); };

  function scoreRef(ref, query) {
    if (!query) return 0;
    var terms = query.toLowerCase().split(/\s+/).filter(Boolean);
    var title = String(ref.title || "").toLowerCase();
    var path = String(ref.path || "").toLowerCase();
    var hay = [ref.title, ref.path, ref.summary, ref.excerpt, ref.kind, ref.provenance].join(" ").toLowerCase();
    var score = 0;
    terms.forEach(function (term) {
      if (title.indexOf(term) >= 0) score += 4;
      if (path.indexOf(term) >= 0) score += 2;
      if (hay.indexOf(term) >= 0) score += 1;
    });
    return score;
  }

  function resultRefs(state) {
    var query = String(state.contextQuery || "").trim();
    var refs = SFAI.store.getState().contextRefs.slice();
    if (!query) return refs;
    return refs
      .map(function (ref) { return { ref: ref, score: scoreRef(ref, query) }; })
      .filter(function (item) { return item.score > 0; })
      .sort(function (a, b) { return b.score - a.score || a.ref.title.localeCompare(b.ref.title); })
      .map(function (item) { return item.ref; });
  }

  function resultsHtml(state) {
    var refs = resultRefs(state);
    var query = state.contextQuery || "";
    if (!refs.length) {
      return '<div class="context-empty"><span>没有匹配的引用。</span><button class="btn btn--sm" type="button" data-action="clear-context-search">清除检索</button></div>';
    }
    return refs.map(function (ref) {
      var kind = U.refKindLabel(ref.kind);
      return '<button class="context-result' + (ref.id === state.selectedRefId ? " is-selected" : "") + '" type="button" data-action="select-ref" data-id="' +
        U.escapeHtml(ref.id) + '" aria-current="' + (ref.id === state.selectedRefId ? "true" : "false") + '">' +
        '<span class="context-result-title">' + U.highlight(ref.title, query) + "</span>" +
        '<span class="context-result-path">' + U.highlight(ref.path, query) + "</span>" +
        '<span class="context-result-summary">' + U.highlight(ref.summary, query) + "</span>" +
        '<span class="context-result-meta"><span>' + kind + "</span><span aria-hidden=\"true\">·</span><span>" + U.escapeHtml(ref.id) +
          "</span><span aria-hidden=\"true\">·</span><span>" + U.tokens(ref.tokens) + "</span></span>" +
      "</button>";
    }).join("");
  }

  function previewHtml(state, task) {
    var ref = SFAI.store.getRef(state.selectedRefId) || resultRefs(state)[0];
    if (!ref) {
      return '<div class="context-empty">选择一条引用查看有界摘录与 provenance。</div>';
    }
    var query = state.contextQuery || "";
    var inWorkingSet = task && task.workingSet.indexOf(ref.id) >= 0;
    return '<div class="preview-head">' +
        '<div><p class="eyebrow">' + U.escapeHtml(U.refKindLabel(ref.kind)) + " · " + U.escapeHtml(ref.id) + "</p>" +
          '<h2 class="preview-title" id="preview-title" tabindex="-1">' + U.highlight(ref.title, query) + "</h2>" +
          '<p class="preview-path">' + U.highlight(ref.path, query) + "</p></div>" +
        '<div class="preview-actions">' +
          '<button class="btn btn--sm" type="button" data-action="toggle-working-ref" data-id="' + U.escapeHtml(ref.id) + '">' +
            icon(inWorkingSet ? "close" : "plus") + (inWorkingSet ? "移出工作集" : "加入工作集") + "</button>" +
          '<button class="btn btn--sm" type="button" data-action="copy-ref" data-id="' + U.escapeHtml(ref.id) + '" aria-label="复制引用 ID">' + icon("copy") + "</button>" +
        "</div>" +
      "</div>" +
      '<div class="preview-meta">' +
        "<span>revision " + U.escapeHtml(ref.revision) + "</span>" +
        "<span>" + U.tokens(ref.tokens) + "</span>" +
        "<span>updated " + U.escapeHtml(ref.updated) + "</span>" +
        (inWorkingSet ? '<span class="skill-active">已在工作集</span>' : "") +
      "</div>" +
      '<section class="preview-excerpt"><p class="eyebrow">EXCERPT</p><p>' + U.highlight(ref.excerpt, query) + "</p></section>" +
      '<div class="preview-provenance"><span class="eyebrow">PROVENANCE</span><br>' + U.escapeHtml(ref.provenance) +
        "<br>引用只返回有界摘录；原始来源仍保留在不可变存储中。</div>";
  }

  views.renderContextResults = function () {
    var state = SFAI.store.getState();
    var results = document.getElementById("context-results");
    var preview = document.getElementById("context-preview");
    if (results) results.innerHTML = resultsHtml(state);
    if (preview) preview.innerHTML = previewHtml(state, SFAI.store.activeTask());
    var count = document.getElementById("context-result-count");
    if (count) count.textContent = resultRefs(state).length + " 条";
    var input = document.getElementById("context-search-input");
    if (input && input.value !== state.contextQuery) input.value = state.contextQuery;
  };

  views.renderContextView = function (task, state) {
    var totalTokens = task ? SFAI.store.tokenTotal(task) : 0;
    var workingCount = task ? task.workingSet.length : 0;
    return '<div class="page page--context">' +
      '<header class="page-head">' +
        '<div class="page-head-main">' +
          '<div class="eyebrow-row"><span class="eyebrow">CONTEXT</span><span class="eyebrow-sep" aria-hidden="true"></span><span class="eyebrow">ROOT INDEX · FTS5</span></div>' +
          "<h2>可寻址上下文</h2>" +
          '<p class="page-sub">工作集只保留目标、固定约束与信息地址；模型按需读取有界摘录，并始终保留回到原始来源的路径。</p>' +
        "</div>" +
        '<div class="head-actions"><span class="skill-chip">' + workingCount + " 个引用 · " + U.tokens(totalTokens) + "</span></div>" +
      "</header>" +
      '<div class="context-layout">' +
        '<div class="context-list-pane">' +
          '<div class="context-search">' +
            '<label class="field-label" for="context-search-input">检索上下文</label>' +
            '<div class="context-search-row"><div class="input-wrap input-wrap--search"><span class="input-icon" data-icon-slot="search" aria-hidden="true"></span>' +
              '<input id="context-search-input" type="search" value="' + U.escapeHtml(state.contextQuery || "") + '" placeholder="路径、标题或片段" autocomplete="off" spellcheck="false"></div>' +
              '<span class="context-count" id="context-result-count">' + resultRefs(state).length + " 条</span></div>" +
          "</div>" +
          '<div class="context-results" id="context-results">' + resultsHtml(state) + "</div>" +
        "</div>" +
        '<div class="context-preview" id="context-preview" tabindex="-1" aria-live="polite">' + previewHtml(state, task) + "</div>" +
      "</div>" +
    "</div>";
  };
})();
