/* sfai console state seed. No demo content: everything is real runtime data. */
(function () {
  "use strict";

  var SFAI = (window.SFAI = window.SFAI || {});

  var seed = {
    meta: {
      version: "0.2.0",
      workspace: "system-for-future-ai",
      generatedAt: null
    },

    settings: {
      theme: "system",
      endpoint: "",
      model: "deepseek-chat",
      approvalPolicy: "ask",
      motion: "system",
      density: "comfortable",
      timestamps: true,
      sendKey: "meta",
      autoFollow: true
    },

    // 真实测评模式：无演示数据。任务/事件/回执/引用均来自真实运行时。
    tasks: [],
    events: [],
    approvals: [],
    contextRefs: [],
    capabilities: [],
    skills: []
  };

  SFAI.seed = function () {
    return JSON.parse(JSON.stringify(seed));
  };

  SFAI.seedMeta = seed.meta;
})();
