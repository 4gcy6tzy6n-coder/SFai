/* Demo workspace for the sfai console. All runtime facts are sample data. */
(function () {
  "use strict";

  var SFAI = (window.SFAI = window.SFAI || {});

  var seed = {
    meta: {
      version: "0.5.0",
      workspace: "system-for-future-ai",
      generatedAt: "2026-11-16T12:03:08+08:00"
    },

    settings: {
      theme: "system",
      endpoint: "http://127.0.0.1:8787/v1",
      model: "deepseek-chat",
      approvalPolicy: "ask",
      motion: "system",
      density: "comfortable",
      timestamps: true,
      sendKey: "meta",
      autoFollow: true
    },

    tasks: [
      {
        id: "T-1042",
        title: "为 M6 校验补齐回放诊断",
        goal: "让 verify 在回放不一致时输出可定位的差异摘要，同时保持现有验收规则不变。",
        status: "approval",
        statusNote: "等待高影响执行审批",
        model: "deepseek-chat",
        mode: "standard",
        updatedAt: "刚刚",
        createdAt: "12:03",
        constraints: ["验收规则不得放宽", "外部写入需要审批", "所有成本必须计量"],
        acceptance: [
          { id: "a1", label: "cargo test 全绿", state: "pending" },
          { id: "a2", label: "replay 状态与任务状态一致", state: "pending" },
          { id: "a3", label: "失败时输出差异摘要", state: "pending" }
        ],
        budget: { used: 0.42, cap: 5, tokens: 128400, calls: 19, wall: "12m 08s" },
        stats: { turns: 3, steps: 10, duration: "12m 08s", costSeries: [2, 4, 1, 6, 3, 2, 5, 4, 2, 3] },
        workingSet: ["ctx_8f21", "ctx_0c7d", "ctx_a1f0"],
        activeSkills: ["debug-session", "verify-receipt"],
        pendingApprovalId: "ap_1042",
        verification: null
      },
      {
        id: "T-1039",
        title: "定位 FTS5 检索召回下降",
        goal: "比较最近三次索引重建的召回结果，找出 FTS5 查询改写导致的漏召回。",
        status: "active",
        statusNote: "正在对齐漏召回样本",
        model: "deepseek-chat",
        mode: "standard",
        updatedAt: "8 分钟前",
        createdAt: "12:11",
        constraints: ["只读分析，不重建生产索引", "保留原始查询日志"],
        acceptance: [
          { id: "a1", label: "三组查询结果对齐", state: "pending" },
          { id: "a2", label: "漏召回样本可复现", state: "pending" },
          { id: "a3", label: "生产索引未被修改", state: "pass" }
        ],
        budget: { used: 0.18, cap: 2, tokens: 54100, calls: 8, wall: "4m 31s" },
        stats: { turns: 2, steps: 5, duration: "4m 31s", costSeries: [1, 2, 3, 2, 4] },
        workingSet: ["ctx_15f7", "ctx_6a02"],
        activeSkills: ["debug-session"],
        pendingApprovalId: null,
        verification: null
      },
      {
        id: "T-1037",
        title: "整理 ADR-012 迁移记录",
        goal: "把 Python → Rust 的决策、未决问题与证据整理成可检索的 ADR 记录。",
        status: "done",
        statusNote: "核验通过，任务已归档",
        model: "deepseek-reasoner",
        mode: "standard",
        updatedAt: "今天 11:42",
        createdAt: "11:21",
        constraints: ["不删除旧 ADR", "保持决策日期可追溯"],
        acceptance: [
          { id: "a1", label: "决策与证据可追溯", state: "pass" },
          { id: "a2", label: "未决问题已标注", state: "pass" },
          { id: "a3", label: "旧 ADR 未被修改", state: "pass" }
        ],
        budget: { used: 0.36, cap: 3, tokens: 96300, calls: 14, wall: "18m 42s" },
        stats: { turns: 3, steps: 8, duration: "18m 42s", costSeries: [3, 2, 5, 4, 2, 3, 1] },
        workingSet: ["ctx_2b3e", "ctx_44c9"],
        activeSkills: ["verify-receipt"],
        pendingApprovalId: null,
        verification: { verdict: "pass", label: "3/3 通过", time: "11:42:08" }
      },
      {
        id: "T-1035",
        title: "接入真实托管模型验收",
        goal: "用真实托管模型跑通 tool-use fixture，并记录失败诊断。",
        status: "blocked",
        statusNote: "等待 provider 凭据",
        model: "deepseek-reasoner",
        mode: "standard",
        updatedAt: "今天 10:18",
        createdAt: "10:14",
        constraints: ["不得使用伪造 API 结果", "失败必须保留 trace"],
        acceptance: [
          { id: "a1", label: "fixture 可加载", state: "pass" },
          { id: "a2", label: "托管模型返回 tool call", state: "fail" },
          { id: "a3", label: "失败 trace 已保留", state: "pending" }
        ],
        budget: { used: 0.09, cap: 1, tokens: 22400, calls: 3, wall: "2m 56s" },
        stats: { turns: 2, steps: 5, duration: "2m 56s", costSeries: [1, 1, 4, 2] },
        workingSet: ["ctx_b7c4", "ctx_3e18"],
        activeSkills: ["verify-receipt"],
        pendingApprovalId: null,
        verification: { verdict: "fail", label: "1/3 通过", time: "10:18:40" }
      }
    ],

    events: [
      {
        id: "e1042-1", taskId: "T-1042", seq: 1, turn: 1, kind: "SYSTEM", time: "12:03:08",
        title: "任务创建",
        text: "预算上限 ¥5.00 · 验收规则 3 条 · 审批策略 ask。"
      },
      {
        id: "e1042-2", taskId: "T-1042", seq: 2, turn: 1, kind: "USER", time: "12:03:10",
        title: "任务目标",
        text: "让 verify 在回放不一致时输出可定位的差异摘要，同时保持现有验收规则不变。"
      },
      {
        id: "e1042-3", taskId: "T-1042", seq: 3, turn: 1, kind: "CONTEXT", time: "12:03:11",
        title: "工作集已装载",
        text: "goal + 3 条固定约束 + 2 个上下文引用 + 1 个 Skill。",
        refs: ["ctx_8f21", "ctx_0c7d", "ctx_a1f0"]
      },
      {
        id: "e1042-4", taskId: "T-1042", seq: 4, turn: 1, kind: "ASSISTANT", time: "12:03:14",
        intent: "discover",
        title: "先确认可用能力",
        text: "先确认可用的上下文与核验能力，再决定是否读取源码；避免把全部 schema 塞进工作集。"
      },
      {
        id: "e1042-5", taskId: "T-1042", seq: 5, turn: 1, kind: "TOOL", time: "12:03:15",
        tool: "capabilities.discover",
        status: "ok",
        duration: "96ms",
        summary: "命中 4 个能力",
        args: "{\"query\":\"replay diagnostics verify\",\"limit\":6}",
        result: "4 matches\n- context.search (read)\n- context.read (read)\n- workspace.exec (high)\n- verify.run (read)"
      },
      {
        id: "e1042-6", taskId: "T-1042", seq: 6, turn: 2, kind: "ASSISTANT", time: "12:03:18",
        intent: "retrieve",
        title: "读取诊断契约与回放测试",
        text: "需要确认 Diagnostic 的现有字段，以及 replay 测试如何比较任务状态；只读取相关片段。"
      },
      {
        id: "e1042-7", taskId: "T-1042", seq: 7, turn: 2, kind: "TOOL", time: "12:03:19",
        tool: "context.read",
        status: "ok",
        duration: "112ms",
        summary: "verify/diagnostics.rs · 1,840 tokens",
        args: "{\"ref_id\":\"ctx_8f21\",\"max_tokens\":2200}",
        result: "Diagnostic { severity, code, detail, hint }\nseverity: info | warning | error\ncode: 稳定字符串，供机器比较"
      },
      {
        id: "e1042-8", taskId: "T-1042", seq: 8, turn: 2, kind: "TOOL", time: "12:03:20",
        tool: "context.read",
        status: "ok",
        duration: "88ms",
        summary: "tests/replay.rs · 1,120 tokens",
        args: "{\"ref_id\":\"ctx_0c7d\",\"range\":\"L1-L120\"}",
        result: "assert_eq!(replayed.status, task.status);\nassert_eq!(replayed.outputs, task.outputs);\n// 失败时仅打印 assertion，缺少可定位差异"
      },
      {
        id: "e1042-9", taskId: "T-1042", seq: 9, turn: 3, kind: "ASSISTANT", time: "12:03:26",
        intent: "execute",
        title: "请求运行测试",
        text: "需要在工作区运行 cargo test 复现差异；该操作属于高影响执行，按策略等待审批。"
      },
      {
        id: "e1042-10", taskId: "T-1042", seq: 10, turn: 3, kind: "APPROVAL", time: "12:03:27",
        approvalId: "ap_1042",
        title: "需要审批：workspace.exec",
        text: "批准后将执行 cargo test --workspace，并在受限工作区内记录完整回执。"
      },

      {
        id: "e1039-1", taskId: "T-1039", seq: 1, turn: 1, kind: "SYSTEM", time: "12:11:02",
        title: "任务创建",
        text: "只读分析 · 预算上限 ¥2.00 · 生产索引未被挂载为可写。"
      },
      {
        id: "e1039-2", taskId: "T-1039", seq: 2, turn: 1, kind: "USER", time: "12:11:04",
        title: "任务目标",
        text: "比较最近三次索引重建的召回结果，找出 FTS5 查询改写导致的漏召回。"
      },
      {
        id: "e1039-3", taskId: "T-1039", seq: 3, turn: 1, kind: "ASSISTANT", time: "12:11:09",
        intent: "retrieve",
        title: "搜索召回相关记录",
        text: "先检索 retrieval.rs 与最近一次测试输出，确认查询改写发生在哪一层。"
      },
      {
        id: "e1039-4", taskId: "T-1039", seq: 4, turn: 1, kind: "TOOL", time: "12:11:10",
        tool: "context.search",
        status: "ok",
        duration: "74ms",
        summary: "命中 5 条引用",
        args: "{\"query\":\"FTS5 query rewrite recall\",\"scope\":\"workspace\"}",
        result: "5 candidates\n- ctx_15f7 context/retrieval.rs (0.92)\n- ctx_6a02 tests/retrieval.rs (0.88)\n- ctx_71aa debug process (0.61)"
      },
      {
        id: "e1039-5", taskId: "T-1039", seq: 5, turn: 2, kind: "ASSISTANT", time: "12:11:16",
        intent: "retrieve",
        status: "running",
        title: "对齐漏召回样本",
        text: "已取得三组查询结果，正在按查询词与文档长度对齐漏召回样本；尚未写入任何索引。"
      },

      {
        id: "e1037-1", taskId: "T-1037", seq: 1, turn: 1, kind: "SYSTEM", time: "11:21:50",
        title: "任务创建",
        text: "来源：ADR-012 手稿与迁移决策记录 · 预算上限 ¥3.00。"
      },
      {
        id: "e1037-2", taskId: "T-1037", seq: 2, turn: 1, kind: "USER", time: "11:21:52",
        title: "任务目标",
        text: "把 Python → Rust 的决策、未决问题与证据整理成可检索的 ADR 记录。"
      },
      {
        id: "e1037-3", taskId: "T-1037", seq: 3, turn: 1, kind: "ASSISTANT", time: "11:22:01",
        intent: "retrieve",
        title: "读取原始决策记录",
        text: "读取 ADR-012 手稿与 contracts.rs 的迁移说明，保留日期、证据与仍未解决的问题。"
      },
      {
        id: "e1037-4", taskId: "T-1037", seq: 4, turn: 1, kind: "TOOL", time: "11:22:02",
        tool: "context.read",
        status: "ok",
        duration: "91ms",
        summary: "ADR-012 手稿 · 680 tokens",
        args: "{\"ref_id\":\"ctx_2b3e\",\"include_provenance\":true}",
        result: "Decision: Rust is the implementation language.\nOpen: delete or keep Python scaffold.\nEvidence: user instruction, 2026-11-16."
      },
      {
        id: "e1037-5", taskId: "T-1037", seq: 5, turn: 2, kind: "ASSISTANT", time: "11:22:10",
        intent: "respond",
        title: "整理 ADR 草稿",
        text: "将决策、上下文、后果与证据分节整理；迁移记录写入 docs/decisions/，不覆盖旧 ADR。"
      },
      {
        id: "e1037-6", taskId: "T-1037", seq: 6, turn: 2, kind: "TOOL", time: "11:40:32",
        tool: "filesystem.write",
        status: "ok",
        duration: "42ms",
        summary: "docs/decisions/ADR-012.md · 3,284 bytes",
        args: "{\"path\":\"docs/decisions/ADR-012.md\",\"mode\":\"create\"}",
        result: "Write committed.\nblob: sha256:5c1d…9a2e\nreceipt: 2026-11-16T11:40:32+08:00"
      },
      {
        id: "e1037-7", taskId: "T-1037", seq: 7, turn: 3, kind: "ASSISTANT", time: "11:42:01",
        intent: "verify",
        title: "运行验收检查",
        text: "逐条检查决策可追溯性、未决问题标注与旧 ADR 完整性。"
      },
      {
        id: "e1037-8", taskId: "T-1037", seq: 8, turn: 3, kind: "VERIFICATION", time: "11:42:08",
        verdict: "pass",
        title: "核验通过",
        text: "3/3 验收规则通过，任务可归档。",
        findings: [
          { label: "decision dates present", state: "pass" },
          { label: "open questions marked", state: "pass" },
          { label: "existing ADRs unchanged", state: "pass" }
        ]
      },

      {
        id: "e1035-1", taskId: "T-1035", seq: 1, turn: 1, kind: "SYSTEM", time: "10:14:32",
        title: "任务创建",
        text: "真实 provider 验收 · 失败必须保留 trace · 预算上限 ¥1.00。"
      },
      {
        id: "e1035-2", taskId: "T-1035", seq: 2, turn: 1, kind: "USER", time: "10:14:35",
        title: "任务目标",
        text: "用真实托管模型跑通 tool-use fixture，并记录失败诊断。"
      },
      {
        id: "e1035-3", taskId: "T-1035", seq: 3, turn: 1, kind: "ASSISTANT", time: "10:14:40",
        intent: "execute",
        title: "发起 provider 调用",
        text: "使用 OpenAI-compatible endpoint 发起一次带工具声明的请求；不伪造任何返回。"
      },
      {
        id: "e1035-4", taskId: "T-1035", seq: 4, turn: 1, kind: "TOOL", time: "10:14:42",
        tool: "provider.complete",
        status: "fail",
        duration: "2.4s",
        summary: "401 Unauthorized",
        args: "{\"model\":\"deepseek-reasoner\",\"tools\":[\"workspace.exec\"]}",
        result: "HTTP 401 Unauthorized\nmissing SFAI_MODEL_API_KEY\nrequest_id: req_demo_7f31"
      },
      {
        id: "e1035-5", taskId: "T-1035", seq: 5, turn: 2, kind: "VERIFICATION", time: "10:18:40",
        verdict: "fail",
        title: "核验未通过",
        text: "缺少 provider 凭据，无法证明真实 tool call 可被解析；任务停在安全边界。",
        findings: [
          { label: "fixture loaded", state: "pass" },
          { label: "provider returned tool call", state: "fail" },
          { label: "failure trace preserved", state: "pending" }
        ]
      }
    ],

    approvals: [
      {
        id: "ap_1042",
        taskId: "T-1042",
        status: "pending",
        action: "workspace.exec",
        risk: "high",
        title: "在受限工作区运行测试",
        summary: "执行 cargo test --workspace，以复现 replay 状态差异并保留完整回执。",
        command: "cargo test --workspace",
        scope: "/workspace/system-for-future-ai",
        reason: "验证 replay 状态与任务状态一致",
        requested: "12:03:27",
        expires: "12:33:27",
        decided: null,
        mode: null
      },
      {
        id: "ap_1037",
        taskId: "T-1037",
        status: "approved",
        action: "filesystem.write",
        risk: "write",
        title: "写入 ADR-012 迁移记录",
        summary: "把整理后的 ADR 写入 docs/decisions/，不覆盖既有决策文件。",
        command: "write docs/decisions/ADR-012.md",
        scope: "docs/decisions/",
        reason: "归档迁移决策与证据",
        requested: "11:40:28",
        expires: "12:10:28",
        decided: "11:40:30",
        mode: "once"
      }
    ],

    contextRefs: [
      {
        id: "ctx_8f21",
        title: "verify/diagnostics.rs",
        kind: "source",
        path: "crates/sfai/src/verify/diagnostics.rs",
        revision: "r4",
        tokens: 1840,
        updated: "12:01",
        summary: "差异诊断的结构、严重级别与稳定错误码。",
        excerpt: "pub struct Diagnostic {\n    pub severity: Severity,\n    pub code: String,\n    pub detail: String,\n    pub hint: Option<String>,\n}\n\nseverity: info | warning | error\ncode: 稳定字符串，供机器比较与回归测试。",
        provenance: "工作区文件 · 12:01:44 · hash 8f21c9"
      },
      {
        id: "ctx_0c7d",
        title: "tests/replay.rs",
        kind: "source",
        path: "crates/sfai/tests/replay.rs",
        revision: "r2",
        tokens: 1120,
        updated: "11:58",
        summary: "回放状态与任务状态的现有断言。",
        excerpt: "let replayed = store.replay(task_id)?;\nassert_eq!(replayed.status, task.status);\nassert_eq!(replayed.outputs, task.outputs);\n\n// 失败时仅输出 assertion，缺少可定位的差异摘要。",
        provenance: "工作区文件 · 11:58:02 · hash 0c7dd1"
      },
      {
        id: "ctx_71aa",
        title: "development-debug-process.md",
        kind: "doc",
        path: "docs/development-debug-process.md",
        revision: "r6",
        tokens: 960,
        updated: "10:47",
        summary: "每个开发增量的强制调试阶段。",
        excerpt: "复现 → 隔离 → 根因 → 修复 → 回归 → 记录。\n\n任何完成的开发增量都必须通过强制调试阶段；失败必须留下可复现证据，而不是只在对话中解释。",
        provenance: "工作区文件 · 10:47:19 · hash 71aa4b"
      },
      {
        id: "ctx_44c9",
        title: "contracts.rs",
        kind: "source",
        path: "crates/sfai/src/contracts.rs",
        revision: "r7",
        tokens: 2210,
        updated: "09:31",
        summary: "Task、TaskState、Event 的持久化字段。",
        excerpt: "pub struct Task {\n    pub task_id: TaskId,\n    pub goal: String,\n    pub constraints: Vec<String>,\n    pub acceptance: Vec<AcceptanceRule>,\n    pub status: TaskStatus,\n    pub budget: Budget,\n}\n\npub struct TaskState {\n    pub active_intent: IntentKind,\n    pub completed_outputs: Vec<ArtifactId>,\n    pub unresolved_errors: Vec<Diagnostic>,\n}",
        provenance: "工作区文件 · 09:31:08 · hash 44c9f0"
      },
      {
        id: "ctx_2b3e",
        title: "ADR-012 语言迁移",
        kind: "decision",
        path: "docs/decisions/ADR-012.md",
        revision: "r1",
        tokens: 680,
        updated: "昨天",
        summary: "Python 基准切换为 Rust 的决策与证据。",
        excerpt: "Decision: the implementation language for sfai is Rust.\nSupersedes: the Python baseline in the implementation plan.\nOpen question: delete or keep the Python scaffold as a semantics oracle.\nEvidence: user instruction, 2026-11-16.",
        provenance: "docs/decisions · 2026-11-15 18:22 · hash 2b3e77"
      },
      {
        id: "ctx_9d10",
        title: "Wiki 编译报告 · T-1037",
        kind: "derived",
        path: ".sfai/wiki/reports/T-1037.json",
        revision: "r3",
        tokens: 540,
        updated: "11:43",
        summary: "派生 Wiki 的材料变更与陈旧引用报告。",
        excerpt: "{\n  \"material_change\": false,\n  \"stale_refs\": 0,\n  \"contradiction_edges\": 0,\n  \"write_amplification\": 0.18\n}\n\nWiki 永远是派生表示，不是事实来源。",
        provenance: "派生索引 · 11:43:07 · 由事件账本重算"
      },
      {
        id: "ctx_15f7",
        title: "context/retrieval.rs",
        kind: "source",
        path: "crates/sfai/src/context/retrieval.rs",
        revision: "r5",
        tokens: 1580,
        updated: "12:08",
        summary: "FTS5 查询构造、候选排序与引用范围。",
        excerpt: "pub fn search(&self, query: &str, limit: usize) -> Vec<SearchCandidate> {\n    let fts_query = rewrite_for_fts5(query);\n    let rows = self.fts.query(&fts_query, limit)?;\n    rows.into_iter().map(SearchCandidate::from_row).collect()\n}\n\n// rewrite_for_fts5 会把 CJK 短语按字符切分。",
        provenance: "工作区文件 · 12:08:22 · hash 15f7aa"
      },
      {
        id: "ctx_6a02",
        title: "tests/retrieval.rs",
        kind: "source",
        path: "crates/sfai/tests/retrieval.rs",
        revision: "r2",
        tokens: 840,
        updated: "12:10",
        summary: "检索召回与增量索引测试。",
        excerpt: "#[test]\nfn cjk_phrase_still_matches() {\n    let hits = index.search(\"回放 诊断\", 10);\n    assert!(!hits.is_empty());\n}\n\n// 当前 fixture 只覆盖单次重建，没有三次重建的对照。",
        provenance: "工作区文件 · 12:10:05 · hash 6a02b8"
      },
      {
        id: "ctx_b7c4",
        title: "implementation-plan §12",
        kind: "doc",
        path: "docs/implementation-plan.md",
        revision: "r9",
        tokens: 1320,
        updated: "昨天",
        summary: "验收场景 A–L 与真实 provider 验收要求。",
        excerpt: "§12 Acceptance scenarios A–L\n\n- A direct answer, zero discovery calls\n- B tool use, idempotent replay\n- C failed tool with visible diagnostics\n- D approval-gated external write\n- L real hosted model tool call",
        provenance: "工作区文件 · 2026-11-15 17:40 · hash b7c4e2"
      },
      {
        id: "ctx_3e18",
        title: "telemetry/replay.rs",
        kind: "source",
        path: "crates/sfai/src/telemetry/replay.rs",
        revision: "r3",
        tokens: 1170,
        updated: "10:12",
        summary: "trace 重放与成本重算。",
        excerpt: "pub fn replay_trace(path: &Path) -> Result<ReplayReport> {\n    let trace = Trace::read_jsonl(path)?;\n    let totals = totals_from_events(&trace.events);\n    Ok(ReplayReport { events: trace.events.len(), totals })\n}",
        provenance: "工作区文件 · 10:12:51 · hash 3e18cd"
      },
      {
        id: "ctx_a1f0",
        title: "skills/debug-session/SKILL.md",
        kind: "skill",
        path: "skills/research/debug-session/SKILL.md",
        revision: "r2",
        tokens: 420,
        updated: "昨天",
        summary: "调试阶段的步骤、证据与记录要求。",
        excerpt: "name: debug-session\nversion: 1.4.0\n\nReproduce → isolate → root-cause → fix → regression → record.\n\nNever declare a fix without a regression run and a written record.",
        provenance: "Skill 包 · 元数据发现，不读取正文直到激活 · hash a1f0de"
      },
      {
        id: "ctx_5c09",
        title: "verify/engine.rs",
        kind: "source",
        path: "crates/sfai/src/verify/engine.rs",
        revision: "r6",
        tokens: 1420,
        updated: "12:02",
        summary: "验收规则执行与核验结果生成。",
        excerpt: "pub fn run(&self, refs: &[ContextRef], rules: &[AcceptanceRule]) -> VerificationResult {\n    let findings = rules.iter().map(|rule| self.check(rule, refs)).collect();\n    VerificationResult::from_findings(findings)\n}",
        provenance: "工作区文件 · 12:02:13 · hash 5c09af"
      }
    ],

    capabilities: [
      {
        id: "context.search",
        name: "上下文检索",
        risk: "read",
        version: "0.5.0",
        description: "在任务索引中执行 FTS5 检索，返回候选引用、命中分数与范围。检索结果是提示，不是事实声明。",
        safety: "只读 · 不产生外部副作用",
        operations: [
          { name: "search", description: "按查询返回候选引用" }
        ],
        schema: "{\n  \"query\": \"string\",\n  \"scope\": \"task | workspace\",\n  \"limit\": 10\n}"
      },
      {
        id: "context.read",
        name: "受限上下文读取",
        risk: "read",
        version: "0.5.0",
        description: "按引用读取有界摘录，保留稳定引用与回到原始来源的路径。不会把整份文档塞进工作集。",
        safety: "只读 · 返回带 provenance 的有界片段",
        operations: [
          { name: "read", description: "按 ref_id 读取范围" },
          { name: "resolve", description: "解析引用与 provenance" }
        ],
        schema: "{\n  \"ref_id\": \"ctx_…\",\n  \"range\": \"L1-L120\",\n  \"max_tokens\": 2200\n}"
      },
      {
        id: "workspace.exec",
        name: "受限工作区执行",
        risk: "high",
        version: "0.5.0",
        description: "在受限工作区运行测试与命令。高影响执行必须经过审批，并写入工具回执与事件账本。",
        safety: "高影响 · 默认每次询问 · 需要完整回执",
        operations: [
          { name: "run", description: "运行受限命令" }
        ],
        schema: "{\n  \"command\": \"cargo test --workspace\",\n  \"cwd\": \"/workspace/system-for-future-ai\",\n  \"timeout_ms\": 120000\n}"
      },
      {
        id: "verify.run",
        name: "独立核验",
        risk: "read",
        version: "0.5.0",
        description: "执行确定性验收规则，产出机器可读的 findings、verdict 与诊断信息。核验独立于路由。",
        safety: "只读 · 失败会成为开放问题",
        operations: [
          { name: "run", description: "执行验收规则" }
        ],
        schema: "{\n  \"target_refs\": [\"ctx_…\"],\n  \"criteria\": [\"cargo test 全绿\"],\n  \"mode\": \"deterministic\"\n}"
      },
      {
        id: "wiki.compile",
        name: "增量 Wiki 编译",
        risk: "write",
        version: "0.4.0",
        description: "从事件账本增量派生 Wiki 节点、边与陈旧引用报告。Wiki 永远不是事实来源。",
        safety: "写入派生层 · 失败不推进游标",
        operations: [
          { name: "compile", description: "编译任务派生知识" }
        ],
        schema: "{\n  \"task_id\": \"T-1042\",\n  \"mode\": \"incremental\"\n}"
      },
      {
        id: "filesystem.write",
        name: "文件写入",
        risk: "write",
        version: "0.5.0",
        description: "在授权路径内创建或更新文件，返回内容哈希与写入回执。路径和覆盖策略在执行时再次校验。",
        safety: "写入 · 受策略约束 · 不覆盖未授权路径",
        operations: [
          { name: "write", description: "创建或更新文件" }
        ],
        schema: "{\n  \"path\": \"docs/decisions/ADR-012.md\",\n  \"mode\": \"create | replace\"\n}"
      },
      {
        id: "web.fetch",
        name: "外部读取",
        risk: "external",
        version: "0.3.1",
        description: "读取外部页面或 API 并以不可信数据处理。检索文本永远不能授予权限。",
        safety: "外部 · 数据不可信 · 不携带凭据",
        operations: [
          { name: "get", description: "读取外部 URL" }
        ],
        schema: "{\n  \"url\": \"https://…\",\n  \"max_bytes\": 262144\n}"
      }
    ],

    skills: [
      {
        id: "debug-session",
        name: "debug-session",
        version: "1.4.0",
        trust: "local",
        description: "把调试拆成复现、隔离、根因、修复、回归与记录六个阶段；没有回归与记录就不算完成。",
        refs: ["docs/development-debug-process.md", "skills/research/debug-session/SKILL.md"]
      },
      {
        id: "verify-receipt",
        name: "verify-receipt",
        version: "0.9.2",
        trust: "reviewed",
        description: "为高影响操作生成可核验的回执：目标、证据、findings 与 verdict 必须来自同一份事件账本。",
        refs: ["crates/sfai/src/verify/engine.rs"]
      },
      {
        id: "wiki-compiler",
        name: "wiki-compiler",
        version: "1.1.0",
        trust: "local",
        description: "增量编译派生 Wiki，标注材料变更、矛盾边与陈旧引用；只写派生层，不改任务权威状态。",
        refs: ["crates/sfai/src/context/wiki.rs"]
      },
      {
        id: "context-audit",
        name: "context-audit",
        version: "0.6.0",
        trust: "draft",
        description: "审计工作集是否缺少固定约束、是否引入了过期引用，并输出可复核的检索回执。",
        refs: ["crates/sfai/src/context/working_set.rs"]
      }
    ]
  };

  SFAI.seed = function () {
    return JSON.parse(JSON.stringify(seed));
  };

  SFAI.seedMeta = seed.meta;
})();
