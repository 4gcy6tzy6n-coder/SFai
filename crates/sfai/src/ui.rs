//! Claude Code-style CLI UI design system and terminal renderer for `sfai`.
//!
//! Provides:
//! - ANSI / plain theme with automatic `NO_COLOR` & `--no-color` support;
//! - Unicode-aware box drawing (`╭─╮│├┤╰─╯`) and tree connectors (`●`, `⎿`);
//! - `ClaudeStepObserver`: real-time streaming timeline of self-routing kernel
//!   decisions, working-set receipts, tool executions, and verification gates;
//! - `CliApprovalProvider`: interactive Claude Code permission prompt box for
//!   `external_write` and `high_impact` operations (`deny` / `ask` / `auto`);
//! - Rich inspector cards for tasks, status, event ledger, context search/read,
//!   derived Wiki graph, progressive-disclosure Skills, capabilities, verify,
//!   trace replay, cost dashboard, and system doctor.

use crate::app::App;
use crate::authority::approvals::{ApprovalDecision, ApprovalProvider};
use crate::capabilities::skills::{SkillActivation, SkillManifest};
use crate::context::retrieval::{ReadPayload, SearchCandidate};
use crate::contracts::{
    ActionIntent, CapabilityManifest, ContextEnvelope, Event, IntentKind, ModelResponse, RiskClass,
    Task, TaskState, TaskStatus, ToolResult, ToolStatus, Verdict, VerificationResult,
};
use crate::core::decision_kernel::{KernelObserver, TaskRunResult};
use crate::fixtures::{get_fixture, FIXTURE_NAMES};
use crate::telemetry::replay::ReplayReport;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::sync::Mutex;

// ---- Theme & Styling ----

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub color: bool,
}

impl Theme {
    pub fn auto(no_color_flag: bool) -> Self {
        let env_no_color = std::env::var_os("NO_COLOR").is_some();
        let dumb_term = std::env::var("TERM")
            .map(|t| t.eq_ignore_ascii_case("dumb"))
            .unwrap_or(false);
        Self {
            color: !no_color_flag && !env_no_color && !dumb_term,
        }
    }

    pub fn colored() -> Self {
        Self { color: true }
    }

    pub fn plain() -> Self {
        Self { color: false }
    }

    fn wrap(&self, code: &str, text: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    /// Claude Code signature warm terracotta/amber accent.
    pub fn accent(&self, text: &str) -> String {
        self.wrap("38;5;209;1", text)
    }

    pub fn bold(&self, text: &str) -> String {
        self.wrap("1", text)
    }

    pub fn dim(&self, text: &str) -> String {
        self.wrap("2", text)
    }

    pub fn cyan(&self, text: &str) -> String {
        self.wrap("36", text)
    }

    pub fn green(&self, text: &str) -> String {
        self.wrap("32", text)
    }

    pub fn green_bold(&self, text: &str) -> String {
        self.wrap("32;1", text)
    }

    pub fn yellow(&self, text: &str) -> String {
        self.wrap("33", text)
    }

    pub fn yellow_bold(&self, text: &str) -> String {
        self.wrap("33;1", text)
    }

    pub fn red(&self, text: &str) -> String {
        self.wrap("31", text)
    }

    pub fn red_bold(&self, text: &str) -> String {
        self.wrap("31;1", text)
    }

    pub fn magenta(&self, text: &str) -> String {
        self.wrap("35", text)
    }

    pub fn status_badge(&self, status: TaskStatus) -> String {
        match status {
            TaskStatus::Completed => self.green_bold("✓ completed"),
            TaskStatus::Active => self.cyan("● active"),
            TaskStatus::WaitingApproval => self.yellow_bold("◆ waiting_approval"),
            TaskStatus::Paused => self.yellow("◆ paused"),
            TaskStatus::Blocked => self.red_bold("✗ blocked"),
            TaskStatus::Failed => self.red_bold("✗ failed"),
            TaskStatus::Cancelled => self.dim("⊘ cancelled"),
            TaskStatus::Created => self.dim("○ created"),
        }
    }

    pub fn risk_badge(&self, risk: RiskClass) -> String {
        match risk {
            RiskClass::ReadOnly => self.green("read_only"),
            RiskClass::ReversibleWrite => self.cyan("reversible_write"),
            RiskClass::ExternalWrite => self.yellow_bold("external_write"),
            RiskClass::HighImpact => self.red_bold("high_impact"),
        }
    }
}

// ---- Unicode Display Width & Box Drawing ----

/// Visual column width of a string, ignoring ANSI escape sequences and
/// counting East Asian Wide / Fullwidth characters as 2 columns.
pub fn display_width(s: &str) -> usize {
    let mut width = 0usize;
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
            continue;
        }
        width += char_width(ch);
    }
    width
}

fn char_width(ch: char) -> usize {
    let u = ch as u32;
    if u < 0x20 || (0x7f..0xa0).contains(&u) {
        return 0;
    }
    // CJK Unified Ideographs, Hangul, Fullwidth forms, Kana, CJK Symbols
    if (0x1100..=0x115f).contains(&u)
        || (0x2e80..=0x303e).contains(&u)
        || (0x3040..=0x33ff).contains(&u)
        || (0x3400..=0x4dbf).contains(&u)
        || (0x4e00..=0x9fff).contains(&u)
        || (0xa960..=0xa97f).contains(&u)
        || (0xac00..=0xd7ff).contains(&u)
        || (0xf900..=0xfaff).contains(&u)
        || (0xfe10..=0xfe6f).contains(&u)
        || (0xff01..=0xff60).contains(&u)
        || (0xffe0..=0xffe6).contains(&u)
        || (0x1f300..=0x1faff).contains(&u)
        || (0x20000..=0x2fa1f).contains(&u)
    {
        2
    } else {
        1
    }
}

pub fn pad_right_ansi(s: &str, target_width: usize) -> String {
    let w = display_width(s);
    if w >= target_width {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(target_width - w))
    }
}

pub fn truncate_plain(s: &str, max_chars: usize) -> String {
    let clean = s.replace('\n', " ");
    let count = clean.chars().count();
    if count <= max_chars {
        clean
    } else {
        let taken: String = clean.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{taken}…")
    }
}

/// Render a rounded Claude Code-style box (`╭─ title ───╮ ... ╰───────────╯`).
pub fn render_box(title: &str, lines: &[String], min_inner_width: usize, theme: &Theme) -> String {
    let title_plain_len = if title.is_empty() {
        0
    } else {
        display_width(title) + 4
    };
    let max_line = lines.iter().map(|l| display_width(l)).max().unwrap_or(0);
    let inner_width = min_inner_width.max(max_line).max(title_plain_len);

    let mut out = String::new();
    if title.is_empty() {
        let top = format!("╭{}╮", "─".repeat(inner_width + 2));
        out.push_str(&theme.dim(&top));
        out.push('\n');
    } else {
        let title_w = display_width(title);
        let dashes = (inner_width + 2).saturating_sub(title_w + 3);
        out.push_str(&format!(
            "{} {} {}\n",
            theme.dim("╭─"),
            title,
            theme.dim(&format!("{}╮", "─".repeat(dashes)))
        ));
    }

    for line in lines {
        let padded = pad_right_ansi(line, inner_width);
        out.push_str(&format!(
            "{} {} {}\n",
            theme.dim("│"),
            padded,
            theme.dim("│")
        ));
    }

    let bottom = format!("╰{}╯", "─".repeat(inner_width + 2));
    out.push_str(&theme.dim(&bottom));
    out
}

// ---- Permission / Approval Mode & Interactive Prompt ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionMode {
    Deny,
    Ask,
    Auto,
}

impl PermissionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionMode::Deny => "deny",
            PermissionMode::Ask => "ask",
            PermissionMode::Auto => "auto",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "deny" | "deny-all" | "strict" => Some(PermissionMode::Deny),
            "ask" | "prompt" | "interactive" => Some(PermissionMode::Ask),
            "auto" | "allow" | "auto-approve" => Some(PermissionMode::Auto),
            _ => None,
        }
    }

    pub fn badge(&self, theme: &Theme) -> String {
        match self {
            PermissionMode::Deny => theme.red("⊘ deny-all"),
            PermissionMode::Ask => theme.yellow_bold("◆ ask-before-write"),
            PermissionMode::Auto => theme.green_bold("▸ auto-approve"),
        }
    }
}

pub struct CliApprovalProvider {
    mode: Mutex<PermissionMode>,
    theme: Theme,
}

impl CliApprovalProvider {
    pub fn new(mode: PermissionMode, theme: Theme) -> Self {
        Self {
            mode: Mutex::new(mode),
            theme,
        }
    }

    pub fn mode(&self) -> PermissionMode {
        *self.mode.lock().expect("permissions mutex poisoned")
    }

    pub fn set_mode(&self, mode: PermissionMode) {
        *self.mode.lock().expect("permissions mutex poisoned") = mode;
    }
}

pub fn render_permission_box(
    task_id: &str,
    action_summary: &str,
    risk_class: RiskClass,
    theme: &Theme,
) -> String {
    let title = theme.yellow_bold("⚠ Permission Required");
    let lines = vec![
        format!(
            "{}  {}",
            theme.dim("Action   :"),
            theme.bold(action_summary)
        ),
        format!(
            "{}  {}",
            theme.dim("Risk Tier:"),
            theme.risk_badge(risk_class)
        ),
        format!("{}  {}", theme.dim("Task ID  :"), theme.cyan(task_id)),
        String::new(),
        format!(
            "Allow this consequential action? {} / {} / {}",
            theme.green_bold("[y]es"),
            theme.red_bold("[n]o"),
            theme.cyan("[a]lways for session")
        ),
    ];
    render_box(&title, &lines, 64, theme)
}

impl ApprovalProvider for CliApprovalProvider {
    fn request(
        &self,
        task_id: &str,
        action_summary: &str,
        risk_class: RiskClass,
    ) -> ApprovalDecision {
        match *self.mode.lock().expect("permissions mutex poisoned") {
            PermissionMode::Deny => ApprovalDecision {
                approved: false,
                decided_by: "policy:deny_all".into(),
                receipt: None,
            },
            PermissionMode::Auto => ApprovalDecision {
                approved: true,
                decided_by: "policy:auto_approve".into(),
                receipt: Some(json!({
                    "action": action_summary,
                    "risk_class": risk_class.as_str(),
                    "mode": "auto"
                })),
            },
            PermissionMode::Ask => {
                println!(
                    "\n{}",
                    render_permission_box(task_id, action_summary, risk_class, &self.theme)
                );
                print!("  {} ", self.theme.accent("Allow? [y/n/a] ❯"));
                let _ = std::io::stdout().flush();
                let mut line = String::new();
                let read = std::io::stdin().lock().read_line(&mut line).unwrap_or(0);
                if read == 0 {
                    println!("{}", self.theme.red("  ⎿  ⊘ Denied (EOF on stdin)"));
                    return ApprovalDecision {
                        approved: false,
                        decided_by: "user:cli_eof_denied".into(),
                        receipt: None,
                    };
                }
                let choice = line.trim().to_ascii_lowercase();
                if matches!(choice.as_str(), "a" | "always") {
                    *self.mode.lock().expect("permissions mutex poisoned") = PermissionMode::Auto;
                    println!(
                        "  {} {}",
                        self.theme.dim("⎿"),
                        self.theme
                            .green("✓ Approved by user (switched session to auto-approve)")
                    );
                    ApprovalDecision {
                        approved: true,
                        decided_by: "user:cli_always".into(),
                        receipt: Some(json!({
                            "action": action_summary,
                            "risk_class": risk_class.as_str(),
                            "mode": "always"
                        })),
                    }
                } else if matches!(choice.as_str(), "y" | "yes") {
                    println!(
                        "  {} {}",
                        self.theme.dim("⎿"),
                        self.theme.green("✓ Approved by user")
                    );
                    ApprovalDecision {
                        approved: true,
                        decided_by: "user:cli_interactive".into(),
                        receipt: Some(json!({
                            "action": action_summary,
                            "risk_class": risk_class.as_str(),
                            "mode": "interactive"
                        })),
                    }
                } else {
                    println!(
                        "  {} {}",
                        self.theme.dim("⎿"),
                        self.theme.red("⊘ Denied by user")
                    );
                    ApprovalDecision {
                        approved: false,
                        decided_by: "user:cli_denied".into(),
                        receipt: None,
                    }
                }
            }
        }
    }
}

// ---- Welcome Header & Prompt ----

pub struct BannerInfo<'a> {
    pub storage: &'a str,
    pub model_label: &'a str,
    pub capabilities_count: usize,
    pub skills_count: usize,
    pub tasks_count: usize,
    pub permission_mode: PermissionMode,
}

pub fn detect_model_label() -> String {
    let model = std::env::var("SFAI_MODEL_NAME").unwrap_or_default();
    let url = std::env::var("SFAI_MODEL_BASE_URL").unwrap_or_default();
    if !model.is_empty() && !url.is_empty() {
        format!("{model} ({url})")
    } else if !model.is_empty() {
        model
    } else {
        "offline-ready (fixtures + OpenAI-compat via SFAI_MODEL_*)".to_string()
    }
}

pub fn render_welcome_banner(info: &BannerInfo<'_>, theme: &Theme) -> String {
    let title = format!(
        "{} {}",
        theme.accent("✻ System for future ai (sfai)"),
        theme.dim("v0.1.0 · interactive session")
    );
    let lines = vec![
        format!(
            "{}  {}",
            theme.dim("Architecture :"),
            "Model-native self-routing · Index-native context · Independent verify"
        ),
        format!(
            "{}  {}",
            theme.dim("Model        :"),
            theme.cyan(info.model_label)
        ),
        format!(
            "{}  {} {}",
            theme.dim("Workspace    :"),
            theme.bold(info.storage),
            theme.dim(&format!(
                "({} capabilities · {} skills · {} tasks)",
                info.capabilities_count, info.skills_count, info.tasks_count
            ))
        ),
        format!(
            "{}  {}",
            theme.dim("Permissions  :"),
            info.permission_mode.badge(theme)
        ),
        String::new(),
        format!(
            "{} {} · {} · {} · {} · {} · {}",
            theme.dim("Quick start  :"),
            theme.cyan("/run tool-use"),
            theme.cyan("/fixtures"),
            theme.cyan("/tasks"),
            theme.cyan("/wiki"),
            theme.cyan("/permissions"),
            theme.cyan("/help")
        ),
    ];
    render_box(&title, &lines, 74, theme)
}

pub fn render_prompt(
    active_task: Option<&str>,
    perm_mode: PermissionMode,
    constraints_count: usize,
    theme: &Theme,
) -> String {
    let mut badges = Vec::new();
    if let Some(tid) = active_task {
        let short = if tid.len() > 14 { &tid[..14] } else { tid };
        badges.push(theme.cyan(short));
    }
    if perm_mode != PermissionMode::Deny {
        badges.push(match perm_mode {
            PermissionMode::Ask => theme.yellow("ask"),
            PermissionMode::Auto => theme.green("auto"),
            PermissionMode::Deny => String::new(),
        });
    }
    if constraints_count > 0 {
        badges.push(theme.magenta(&format!("{} pinned", constraints_count)));
    }
    let prefix = if badges.is_empty() {
        String::new()
    } else {
        format!("[{}] ", badges.join("·"))
    };
    // Keep `sfai> ` token for script friendliness while styling it cleanly.
    format!("{}{}", prefix, theme.accent("sfai> "))
}

// ---- Claude Code Live Step Observer ----

pub struct ClaudeStepObserver<W: Write> {
    out: W,
    theme: Theme,
}

impl<W: Write> ClaudeStepObserver<W> {
    pub fn new(out: W, theme: Theme) -> Self {
        Self { out, theme }
    }

    pub fn into_inner(self) -> W {
        self.out
    }
}

pub fn format_intent_badge(intent: &ActionIntent, theme: &Theme) -> String {
    match intent.kind {
        IntentKind::Discover => {
            let q = intent
                .payload
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("{}({:?})", theme.bold("Discover"), truncate_plain(q, 48))
        }
        IntentKind::Activate => {
            if let Some(skill) = intent.payload.get("skill_id").and_then(|v| v.as_str()) {
                format!("{}({})", theme.magenta("Skill"), theme.bold(skill))
            } else {
                let cap = intent
                    .payload
                    .get("capability_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                format!("{}({})", theme.magenta("Activate"), theme.bold(cap))
            }
        }
        IntentKind::Retrieve => {
            let op = intent
                .payload
                .get("op")
                .and_then(|v| v.as_str())
                .unwrap_or("search");
            match op {
                "search" => {
                    let q = intent
                        .payload
                        .get("query")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    format!(
                        "{}(query: {:?})",
                        theme.cyan("ContextSearch"),
                        truncate_plain(q, 52)
                    )
                }
                "read" => {
                    let r = intent
                        .payload
                        .get("ref_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let max_tok = intent
                        .payload
                        .get("max_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(600);
                    format!(
                        "{}({}, max_tokens: {})",
                        theme.cyan("ContextRead"),
                        theme.bold(r),
                        max_tok
                    )
                }
                "resolve" => {
                    let count = intent
                        .payload
                        .get("ref_ids")
                        .and_then(|v| v.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0);
                    format!("{}({} ref(s))", theme.cyan("ContextResolve"), count)
                }
                other => format!("{}({})", theme.cyan("Retrieve"), other),
            }
        }
        IntentKind::Execute => {
            let op = intent
                .payload
                .get("operation_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let args = intent
                .payload
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let arg_summary = summarize_tool_args(op, &args);
            format!("{}({})", theme.bold(&friendly_tool_name(op)), arg_summary)
        }
        IntentKind::Verify => {
            let criteria: Vec<String> = intent
                .payload
                .get("criteria")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let summary = if criteria.is_empty() {
                "acceptance_rules".to_string()
            } else {
                criteria.join(", ")
            };
            format!(
                "{}({})",
                theme.yellow_bold("Verify"),
                truncate_plain(&summary, 56)
            )
        }
        IntentKind::Respond => theme.green_bold("Respond"),
        IntentKind::Finish => theme.green_bold("Finish"),
        IntentKind::AskUser => {
            let q = intent
                .payload
                .get("question")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!(
                "{}({:?})",
                theme.yellow_bold("AskUser"),
                truncate_plain(q, 52)
            )
        }
        IntentKind::Pause => theme.yellow("Pause"),
    }
}

fn friendly_tool_name(op: &str) -> String {
    match op {
        "web.search" => "WebSearch".into(),
        "file.write" => "Write".into(),
        "file.read" => "Read".into(),
        "mail.send" => "SendMail".into(),
        other => format!("Tool:{other}"),
    }
}

fn summarize_tool_args(op: &str, args: &Value) -> String {
    match op {
        "web.search" => args
            .get("query")
            .and_then(|q| q.as_str())
            .map(|q| format!("{:?}", truncate_plain(q, 48)))
            .unwrap_or_else(|| compact_json(args, 48)),
        "file.write" => {
            let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("?");
            let bytes = args
                .get("content")
                .and_then(|c| c.as_str())
                .map(|c| c.len())
                .unwrap_or(0);
            format!("{path}, {bytes} bytes")
        }
        "file.read" => args
            .get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("?")
            .to_string(),
        _ => compact_json(args, 52),
    }
}

fn compact_json(v: &Value, max_chars: usize) -> String {
    let s = serde_json::to_string(v).unwrap_or_else(|_| "{}".into());
    truncate_plain(&s, max_chars)
}

impl<W: Write> KernelObserver for ClaudeStepObserver<W> {
    fn on_run_start(&mut self, task: &Task, _state: &TaskState) {
        let mut lines = vec![
            format!(
                "{}  {}",
                self.theme.dim("Task ID :"),
                self.theme.cyan(&task.task_id)
            ),
            format!(
                "{}  {}",
                self.theme.dim("Goal    :"),
                self.theme.bold(&truncate_plain(&task.goal, 64))
            ),
        ];
        if !task.constraints.is_empty() {
            lines.push(format!(
                "{}  {}",
                self.theme.dim("Pinned  :"),
                self.theme
                    .magenta(&truncate_plain(&task.constraints.join(" · "), 64))
            ));
        }
        if !task.acceptance_rules.is_empty() {
            lines.push(format!(
                "{}  {}",
                self.theme.dim("Accept  :"),
                self.theme
                    .yellow(&truncate_plain(&task.acceptance_rules.join(", "), 64))
            ));
        }
        let header = render_box(
            &self.theme.accent("✻ Task Execution"),
            &lines,
            68,
            &self.theme,
        );
        let _ = writeln!(self.out, "\n{header}");
        let _ = self.out.flush();
    }

    fn on_step_start(&mut self, step: usize, envelope: &ContextEnvelope) {
        let est_tokens = envelope
            .retrieval_receipt
            .as_ref()
            .map(|r| r.estimated_tokens)
            .unwrap_or(0);
        let skills_str = if envelope.active_skills.is_empty() {
            String::new()
        } else {
            let names: Vec<&str> = envelope
                .active_skills
                .iter()
                .map(|s| s.name.as_str())
                .collect();
            format!(" · skills: [{}]", names.join(", "))
        };
        let _ = writeln!(
            self.out,
            "  {}",
            self.theme.dim(&format!(
                "◌ Step {step} · working-set: {} active ref(s) (~{est_tokens} tok) · {} pinned constraint(s){skills_str}",
                envelope.active_refs.len(),
                envelope.pinned_constraints.len()
            ))
        );
        let _ = self.out.flush();
    }

    fn on_decision(&mut self, _step: usize, response: &ModelResponse) {
        let badge = format_intent_badge(&response.intent, &self.theme);
        let meta = self.theme.dim(&format!(
            "[reason: {} · model: {} · {}→{} tok]",
            response.intent.reason_code,
            response.model_id,
            response.usage.input_tokens,
            response.usage.output_tokens
        ));
        let _ = writeln!(self.out, "  {} {} {}", self.theme.accent("●"), badge, meta);
        let _ = self.out.flush();
    }

    fn on_tool_result(&mut self, _step: usize, _intent: &ActionIntent, result: &ToolResult) {
        let status_icon = match result.status {
            ToolStatus::Ok => {
                if result.replayed {
                    self.theme.cyan("↺ replayed")
                } else {
                    self.theme.green("✓")
                }
            }
            ToolStatus::Denied => self.theme.red_bold("⊘ denied"),
            ToolStatus::Error => self.theme.red_bold("✗ error"),
        };
        let preview = truncate_plain(&result.result_preview, 76);
        let _ = writeln!(
            self.out,
            "    {}  {} {}",
            self.theme.dim("⎿"),
            status_icon,
            preview
        );
        if !result.result_refs.is_empty() {
            let refs_colored: Vec<String> = result
                .result_refs
                .iter()
                .take(4)
                .map(|r| self.theme.cyan(r))
                .collect();
            let extra = if result.result_refs.len() > 4 {
                format!(" (+{} more)", result.result_refs.len() - 4)
            } else {
                String::new()
            };
            let _ = writeln!(
                self.out,
                "       {} {}{}",
                self.theme.dim("refs:"),
                refs_colored.join(", "),
                self.theme.dim(&extra)
            );
        }
        if !result.side_effects.is_empty() {
            let _ = writeln!(
                self.out,
                "       {} {}",
                self.theme.dim("side-effects:"),
                self.theme.yellow(&result.side_effects.join(", "))
            );
        }
        if let Some(diag) = &result.diagnostic {
            let code = diag
                .get("error_code")
                .and_then(|c| c.as_str())
                .unwrap_or("DIAGNOSTIC");
            let hint = diag
                .get("repair_hint")
                .and_then(|h| h.as_str())
                .unwrap_or("");
            if !hint.is_empty() {
                let _ = writeln!(
                    self.out,
                    "       {} {} — {}",
                    self.theme.yellow("↳ hint:"),
                    self.theme.red(code),
                    hint
                );
            }
        }
        let _ = self.out.flush();
    }

    fn on_acceptance_blocked(
        &mut self,
        _step: usize,
        issues: &[Value],
        retry: u32,
        max_retries: u32,
    ) {
        let summary = issues
            .last()
            .map(|i| {
                let crit = i
                    .get("criterion")
                    .or_else(|| i.get("error_code"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("acceptance");
                let hint = i
                    .get("repair_hint")
                    .or_else(|| i.get("findings"))
                    .and_then(|h| h.as_str())
                    .unwrap_or("verification failed");
                format!("{crit}: {hint}")
            })
            .unwrap_or_else(|| "acceptance criteria not met".into());
        let _ = writeln!(
            self.out,
            "    {}  {} {}",
            self.theme.dim("⎿"),
            self.theme.yellow_bold(&format!(
                "⚠ Verification gate (retry {retry}/{max_retries}):"
            )),
            truncate_plain(&summary, 72)
        );
        let _ = self.out.flush();
    }
}

// ---- Markdown & Run Summary Rendering ----

/// Render Markdown-like assistant output with highlighted headings, code blocks,
/// bullet points, and `source://` / `artifact://` provenance citations.
pub fn render_markdown_block(text: &str, theme: &Theme) -> String {
    let mut out = Vec::new();
    let mut in_code = false;
    for raw_line in text.lines() {
        let trimmed = raw_line.trim_start();
        if trimmed.starts_with("```") {
            in_code = !in_code;
            out.push(format!("  {}", theme.dim(raw_line)));
            continue;
        }
        if in_code {
            out.push(format!("  {} {}", theme.dim("│"), theme.cyan(raw_line)));
            continue;
        }
        if let Some(h) = trimmed.strip_prefix("### ") {
            out.push(format!("  {}", theme.bold(h)));
        } else if let Some(h) = trimmed.strip_prefix("## ") {
            out.push(format!("  {}", theme.accent(h)));
        } else if let Some(h) = trimmed.strip_prefix("# ") {
            out.push(format!("  {}", theme.accent(h)));
        } else if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            out.push(format!(
                "  {} {}",
                theme.accent("•"),
                highlight_refs(item, theme)
            ));
        } else {
            out.push(format!("  {}", highlight_refs(raw_line, theme)));
        }
    }
    out.join("\n")
}

fn highlight_refs(line: &str, theme: &Theme) -> String {
    if !theme.color {
        return line.to_string();
    }
    let mut out = String::new();
    for token in line.split_inclusive(' ') {
        let trimmed = token.trim_end_matches(|c: char| c.is_whitespace() || ",.;:)]".contains(c));
        if trimmed.starts_with("source://")
            || trimmed.starts_with("artifact://")
            || trimmed.starts_with("skill://")
            || trimmed.starts_with("wiki://")
        {
            let suffix = &token[trimmed.len()..];
            out.push_str(&theme.cyan(trimmed));
            out.push_str(suffix);
        } else {
            out.push_str(token);
        }
    }
    out
}

pub fn render_run_summary(result: &TaskRunResult, theme: &Theme) -> String {
    let mut sections = Vec::new();

    if let Some(answer) = &result.answer {
        if !answer.trim().is_empty() {
            sections.push(format!(
                "\n  {} {}\n{}",
                theme.accent("✻"),
                theme.bold("Response"),
                render_markdown_block(answer, theme)
            ));
        }
    }

    if !result.open_issues.is_empty() {
        let mut issue_lines = Vec::new();
        for issue in &result.open_issues {
            if let Some(q) = issue.get("question").and_then(|v| v.as_str()) {
                if !q.is_empty() {
                    issue_lines.push(format!("{} {}", theme.yellow_bold("? Question:"), q));
                    continue;
                }
            }
            let key = issue
                .get("error_code")
                .or_else(|| issue.get("criterion"))
                .or_else(|| issue.get("key"))
                .and_then(|v| v.as_str())
                .unwrap_or("issue");
            let detail = issue
                .get("findings")
                .or_else(|| issue.get("repair_hint"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| compact_json(issue, 68));
            issue_lines.push(format!("{} — {}", theme.red_bold(key), detail));
        }
        sections.push(format!(
            "\n{}",
            render_box(
                &theme.yellow_bold("Open Issues / Action Required"),
                &issue_lines,
                68,
                theme
            )
        ));
    }

    let cost_usd = result
        .cost
        .get("cost_usd")
        .and_then(|c| c.as_f64())
        .unwrap_or(0.0);
    let tool_calls = result
        .cost
        .get("tool_calls")
        .and_then(|c| c.as_u64())
        .unwrap_or(0);
    let in_tok = result
        .cost
        .get("input_tokens")
        .and_then(|c| c.as_u64())
        .unwrap_or(0);
    let out_tok = result
        .cost
        .get("output_tokens")
        .and_then(|c| c.as_u64())
        .unwrap_or(0);

    // Note: Keep `task=<id>` and `status=<status>` substrings so existing CLI
    // smoke tests (`stdout.contains("completed")` and `stdout.contains("task=")`) pass!
    let status_badge = theme.status_badge(result.status);
    let footer = format!(
        "\n  {} {} · task={} · status={} · steps={} · tool_calls={} · tokens={}→{} · cost=${cost_usd:.6}\n",
        theme.dim("───"),
        status_badge,
        result.task_id,
        result.status.as_str(),
        result.steps,
        tool_calls,
        in_tok,
        out_tok
    );
    sections.push(footer);
    sections.join("")
}

// ---- Inspector Cards (Tasks, Status, Inspect, Context, Wiki, Skills, Verify, Trace, Doctor) ----

pub fn render_tasks_table(tasks: &[(Task, TaskState)], theme: &Theme) -> String {
    if tasks.is_empty() {
        return render_box(
            &theme.accent("✻ Tasks"),
            &[theme.dim("No tasks recorded yet. Run `/run tool-use` or type a goal.")],
            64,
            theme,
        );
    }
    let mut lines = vec![format!(
        "{:<18} {:<20} {:<8} {:<8} {}",
        theme.dim("TASK ID"),
        theme.dim("STATUS"),
        theme.dim("SEQ"),
        theme.dim("TOOLS"),
        theme.dim("GOAL")
    )];
    lines.push(theme.dim(&"─".repeat(76)));
    for (task, state) in tasks {
        lines.push(format!(
            "{} {} {:<8} {:<8} {}",
            pad_right_ansi(&theme.cyan(&task.task_id), 18),
            pad_right_ansi(&theme.status_badge(state.status), 20),
            state.last_seq,
            state.tool_calls_made,
            truncate_plain(&task.goal, 34)
        ));
    }
    render_box(&theme.accent("✻ Recent Tasks"), &lines, 76, theme)
}

pub fn render_task_status_card(task: &Task, state: &TaskState, theme: &Theme) -> String {
    let mut lines = vec![
        format!(
            "{}  {}",
            theme.dim("Task ID      :"),
            theme.cyan(&task.task_id)
        ),
        format!(
            "{}  {} (status={})",
            theme.dim("Status       :"),
            theme.status_badge(state.status),
            state.status.as_str()
        ),
        format!(
            "{}  {}",
            theme.dim("Goal         :"),
            theme.bold(&truncate_plain(&task.goal, 62))
        ),
        format!(
            "{}  seq={} · version={} · model_calls={} · tool_calls={}",
            theme.dim("Progress     :"),
            state.last_seq,
            state.version,
            state.model_calls_made,
            state.tool_calls_made
        ),
    ];
    if !state.pinned_constraints.is_empty() {
        lines.push(format!(
            "{}  {}",
            theme.dim("Constraints  :"),
            theme.magenta(&truncate_plain(&state.pinned_constraints.join(" · "), 62))
        ));
    }
    if !task.acceptance_rules.is_empty() {
        lines.push(format!(
            "{}  {}",
            theme.dim("Acceptance   :"),
            theme.yellow(&truncate_plain(&task.acceptance_rules.join(", "), 62))
        ));
    }
    if !state.completed_outputs.is_empty() {
        lines.push(format!(
            "{}  {}",
            theme.dim("Outputs      :"),
            theme.cyan(&state.completed_outputs.join(", "))
        ));
    }
    if !state.verifications.is_empty() {
        lines.push(format!(
            "{}  {} recorded check(s)",
            theme.dim("Verifications:"),
            state.verifications.len()
        ));
    }
    if !state.open_issues.is_empty() {
        lines.push(format!(
            "{}  {} open issue(s)",
            theme.dim("Open Issues  :"),
            theme.red_bold(&state.open_issues.len().to_string())
        ));
        for issue in &state.open_issues {
            lines.push(format!("   {} {}", theme.red("•"), compact_json(issue, 66)));
        }
    }
    if !state.unresolved_errors.is_empty() {
        lines.push(format!(
            "{}  {} unresolved error(s)",
            theme.dim("Errors       :"),
            theme.red_bold(&state.unresolved_errors.len().to_string())
        ));
    }
    render_box(&theme.accent("✻ Task Authority & State"), &lines, 74, theme)
}

pub fn render_inspect_timeline(
    task: &Task,
    state: &TaskState,
    events: &[Event],
    theme: &Theme,
) -> String {
    let mut out = render_task_status_card(task, state, theme);
    out.push('\n');
    let mut lines = Vec::new();
    for ev in events {
        let etype = match ev.event_type.as_str() {
            "task.created" | "task.status_changed" => theme.cyan(&ev.event_type),
            "tool.committed" => theme.green(&ev.event_type),
            "verification.recorded" => theme.yellow_bold(&ev.event_type),
            "task.error_opened" | "task.issue_recorded" => theme.red_bold(&ev.event_type),
            other => theme.bold(other),
        };
        let detail = summarize_event_payload(&ev.event_type, &ev.payload);
        lines.push(format!(
            "{:<5} {:<16} {}  {}",
            theme.dim(&format!("#{}", ev.seq)),
            pad_right_ansi(&theme.dim(&ev.actor), 16),
            pad_right_ansi(&etype, 24),
            truncate_plain(&detail, 42)
        ));
    }
    out.push_str(&render_box(
        &theme.accent(&format!(
            "✻ Append-Only Event Ledger ({} events)",
            events.len()
        )),
        &lines,
        76,
        theme,
    ));
    out
}

fn summarize_event_payload(event_type: &str, payload: &Value) -> String {
    match event_type {
        "task.status_changed" => format!(
            "-> {} ({})",
            payload
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("?"),
            payload.get("reason").and_then(|v| v.as_str()).unwrap_or("")
        ),
        "task.intent_started" => format!(
            "kind={} reason={}",
            payload.get("kind").and_then(|v| v.as_str()).unwrap_or("?"),
            payload
                .get("reason_code")
                .and_then(|v| v.as_str())
                .unwrap_or("")
        ),
        "tool.committed" => format!(
            "op={} status={} preview={}",
            payload
                .get("operation_id")
                .and_then(|v| v.as_str())
                .unwrap_or("?"),
            payload
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("?"),
            payload
                .get("result_preview")
                .and_then(|v| v.as_str())
                .unwrap_or("")
        ),
        "verification.recorded" => format!(
            "criterion={} verdict={}",
            payload
                .get("criterion")
                .and_then(|v| v.as_str())
                .unwrap_or("?"),
            payload
                .get("verdict")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
        ),
        _ => compact_json(payload, 48),
    }
}

pub fn render_context_search(query: &str, hits: &[SearchCandidate], theme: &Theme) -> String {
    let mut lines = vec![format!(
        "{} {:?} ({} hit(s))",
        theme.dim("Query:"),
        query,
        hits.len()
    )];
    if hits.is_empty() {
        lines.push(theme.dim("No indexed documents matched."));
    } else {
        lines.push(theme.dim(&"─".repeat(72)));
        for h in hits {
            lines.push(format!(
                "● {}  {}  {}",
                theme.cyan(&h.ref_id),
                theme.bold(&h.title),
                theme.dim(&format!(
                    "(~{} tok · score {:.2})",
                    h.token_estimate, h.lexical_score
                ))
            ));
            lines.push(format!(
                "  {}  {}",
                theme.dim("⎿"),
                truncate_plain(&h.snippet, 68)
            ));
        }
    }
    render_box(
        &theme.accent("✻ Context Index Search (FTS5)"),
        &lines,
        74,
        theme,
    )
}

pub fn render_context_read(payload: &ReadPayload, theme: &Theme) -> String {
    let mut lines = vec![
        format!(
            "{}  {}",
            theme.dim("Reference :"),
            theme.cyan(&payload.ref_id)
        ),
        format!(
            "{}  rev={} · ~{} tokens · truncated={}",
            theme.dim("Metadata  :"),
            payload.revision,
            payload.token_estimate,
            payload.truncated
        ),
        format!(
            "{}  {}",
            theme.dim("Provenance:"),
            compact_json(&payload.provenance, 60)
        ),
        theme.dim(&"─".repeat(72)),
    ];
    for line in payload.excerpt.lines() {
        lines.push(truncate_plain(line, 72));
    }
    render_box(&theme.accent("✻ Bounded Context Read"), &lines, 74, theme)
}

pub fn render_wiki_view(
    task_id: &str,
    summary: &Value,
    nodes: &[Value],
    edges: &[Value],
    theme: &Theme,
) -> String {
    let last_seq = summary
        .get("last_compiled_seq")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let stale_refs = summary
        .get("stale_refs")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let edge_count = summary.get("edges").and_then(|v| v.as_i64()).unwrap_or(0);

    let mut lines = vec![
        format!("{}  {}", theme.dim("Task ID     :"), theme.cyan(task_id)),
        format!(
            "{}  last_compiled_seq={} · nodes={} · edges={} · stale_refs={}",
            theme.dim("Graph Stats :"),
            last_seq,
            nodes.len(),
            edge_count,
            stale_refs
        ),
    ];

    let mut node_labels = std::collections::BTreeMap::new();
    for n in nodes {
        if let Some(id) = n.get("node_id").and_then(|v| v.as_str()) {
            let kind = n.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
            let title = n.get("title").and_then(|v| v.as_str()).unwrap_or(id);
            node_labels.insert(
                id.to_string(),
                format!("[{kind}] {}", truncate_plain(title, 22)),
            );
        }
    }

    if !nodes.is_empty() {
        lines.push(theme.dim(&"─".repeat(74)));
        lines.push(theme.bold("Nodes (Derived Knowledge & Provenance)"));
        for n in nodes {
            let kind = n.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
            let status = n.get("status").and_then(|v| v.as_str()).unwrap_or("valid");
            let title = n.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let ref_id = n.get("ref_id").and_then(|v| v.as_str()).unwrap_or("");
            let status_tag = match status {
                "disputed" => theme.red_bold("! disputed"),
                "stale" => theme.yellow_bold("⚠ stale"),
                _ => theme.green("✓ valid"),
            };
            lines.push(format!(
                "  ● [{}] {} {} {}",
                theme.magenta(kind),
                theme.bold(&truncate_plain(title, 34)),
                status_tag,
                if ref_id.is_empty() {
                    String::new()
                } else {
                    theme.cyan(&format!("← {ref_id}"))
                }
            ));
        }
    }

    if !edges.is_empty() {
        lines.push(theme.dim(&"─".repeat(74)));
        lines.push(theme.bold("Typed Edges"));
        for e in edges.iter().take(15) {
            let etype = e.get("edge_type").and_then(|v| v.as_str()).unwrap_or("?");
            let from = e.get("from_node").and_then(|v| v.as_str()).unwrap_or("?");
            let to = e.get("to_node").and_then(|v| v.as_str()).unwrap_or("?");
            let from_label = node_labels.get(from).map(|s| s.as_str()).unwrap_or(from);
            let to_label = node_labels.get(to).map(|s| s.as_str()).unwrap_or(to);
            let styled_edge = if etype == "contradicts" {
                theme.red_bold(etype)
            } else if etype == "verified_by" {
                theme.green_bold(etype)
            } else {
                theme.cyan(etype)
            };
            lines.push(format!(
                "  {} ──[{}]──▶ {}",
                theme.dim(from_label),
                styled_edge,
                theme.dim(to_label)
            ));
        }
    }

    render_box(
        &theme.accent("✻ Incremental Derived Wiki (M4)"),
        &lines,
        76,
        theme,
    )
}

pub fn render_skills_list(manifests: &[SkillManifest], theme: &Theme) -> String {
    let mut lines = vec![theme.dim(
        "Progressive disclosure: discovery loads name/version/description only (0 instruction tokens).",
    )];
    lines.push(theme.dim(&"─".repeat(74)));
    if manifests.is_empty() {
        lines.push(theme.dim("No skill packages found in skills directory."));
    } else {
        for m in manifests {
            lines.push(format!(
                "● {} {} — {}",
                theme.magenta(&m.name),
                theme.dim(&format!("v{}", m.version)),
                m.description
            ));
        }
    }
    render_box(&theme.accent("✻ Installed Skills (M5)"), &lines, 76, theme)
}

pub fn render_skill_detail(act: &SkillActivation, theme: &Theme) -> String {
    let mut lines = vec![
        format!(
            "{}  {} v{}",
            theme.dim("Skill      :"),
            theme.magenta(&act.name),
            act.version
        ),
        format!("{}  {}", theme.dim("Ref ID     :"), theme.cyan(&act.ref_id)),
        format!("{}  {}", theme.dim("Description:"), act.description),
        theme.dim(&"─".repeat(72)),
    ];
    for l in act.instructions.lines() {
        lines.push(truncate_plain(l, 72));
    }
    render_box(
        &theme.accent("✻ Activated Skill Payload"),
        &lines,
        74,
        theme,
    )
}

pub fn render_capabilities_table(manifests: &[CapabilityManifest], theme: &Theme) -> String {
    let mut lines = Vec::new();
    for m in manifests {
        lines.push(format!(
            "● {} {} [{}] — {}",
            theme.bold(&m.id),
            theme.dim(&format!("v{}", m.version)),
            theme.risk_badge(m.risk_class),
            m.description
        ));
        for op in &m.operations {
            lines.push(format!(
                "  {}  {:<20} [{}] {}",
                theme.dim("⎿"),
                theme.cyan(&op.id),
                theme.risk_badge(op.risk_class),
                op.description
            ));
        }
    }
    render_box(
        &theme.accent("✻ Capability Fabric Registry"),
        &lines,
        76,
        theme,
    )
}

pub fn render_verification_card(
    task_id: &str,
    results: &[VerificationResult],
    theme: &Theme,
) -> String {
    let mut lines = vec![format!(
        "{}  {}",
        theme.dim("Task ID:"),
        theme.cyan(task_id)
    )];
    lines.push(theme.dim(&"─".repeat(74)));
    if results.is_empty() {
        lines.push(theme.dim("No verification criteria specified."));
    }
    for r in results {
        let badge = match r.verdict {
            Verdict::Pass => theme.green_bold("✓ PASS"),
            Verdict::Fail => theme.red_bold("✗ FAIL"),
            Verdict::Uncertain => theme.yellow_bold("? UNCERTAIN"),
        };
        lines.push(format!(
            "{}  {} — {}",
            badge,
            theme.bold(&r.criterion),
            r.findings
        ));
        for d in &r.diagnostics {
            lines.push(format!(
                "  {}  {} (target: {}) → {}",
                theme.dim("⎿"),
                theme.red_bold(&d.error_code),
                theme.cyan(&d.target_ref),
                d.repair_hint
            ));
        }
    }
    render_box(
        &theme.accent("✻ Independent Verify & Debug (M6)"),
        &lines,
        76,
        theme,
    )
}

pub fn render_trace_card(
    task_id: &str,
    entries: &[Value],
    replay: &ReplayReport,
    theme: &Theme,
) -> String {
    let status = if replay.ok() {
        theme.green_bold("✓ REPLAY VERIFIED (cost & tokens match step ledger)")
    } else {
        theme.red_bold(&format!("✗ ANOMALIES: {}", replay.anomalies.join(", ")))
    };
    let mut lines = vec![
        format!("{}  {}", theme.dim("Task ID     :"), theme.cyan(task_id)),
        format!("{}  {}", theme.dim("Audit Status:"), status),
        format!(
            "{}  entries={} · decisions={} · tool_results={} · tokens={}→{} · cost=${:.6}",
            theme.dim("Totals      :"),
            entries.len(),
            replay.decisions,
            replay.tool_results,
            replay.recomputed_input_tokens,
            replay.recomputed_output_tokens,
            replay.recomputed_cost_usd
        ),
        theme.dim(&"─".repeat(74)),
    ];
    for (idx, entry) in entries.iter().enumerate() {
        let kind = entry.get("kind").and_then(|k| k.as_str()).unwrap_or("?");
        let payload = entry.get("payload").cloned().unwrap_or(Value::Null);
        lines.push(format!(
            "{:<4} {:<16} {}",
            theme.dim(&format!("{idx}.")),
            theme.bold(kind),
            compact_json(&payload, 54)
        ));
    }
    render_box(
        &theme.accent("✻ Execution Trace & Cost Replay"),
        &lines,
        76,
        theme,
    )
}

pub fn render_fixtures_table(theme: &Theme) -> String {
    let mut lines = Vec::new();
    for name in FIXTURE_NAMES {
        if let Ok(spec) = get_fixture(name) {
            lines.push(format!(
                "● {} — {}",
                theme.cyan(&spec.name),
                theme.bold(&spec.description)
            ));
            lines.push(format!(
                "  {}  Goal: {}",
                theme.dim("⎿"),
                truncate_plain(&spec.goal, 66)
            ));
        }
    }
    render_box(
        &theme.accent("✻ Offline Deterministic Fixtures"),
        &lines,
        76,
        theme,
    )
}

pub fn render_doctor_card(app: &App, perm_mode: PermissionMode, theme: &Theme) -> String {
    let db_path = app.storage_root.join("sfai.sqlite3");
    let blobs_path = app.storage_root.join("blobs");
    let traces_path = &app.trace_dir;
    let caps = app.registry.discover("", 100, None).len();
    let skills = app.skills.discover("", 100).len();
    let tasks = app.task_store.list(1000).map(|t| t.len()).unwrap_or(0);
    let model_env = detect_model_label();

    let lines = vec![
        format!(
            "{}  {} ({})",
            theme.green_bold("✓"),
            theme.bold("SQLite WAL Ledger"),
            db_path.display()
        ),
        format!(
            "{}  {} ({})",
            theme.green_bold("✓"),
            theme.bold("Content-Addressed Object Store"),
            blobs_path.display()
        ),
        format!(
            "{}  {} ({})",
            theme.green_bold("✓"),
            theme.bold("JSONL Trace Directory"),
            traces_path.display()
        ),
        format!(
            "{}  {} ({} registered capabilities, {} installed skills, {} stored tasks)",
            theme.green_bold("✓"),
            theme.bold("Capability & Skill Fabric"),
            caps,
            skills,
            tasks
        ),
        format!(
            "{}  {} ({})",
            theme.green_bold("✓"),
            theme.bold("Runtime Policy & Approval Broker"),
            perm_mode.badge(theme)
        ),
        format!(
            "{}  {} ({})",
            theme.cyan("●"),
            theme.bold("Model Provider Endpoint"),
            model_env
        ),
    ];
    render_box(&theme.accent("✻ System Doctor"), &lines, 76, theme)
}

pub fn render_help_card(theme: &Theme) -> String {
    let lines = vec![
        theme.bold("Execution & Routing"),
        format!(
            "  {:<28} {}",
            theme.cyan("<type any goal>"),
            "Run a goal through the self-routing decision kernel"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/run <fixture>"),
            "Run an offline fixture (direct-answer | tool-use | failed-tool | approval | context-retrieval)"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/resume <task_id> [fixture]"),
            "Crash-recover and resume an interrupted task"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/fixtures"),
            "List deterministic offline fixtures and their goals"
        ),
        String::new(),
        theme.bold("Context, Wiki & Inspection"),
        format!(
            "  {:<28} {}",
            theme.cyan("/tasks"),
            "List recent tasks with lifecycle status badges"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/status [task_id]"),
            "Show task authority card (goal, pinned constraints, outputs, issues)"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/inspect [task_id]"),
            "Show append-only event ledger timeline"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/search <query>"),
            "Search index-native context via FTS5"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/read <ref_id>"),
            "Read a bounded excerpt + provenance from a source/artifact ref"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/wiki [task_id]"),
            "Show derived Wiki knowledge nodes, typed edges & conflicts"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/verify [task_id] [rules..]"),
            "Run independent deterministic verification"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/trace [task_id]"),
            "Replay JSONL trace and audit token/cost accounting"
        ),
        String::new(),
        theme.bold("Capabilities, Governance & Session"),
        format!(
            "  {:<28} {}",
            theme.cyan("/skills [name]"),
            "List discovered skills (metadata only) or inspect one skill"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/capabilities"),
            "List registered capabilities, operations & risk tiers"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/permissions [deny|ask|auto]"),
            "View or switch runtime approval mode for consequential actions"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/constraints [add ..|clear]"),
            "Manage session pinned constraints for new interactive goals"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/accept [add ..|clear]"),
            "Manage session acceptance rules for new interactive goals"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/cost"),
            "Show cumulative session token & cost dashboard"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/doctor"),
            "Check SQLite WAL, blob store, skills, and model configuration"
        ),
        format!(
            "  {:<28} {}",
            theme.cyan("/clear · /help · /quit"),
            "Clear screen, show this help card, or exit session"
        ),
    ];
    render_box(
        &theme.accent("✻ Claude Code-Style CLI Commands"),
        &lines,
        78,
        theme,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_width_ignores_ansi_and_counts_cjk() {
        let theme = Theme::colored();
        let styled = theme.accent("✻ 你好 sfai");
        // "✻" (1) + " " (1) + "你" (2) + "好" (2) + " " (1) + "sfai" (4) = 11
        assert_eq!(display_width(&styled), 11);
    }

    #[test]
    fn box_renderer_aligns_borders() {
        let theme = Theme::plain();
        let b = render_box("Title", &["line one".into(), "line two".into()], 20, &theme);
        let lines: Vec<&str> = b.lines().collect();
        assert_eq!(lines.len(), 4);
        let w0 = display_width(lines[0]);
        for line in &lines {
            assert_eq!(display_width(line), w0, "misaligned box line: {line}");
        }
    }

    #[test]
    fn permission_mode_roundtrip() {
        assert_eq!(PermissionMode::from_str("deny"), Some(PermissionMode::Deny));
        assert_eq!(PermissionMode::from_str("ask"), Some(PermissionMode::Ask));
        assert_eq!(PermissionMode::from_str("auto"), Some(PermissionMode::Auto));
        assert_eq!(PermissionMode::from_str("invalid"), None);
    }
}
