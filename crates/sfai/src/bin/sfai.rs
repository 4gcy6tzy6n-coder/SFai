//! CLI: Claude Code-style interactive session + subcommands (`run`, `resume`,
//! `tasks`, `status`, `inspect`, `context`, `capabilities`, `skills`, `wiki`,
//! `verify`, `export-trace`, `doctor`).

use clap::{Parser, Subcommand};
use serde_json::json;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use sfai::app::{build_app_with_options, App};
use sfai::contracts::Budget;
use sfai::fixtures::{get_fixture, FIXTURE_NAMES};
use sfai::integrations::providers::fake::ConversationalDemoModel;
use sfai::integrations::providers::openai_compat::OpenAICompatibleAdapter;
use sfai::telemetry::replay::replay_trace;
use sfai::ui::{
    collect_modified_files, read_claude_code_viewport_input, render_box, render_capabilities_table,
    render_chat_history, render_context_read, render_context_search, render_doctor_card,
    render_file_diff_box, render_files_summary_card, render_fixtures_table, render_help_card,
    render_inspect_timeline, render_run_summary, render_skill_detail, render_skills_list,
    render_task_status_card, render_tasks_table, render_top_dashboard, render_trace_card,
    render_user_chat_bubble, render_verification_card, render_wiki_view, ChatComposerStatus,
    ChatTurn, ClaudeStepObserver, CliApprovalProvider, DashboardInfo, PermissionMode, TeeWriter,
    Theme,
};

#[derive(Parser)]
#[command(
    name = "sfai",
    about = "System for future ai — Claude Code-style CLI & offline-capable agent runtime"
)]
struct Cli {
    /// Storage directory for SQLite ledger, blobs, workspaces, and traces
    #[arg(long, default_value = ".sfai", global = true)]
    storage: PathBuf,
    /// Optional Skill packages directory (defaults to `<storage>/skills` or `./skills`)
    #[arg(long, global = true)]
    skills: Option<PathBuf>,
    /// Render rich Claude Code terminal UI for subcommands instead of raw JSON
    #[arg(long, global = true)]
    ui: bool,
    /// Disable ANSI color codes
    #[arg(long, global = true)]
    no_color: bool,
    /// Runtime approval policy for consequential actions: deny | ask | auto
    #[arg(long, default_value = "deny", global = true)]
    permissions: String,
    /// Model provider: deepseek | qwen | mimo | custom (custom uses SFAI_MODEL_*)
    #[arg(long, global = true)]
    provider: Option<String>,
    /// Override the provider's model name
    #[arg(long, global = true)]
    model: Option<String>,
    /// Override input price (USD per 1M tokens)
    #[arg(long, global = true)]
    price_in: Option<f64>,
    /// Override output price (USD per 1M tokens)
    #[arg(long, global = true)]
    price_out: Option<f64>,
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run one task (offline fixture or provider goal)
    Run {
        #[arg(long)]
        fixture: Option<String>,
        #[arg(long)]
        goal: Option<String>,
        #[arg(long)]
        constraint: Vec<String>,
        #[arg(long)]
        accept: Vec<String>,
        #[arg(long, default_value_t = 50)]
        max_steps: usize,
    },
    /// Resume an interrupted task
    Resume {
        task_id: String,
        #[arg(long)]
        fixture: Option<String>,
    },
    /// List recent tasks in storage
    Tasks {
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Show task + state
    Status { task_id: String },
    /// Show state + event log
    Inspect { task_id: String },
    /// Context index operations
    Context {
        #[command(subcommand)]
        ctx: CtxCmd,
    },
    /// List capabilities
    Capabilities,
    /// Run verification criteria
    Verify {
        task_id: String,
        #[arg(long)]
        criteria: Vec<String>,
    },
    /// Export the run trace for a task
    ExportTrace { task_id: String },
    /// Skill packages (M5): list metadata or show activated instructions
    Skills {
        #[command(subcommand)]
        skills: SkillsCmd,
    },
    /// Derived Wiki operations (M4)
    Wiki {
        #[command(subcommand)]
        wiki: WikiCmd,
    },
    /// Run system health and configuration diagnostics
    Doctor,
    /// Run the agent framework benchmark (time/correctness/tokens/cost/ablations)
    Bench {
        /// Suite name or comma list: all|tool-use|dataset|long-horizon|edge-case
        #[arg(long, default_value = "all")]
        suite: String,
        /// Ablation or comma list: all|none|full_context|no_pinned|no_verification|no_wiki
        #[arg(long, default_value = "all")]
        ablation: String,
        #[arg(long, default_value_t = 6)]
        size: usize,
        /// Write the JSON report to this path (default: <storage>/benchmarks/report-<ts>.json)
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Run the offline sparse-context evaluation harness (research track)
    Eval {
        #[arg(long, default_value = "sparse-context")]
        suite: String,
        #[arg(long, default_value_t = 12)]
        size: usize,
        #[arg(long, default_value_t = 120)]
        budget: usize,
        /// Write the JSON report to this path (default: <storage>/evals/report-<ts>.json)
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Serve the local HTTP API for the UI (tasks, events/SSE, artifacts, wiki)
    Serve {
        /// TCP port (default 8765)
        #[arg(long, default_value_t = 8765)]
        port: u16,
        /// Bind address (loopback only by default)
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
    },
}

#[derive(Subcommand)]
enum SkillsCmd {
    /// Discover installed skills (metadata only)
    List {
        #[arg(long, default_value = "")]
        query: String,
    },
    /// Show one skill's instructions (activation payload)
    Show { name: String },
}

#[derive(Subcommand)]
enum WikiCmd {
    /// Incrementally compile the Wiki for a task
    Compile { task_id: String },
    /// Show derived Wiki summary for a task
    Summary { task_id: String },
}

#[derive(Subcommand)]
enum CtxCmd {
    Search {
        query: String,
        #[arg(long)]
        limit: Option<usize>,
    },
    Read {
        ref_id: String,
        #[arg(long)]
        max_tokens: Option<usize>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match dispatch(cli) {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("sfai error: {message}");
            ExitCode::from(2)
        }
    }
}

fn resolve_skills_dir(storage: &std::path::Path, explicit: Option<PathBuf>) -> Option<PathBuf> {
    if explicit.is_some() {
        return explicit;
    }
    let storage_skills = storage.join("skills");
    if storage_skills.exists() {
        return Some(storage_skills);
    }
    let local_skills = PathBuf::from("skills");
    if local_skills.exists() {
        return Some(local_skills);
    }
    None
}

fn dispatch(cli: Cli) -> Result<u8, String> {
    let theme = Theme::auto(cli.no_color);
    let perm_mode = PermissionMode::from_str(&cli.permissions).ok_or_else(|| {
        format!(
            "invalid --permissions {:?}; choose from deny, ask, auto",
            cli.permissions
        )
    })?;
    let approvals = Arc::new(CliApprovalProvider::new(perm_mode, theme));
    let skills_dir = resolve_skills_dir(&cli.storage, cli.skills.clone());
    let app: App = build_app_with_options(&cli.storage, approvals.clone(), skills_dir)
        .map_err(|e| e.to_string())?;

    let Some(cmd) = cli.cmd else {
        return interactive(&app, &approvals, theme);
    };

    match cmd {
        Cmd::Run {
            fixture,
            goal,
            constraint,
            accept,
            max_steps,
        } => {
            let (task, mut model) = if let Some(name) = fixture {
                let spec = get_fixture(&name).map_err(|e| e.to_string())?;
                let mut constraints = spec.constraints.clone();
                constraints.extend(constraint);
                let mut rules = spec.acceptance_rules.clone();
                rules.extend(accept);
                let task = app
                    .create_task(&spec.goal, constraints, rules, Some(spec.budget.clone()))
                    .map_err(|e| e.to_string())?;
                if let Some(setup) = spec.setup {
                    setup(&app, &task).map_err(|e| e.to_string())?;
                }
                (task, (spec.build_model)())
            } else if let Some(goal) = goal {
                let task = app
                    .create_task(&goal, constraint, accept, Some(Budget::default()))
                    .map_err(|e| e.to_string())?;
                (
                    task,
                    Box::new(OpenAICompatibleAdapter::from_env())
                        as Box<dyn sfai::core::model_adapter::ModelAdapter>,
                )
            } else {
                return Err(format!(
                    "provide --fixture <{}> or --goal TEXT",
                    FIXTURE_NAMES.join("|")
                ));
            };

            let result = if cli.ui {
                let mut observer = ClaudeStepObserver::new(std::io::stdout(), theme);
                let res = app
                    .run_observed(model.as_mut(), &task.task_id, max_steps, &mut observer)
                    .map_err(|e| e.to_string())?;
                print!("{}", render_run_summary(&res, &theme));
                res
            } else {
                let res = app
                    .run(model.as_mut(), &task.task_id, max_steps)
                    .map_err(|e| e.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "task_id": res.task_id,
                        "status": res.status.as_str(),
                        "answer": res.answer,
                        "steps": res.steps,
                        "cost": res.cost,
                        "open_issues": res.open_issues,
                    }))
                    .map_err(|e| e.to_string())?
                );
                res
            };

            Ok(if result.status == sfai::contracts::TaskStatus::Completed {
                0
            } else {
                1
            })
        }
        Cmd::Resume { task_id, fixture } => {
            let state = app.resume_state(&task_id).map_err(|e| e.to_string())?;
            eprintln!("[sfai] resumed {task_id} at seq {}", state.last_seq);
            let mut model: Box<dyn sfai::core::model_adapter::ModelAdapter> = match fixture {
                Some(name) => (get_fixture(&name).map_err(|e| e.to_string())?.build_model)(),
                None => Box::new(OpenAICompatibleAdapter::from_env()),
            };
            let result = if cli.ui {
                let mut observer = ClaudeStepObserver::new(std::io::stdout(), theme);
                let res = app
                    .run_observed(model.as_mut(), &task_id, 50, &mut observer)
                    .map_err(|e| e.to_string())?;
                print!("{}", render_run_summary(&res, &theme));
                res
            } else {
                let res = app
                    .run(model.as_mut(), &task_id, 50)
                    .map_err(|e| e.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "task_id": res.task_id,
                        "status": res.status.as_str(),
                        "steps": res.steps,
                    }))
                    .map_err(|e| e.to_string())?
                );
                res
            };
            Ok(if result.status == sfai::contracts::TaskStatus::Completed {
                0
            } else {
                1
            })
        }
        Cmd::Tasks { limit } => {
            let tasks = app.task_store.list(limit).map_err(|e| e.to_string())?;
            if cli.ui {
                let mut pairs = Vec::with_capacity(tasks.len());
                for t in tasks {
                    let st = app
                        .task_store
                        .state(&t.task_id)
                        .map_err(|e| e.to_string())?;
                    pairs.push((t, st));
                }
                println!("{}", render_tasks_table(&pairs, &theme));
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&tasks).map_err(|e| e.to_string())?
                );
            }
            Ok(0)
        }
        Cmd::Status { task_id } => {
            let task = app.task_store.load(&task_id).map_err(|e| e.to_string())?;
            let state = app.task_store.state(&task_id).map_err(|e| e.to_string())?;
            if cli.ui {
                println!("{}", render_task_status_card(&task, &state, &theme));
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"task": task, "state": state}))
                        .map_err(|e| e.to_string())?
                );
            }
            Ok(0)
        }
        Cmd::Inspect { task_id } => {
            let task = app.task_store.load(&task_id).map_err(|e| e.to_string())?;
            let state = app.task_store.state(&task_id).map_err(|e| e.to_string())?;
            let events = app
                .ledger
                .events_for(&task_id, 0)
                .map_err(|e| e.to_string())?;
            if cli.ui {
                println!(
                    "{}",
                    render_inspect_timeline(&task, &state, &events, &theme)
                );
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"state": state, "events": events}))
                        .map_err(|e| e.to_string())?
                );
            }
            Ok(0)
        }
        Cmd::Context { ctx } => match ctx {
            CtxCmd::Search { query, limit } => {
                let hits = app
                    .retriever
                    .search(&query, None, None, limit.unwrap_or(5))
                    .map_err(|e| e.to_string())?;
                if cli.ui {
                    println!("{}", render_context_search(&query, &hits, &theme));
                } else {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!(hits
                            .iter()
                            .map(|h| json!({"ref_id": h.ref_id, "title": h.title, "snippet": h.snippet}))
                            .collect::<Vec<_>>()))
                        .map_err(|e| e.to_string())?
                    );
                }
                Ok(0)
            }
            CtxCmd::Read { ref_id, max_tokens } => {
                let payload = app
                    .retriever
                    .read(&ref_id, None, max_tokens.unwrap_or(600))
                    .map_err(|e| e.to_string())?;
                if cli.ui {
                    println!("{}", render_context_read(&payload, &theme));
                } else {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?
                    );
                }
                Ok(0)
            }
        },
        Cmd::Capabilities => {
            let manifests = app.registry.discover("", 50, None);
            if cli.ui {
                println!("{}", render_capabilities_table(&manifests, &theme));
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&manifests).map_err(|e| e.to_string())?
                );
            }
            Ok(0)
        }
        Cmd::Verify { task_id, criteria } => {
            let task = app.task_store.load(&task_id).map_err(|e| e.to_string())?;
            let criteria = if criteria.is_empty() {
                task.acceptance_rules
            } else {
                criteria
            };
            let results = app
                .verifier
                .verify(&task_id, &criteria)
                .map_err(|e| e.to_string())?;
            let all_pass = results
                .iter()
                .all(|r| r.verdict == sfai::contracts::Verdict::Pass);
            if cli.ui {
                println!("{}", render_verification_card(&task_id, &results, &theme));
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&results).map_err(|e| e.to_string())?
                );
            }
            Ok(if all_pass { 0 } else { 1 })
        }
        Cmd::ExportTrace { task_id } => {
            let entries = app.export_trace(&task_id).map_err(|e| e.to_string())?;
            if cli.ui {
                let replay = replay_trace(&entries);
                println!("{}", render_trace_card(&task_id, &entries, &replay, &theme));
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?
                );
            }
            Ok(0)
        }
        Cmd::Skills { skills } => match skills {
            SkillsCmd::List { query } => {
                let manifests = app.skills.discover(&query, 50);
                if cli.ui {
                    println!("{}", render_skills_list(&manifests, &theme));
                } else {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&manifests).map_err(|e| e.to_string())?
                    );
                }
                Ok(0)
            }
            SkillsCmd::Show { name } => {
                let activation = app.skills.activate(&name).map_err(|e| e.to_string())?;
                if cli.ui {
                    println!("{}", render_skill_detail(&activation, &theme));
                } else {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&activation).map_err(|e| e.to_string())?
                    );
                }
                Ok(0)
            }
        },
        Cmd::Wiki { wiki } => match wiki {
            WikiCmd::Compile { task_id } => {
                let report = app.wiki.compile_task(&task_id).map_err(|e| e.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
                );
                Ok(0)
            }
            WikiCmd::Summary { task_id } => {
                let summary = app.wiki.summary(&task_id).map_err(|e| e.to_string())?;
                if cli.ui {
                    let nodes = app.wiki.nodes_for(&task_id).map_err(|e| e.to_string())?;
                    let edges = app.wiki.edges_for(&task_id).map_err(|e| e.to_string())?;
                    println!(
                        "{}",
                        render_wiki_view(&task_id, &summary, &nodes, &edges, &theme)
                    );
                } else {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?
                    );
                }
                Ok(0)
            }
        },
        Cmd::Bench {
            suite,
            ablation,
            size,
            out,
        } => {
            let suites: Vec<String> = if suite == "all" {
                sfai::benchmark::SUITES
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            } else {
                suite.split(',').map(|s| s.trim().to_string()).collect()
            };
            let ablations: Vec<String> = if ablation == "all" {
                sfai::benchmark::ABLATIONS
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            } else {
                ablation.split(',').map(|s| s.trim().to_string()).collect()
            };
            for s in &suites {
                if !sfai::benchmark::SUITES.contains(&s.as_str()) {
                    return Err(format!(
                        "unknown suite {s:?}; choose from {:?}",
                        sfai::benchmark::SUITES
                    ));
                }
            }
            for a in &ablations {
                if !sfai::benchmark::ABLATIONS.contains(&a.as_str()) {
                    return Err(format!(
                        "unknown ablation {a:?}; choose from {:?}",
                        sfai::benchmark::ABLATIONS
                    ));
                }
            }
            let profile = match sfai::integrations::providers::profiles::resolve_for_cli(
                cli.provider.as_deref(),
                cli.model.as_deref(),
                cli.price_in,
                cli.price_out,
            ) {
                Ok(p) => Some(p),
                Err(_) if cli.provider.is_none() => None,
                Err(e) => return Err(e.to_string()),
            };
            let report = sfai::benchmark::run_benchmark_with_provider(
                &suites,
                &ablations,
                size,
                profile.as_ref(),
            );
            println!("{}", report.render_markdown());
            let stamp = report.generated_at.replace([':', '.'], "-");
            let path = out.unwrap_or_else(|| {
                cli.storage
                    .join("benchmarks")
                    .join(format!("report-{stamp}.json"))
            });
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&path, report.to_json()).map_err(|e| e.to_string())?;
            eprintln!("benchmark report written to {}", path.display());
            Ok(0)
        }
        Cmd::Eval {
            suite,
            size,
            budget,
            out,
        } => {
            let report = sfai::eval::run_suite(&suite, size, budget);
            println!("{}", report.render_markdown());
            let stamp = report.generated_at.replace([':', '.'], "-");
            let path = out.unwrap_or_else(|| {
                cli.storage
                    .join("evals")
                    .join(format!("report-{stamp}.json"))
            });
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&path, report.to_json()).map_err(|e| e.to_string())?;
            eprintln!("report written to {}", path.display());
            Ok(0)
        }
        Cmd::Serve { port, bind } => {
            let addr: std::net::SocketAddr = format!("{bind}:{port}")
                .parse()
                .map_err(|e| format!("invalid bind address {bind}:{port}: {e}"))?;
            let app = Arc::new(app);
            let profile = match sfai::integrations::providers::profiles::resolve_for_cli(
                cli.provider.as_deref(),
                cli.model.as_deref(),
                cli.price_in,
                cli.price_out,
            ) {
                Ok(p) => Some(p),
                Err(_) if cli.provider.is_none() => None,
                Err(e) => return Err(e.to_string()),
            };
            if let Some(profile) = &profile {
                eprintln!("default provider: {} ({})", profile.name, profile.model);
            }
            eprintln!("sfai api listening on http://{addr}");
            eprintln!("  GET  /api/health | /api/tasks | /api/tasks/:id | /api/tasks/:id/stream");
            eprintln!("  POST /api/tasks | /api/tasks/:id/run | /api/demo/:fixture | /api/tasks/:id/cancel");
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?
                .block_on(sfai::server::serve(
                    app,
                    addr,
                    std::env::var("SFAI_API_TOKEN").ok(),
                    profile,
                ))
                .map_err(|e| e.to_string())?;
            Ok(0)
        }
        Cmd::Doctor => {
            println!("{}", render_doctor_card(&app, approvals.mode(), &theme));
            Ok(0)
        }
    }
}

// ---- Claude Code-style Interactive REPL Session (bare `sfai`) ----

#[derive(Default)]
struct SessionStats {
    runs: usize,
    total_steps: usize,
    total_tool_calls: u64,
    total_input_tokens: u64,
    total_output_tokens: u64,
    total_cost_usd: f64,
}

impl SessionStats {
    fn record(&mut self, result: &sfai::core::decision_kernel::TaskRunResult) {
        self.runs += 1;
        self.total_steps += result.steps;
        self.total_tool_calls += result
            .cost
            .get("tool_calls")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        self.total_input_tokens += result
            .cost
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        self.total_output_tokens += result
            .cost
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        self.total_cost_usd += result
            .cost
            .get("cost_usd")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn build_dashboard_string(
    app: &App,
    approvals: &CliApprovalProvider,
    last_task: Option<&str>,
    model_mode: ChatModelMode,
    session_constraints_len: usize,
    turns_count: usize,
    stats: &SessionStats,
    theme: &Theme,
) -> String {
    let model_label = model_mode.label();
    let (active_status, active_refs_count, pinned_count, wiki_nodes, wiki_edges) =
        if let Some(tid) = last_task {
            let st = app.task_store.state(tid).ok();
            let status = st.as_ref().map(|s| s.status);
            let pinned = st
                .as_ref()
                .map(|s| s.pinned_constraints.len())
                .unwrap_or(session_constraints_len);
            let sources = app
                .objects
                .source_refs_for_task(tid)
                .map(|v| v.len())
                .unwrap_or(0);
            let arts = app
                .objects
                .artifact_refs_for_task(tid)
                .map(|v| v.len())
                .unwrap_or(0);
            let wn = app.wiki.nodes_for(tid).map(|v| v.len()).unwrap_or(0);
            let we = app.wiki.edges_for(tid).map(|v| v.len()).unwrap_or(0);
            (status, sources + arts, pinned, wn, we)
        } else {
            (None, 0, session_constraints_len, 0, 0)
        };
    let modified_files_count = collect_modified_files(app, None).len();
    let storage_str = app.storage_root.display().to_string();
    let info = DashboardInfo {
        storage: &storage_str,
        model_label: &model_label,
        permission_mode: approvals.mode(),
        active_task: last_task,
        active_status,
        active_refs_count,
        pinned_constraints_count: pinned_count,
        wiki_nodes,
        wiki_edges,
        modified_files_count,
        turns_count,
        total_input_tokens: stats.total_input_tokens,
        total_output_tokens: stats.total_output_tokens,
        session_cost_usd: stats.total_cost_usd,
    };
    render_top_dashboard(&info, theme)
}

#[allow(clippy::too_many_arguments)]
fn print_dashboard(
    app: &App,
    approvals: &CliApprovalProvider,
    last_task: Option<&str>,
    model_mode: ChatModelMode,
    session_constraints_len: usize,
    turns_count: usize,
    stats: &SessionStats,
    theme: &Theme,
) {
    println!(
        "{}\n",
        build_dashboard_string(
            app,
            approvals,
            last_task,
            model_mode,
            session_constraints_len,
            turns_count,
            stats,
            theme
        )
    );
}

fn resolve_target_task(arg: &str, last_task: Option<&str>, app: &App) -> Result<String, String> {
    let trimmed = arg.trim();
    if !trimmed.is_empty() {
        return Ok(trimmed.to_string());
    }
    if let Some(tid) = last_task {
        return Ok(tid.to_string());
    }
    if let Ok(mut recent) = app.task_store.list(1) {
        if let Some(first) = recent.pop() {
            return Ok(first.task_id);
        }
    }
    Err("no active task yet; run a goal or `/run <fixture>` first".into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatModelMode {
    Auto,
    Offline,
    Online,
}

impl ChatModelMode {
    fn label(&self) -> String {
        match self {
            ChatModelMode::Auto => {
                let adapter = OpenAICompatibleAdapter::from_env();
                if !adapter.base_url.trim().is_empty() && !adapter.model_name.trim().is_empty() {
                    adapter.model_name
                } else {
                    "sfai/local-assistant (auto)".into()
                }
            }
            ChatModelMode::Offline => "sfai/local-assistant (offline)".into(),
            ChatModelMode::Online => {
                let adapter = OpenAICompatibleAdapter::from_env();
                if !adapter.model_name.trim().is_empty() {
                    format!("{} (online)", adapter.model_name)
                } else {
                    "openai-compat (unconfigured)".into()
                }
            }
        }
    }
}

fn interactive(
    app: &App,
    approvals: &Arc<CliApprovalProvider>,
    theme: Theme,
) -> Result<u8, String> {
    use std::io::IsTerminal;
    let is_tty = std::io::stdin().is_terminal() && std::io::stdout().is_terminal() && theme.color;

    let mut last_task: Option<String> = app
        .task_store
        .list(1)
        .ok()
        .and_then(|mut v| v.pop().map(|t| t.task_id));
    let mut session_constraints: Vec<String> = Vec::new();
    let mut session_acceptance: Vec<String> = Vec::new();
    let mut stats = SessionStats::default();
    let mut chat_history: Vec<ChatTurn> = Vec::new();
    let mut model_mode = ChatModelMode::Auto;
    let mut file_snapshots: std::collections::BTreeMap<String, (u32, String)> =
        std::collections::BTreeMap::new();
    let mut middle_content = String::new();
    let mut notification = "Ready · sfai Code v0.1.0".to_string();

    if !is_tty {
        print_dashboard(
            app,
            approvals,
            last_task.as_deref(),
            model_mode,
            session_constraints.len(),
            chat_history.len(),
            &stats,
            &theme,
        );
    }

    loop {
        let model_label = model_mode.label();
        let modified_files = collect_modified_files(app, None);
        let composer_status = ChatComposerStatus {
            active_task: last_task.as_deref(),
            perm_mode: approvals.mode(),
            model_label: &model_label,
            constraints_count: session_constraints.len(),
            modified_files_count: modified_files.len(),
            turns_count: chat_history.len(),
            session_cost_usd: stats.total_cost_usd,
        };
        let (active_status, active_refs_count, pinned_count, wiki_nodes, wiki_edges) =
            if let Some(tid) = last_task.as_deref() {
                let st = app.task_store.state(tid).ok();
                let status = st.as_ref().map(|s| s.status);
                let pinned = st
                    .as_ref()
                    .map(|s| s.pinned_constraints.len())
                    .unwrap_or(session_constraints.len());
                let sources = app
                    .objects
                    .source_refs_for_task(tid)
                    .map(|v| v.len())
                    .unwrap_or(0);
                let arts = app
                    .objects
                    .artifact_refs_for_task(tid)
                    .map(|v| v.len())
                    .unwrap_or(0);
                let wn = app.wiki.nodes_for(tid).map(|v| v.len()).unwrap_or(0);
                let we = app.wiki.edges_for(tid).map(|v| v.len()).unwrap_or(0);
                (status, sources + arts, pinned, wn, we)
            } else {
                (None, 0, session_constraints.len(), 0, 0)
            };
        let storage_str = app.storage_root.display().to_string();
        let dash_info = DashboardInfo {
            storage: &storage_str,
            model_label: &model_label,
            permission_mode: approvals.mode(),
            active_task: last_task.as_deref(),
            active_status,
            active_refs_count,
            pinned_constraints_count: pinned_count,
            wiki_nodes,
            wiki_edges,
            modified_files_count: modified_files.len(),
            turns_count: chat_history.len(),
            total_input_tokens: stats.total_input_tokens,
            total_output_tokens: stats.total_output_tokens,
            session_cost_usd: stats.total_cost_usd,
        };

        let Some(line) = read_claude_code_viewport_input(
            &dash_info,
            &middle_content,
            &notification,
            &composer_status,
            &theme,
        )?
        else {
            break;
        };
        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if matches!(input, "/quit" | "/exit" | "quit" | "exit") {
            break;
        }
        if matches!(input, "/help" | "?") {
            let card = render_help_card(&theme);
            println!("{card}\n");
            middle_content = card;
            notification = "Showing /help shortcuts".into();
            continue;
        }
        if matches!(input, "/dashboard" | "/hud") {
            print_dashboard(
                app,
                approvals,
                last_task.as_deref(),
                model_mode,
                session_constraints.len(),
                chat_history.len(),
                &stats,
                &theme,
            );
            notification = "Dashboard refreshed".into();
            continue;
        }
        if input == "/files" || input.starts_with("/files ") {
            let arg = input.strip_prefix("/files").unwrap_or("").trim();
            let scope = if arg.is_empty() { None } else { Some(arg) };
            let files = collect_modified_files(app, scope);
            let card = render_files_summary_card(&files, &theme);
            println!("{card}\n");
            middle_content = card;
            notification = format!("{} modified file(s) in workspace", files.len());
            continue;
        }
        if input == "/diff" || input.starts_with("/diff ") {
            let arg = input.strip_prefix("/diff").unwrap_or("").trim();
            let files = collect_modified_files(app, None);
            let filtered: Vec<_> = if arg.is_empty() {
                files
            } else {
                files.into_iter().filter(|f| f.path == arg).collect()
            };
            if filtered.is_empty() {
                let msg = format!("  {} no modified files matching {:?}", theme.dim("◌"), arg);
                println!("{msg}\n");
                middle_content = msg;
            } else {
                let mut blocks = Vec::new();
                for change in &filtered {
                    let b = render_file_diff_box(change, 80, 0, &theme);
                    println!("{b}\n");
                    blocks.push(b);
                }
                middle_content = blocks.join("\n");
                notification = format!("Showing diff for {} file(s)", filtered.len());
            }
            continue;
        }
        if matches!(input, "/history" | "/chat") {
            let card = render_chat_history(&chat_history, &theme);
            println!("{card}\n");
            middle_content = card;
            notification = format!("Showing {} conversation turn(s)", chat_history.len());
            continue;
        }
        if input == "/model" || input.starts_with("/model ") {
            let arg = input.strip_prefix("/model").unwrap_or("").trim();
            match arg {
                "" => println!(
                    "  {} active chat model = {} (use `/model auto|offline|online` to switch)\n",
                    theme.accent("✻"),
                    theme.cyan(&model_mode.label())
                ),
                "auto" => {
                    model_mode = ChatModelMode::Auto;
                    println!(
                        "  {} switched chat model to {}\n",
                        theme.green_bold("✓"),
                        theme.cyan(&model_mode.label())
                    );
                }
                "offline" | "local" => {
                    model_mode = ChatModelMode::Offline;
                    println!(
                        "  {} switched chat model to {}\n",
                        theme.green_bold("✓"),
                        theme.cyan(&model_mode.label())
                    );
                }
                "online" | "provider" => {
                    model_mode = ChatModelMode::Online;
                    println!(
                        "  {} switched chat model to {}\n",
                        theme.green_bold("✓"),
                        theme.cyan(&model_mode.label())
                    );
                }
                other => println!(
                    "  {} invalid model mode {:?}; choose auto, offline, or online\n",
                    theme.red_bold("✗"),
                    other
                ),
            }
            continue;
        }
        if input == "/clear" {
            middle_content.clear();
            notification = "Cleared middle viewport".into();
            if !is_tty {
                print_dashboard(
                    app,
                    approvals,
                    last_task.as_deref(),
                    model_mode,
                    session_constraints.len(),
                    chat_history.len(),
                    &stats,
                    &theme,
                );
            }
            continue;
        }
        if input == "/fixtures" {
            let card = render_fixtures_table(&theme);
            println!("{card}\n");
            middle_content = card;
            continue;
        }
        if input == "/doctor" {
            let card = render_doctor_card(app, approvals.mode(), &theme);
            println!("{card}\n");
            middle_content = card;
            continue;
        }
        if input == "/capabilities" {
            let manifests = app.registry.discover("", 50, None);
            let card = render_capabilities_table(&manifests, &theme);
            println!("{card}\n");
            middle_content = card;
            continue;
        }
        if input == "/tasks" {
            match app.task_store.list(20) {
                Ok(tasks) => {
                    let mut pairs = Vec::with_capacity(tasks.len());
                    for t in tasks {
                        if let Ok(st) = app.task_store.state(&t.task_id) {
                            pairs.push((t, st));
                        }
                    }
                    let card = render_tasks_table(&pairs, &theme);
                    println!("{card}\n");
                    middle_content = card;
                }
                Err(err) => println!("  {} {}\n", theme.red_bold("✗ error:"), err),
            }
            continue;
        }
        if input == "/cost" {
            let lines = vec![
                format!("{}  {}", theme.dim("Session Runs   :"), stats.runs),
                format!("{}  {}", theme.dim("Total Steps    :"), stats.total_steps),
                format!(
                    "{}  {}",
                    theme.dim("Total Tool Runs:"),
                    stats.total_tool_calls
                ),
                format!(
                    "{}  {} in → {} out",
                    theme.dim("Session Tokens :"),
                    stats.total_input_tokens,
                    stats.total_output_tokens
                ),
                format!(
                    "{}  {}",
                    theme.dim("Session Cost   :"),
                    theme.green_bold(&format!("${:.6}", stats.total_cost_usd))
                ),
            ];
            println!(
                "{}\n",
                render_box(
                    &theme.accent("✻ Session Cost & Usage Meter"),
                    &lines,
                    64,
                    &theme
                )
            );
            continue;
        }
        if input == "/permissions" || input.starts_with("/permissions ") {
            let arg = input.strip_prefix("/permissions").unwrap_or("").trim();
            if arg.is_empty() {
                println!(
                    "  {} current mode = {} (use `/permissions deny|ask|auto` to switch)\n",
                    theme.accent("✻"),
                    approvals.mode().badge(&theme)
                );
            } else if let Some(new_mode) = PermissionMode::from_str(arg) {
                approvals.set_mode(new_mode);
                println!(
                    "  {} switched permission mode to {}\n",
                    theme.green_bold("✓"),
                    new_mode.badge(&theme)
                );
            } else {
                println!(
                    "  {} invalid permission mode {:?}; choose deny, ask, or auto\n",
                    theme.red_bold("✗"),
                    arg
                );
            }
            continue;
        }
        if input == "/constraints" || input.starts_with("/constraints ") {
            let arg = input.strip_prefix("/constraints").unwrap_or("").trim();
            if arg.is_empty() || arg == "list" {
                if session_constraints.is_empty() {
                    println!(
                        "  {} no session constraints pinned (use `/constraints add <text>`)\n",
                        theme.dim("◌")
                    );
                } else {
                    for (i, c) in session_constraints.iter().enumerate() {
                        println!("  {}. {}", i + 1, theme.magenta(c));
                    }
                    println!();
                }
            } else if arg == "clear" {
                session_constraints.clear();
                println!("  {} cleared session constraints\n", theme.green("✓"));
            } else if let Some(text) = arg.strip_prefix("add ") {
                if !text.trim().is_empty() {
                    session_constraints.push(text.trim().to_string());
                    println!(
                        "  {} pinned constraint: {}\n",
                        theme.green("✓"),
                        theme.magenta(text.trim())
                    );
                }
            } else {
                println!("  usage: /constraints [add <text> | clear | list]\n");
            }
            continue;
        }
        if input == "/accept" || input.starts_with("/accept ") {
            let arg = input.strip_prefix("/accept").unwrap_or("").trim();
            if arg.is_empty() || arg == "list" {
                if session_acceptance.is_empty() {
                    println!(
                        "  {} no session acceptance rules set (use `/accept add <rule>`)\n",
                        theme.dim("◌")
                    );
                } else {
                    for (i, r) in session_acceptance.iter().enumerate() {
                        println!("  {}. {}", i + 1, theme.yellow(r));
                    }
                    println!();
                }
            } else if arg == "clear" {
                session_acceptance.clear();
                println!("  {} cleared session acceptance rules\n", theme.green("✓"));
            } else if let Some(rule) = arg.strip_prefix("add ") {
                if !rule.trim().is_empty() {
                    session_acceptance.push(rule.trim().to_string());
                    println!(
                        "  {} added acceptance rule: {}\n",
                        theme.green("✓"),
                        theme.yellow(rule.trim())
                    );
                }
            } else {
                println!("  usage: /accept [add <rule> | clear | list]\n");
            }
            continue;
        }
        if input == "/run" || input.starts_with("/run ") {
            let name = input.strip_prefix("/run").unwrap_or("").trim();
            if name.is_empty() {
                println!("  usage: /run <{}>\n", FIXTURE_NAMES.join("|"));
                continue;
            }
            match run_fixture_interactive(
                app,
                name,
                &mut stats,
                &mut chat_history,
                &mut file_snapshots,
                &theme,
            ) {
                Ok((task_id, mid, notif)) => {
                    last_task = Some(task_id);
                    middle_content = mid;
                    notification = notif;
                    if !is_tty {
                        print_dashboard(
                            app,
                            approvals,
                            last_task.as_deref(),
                            model_mode,
                            session_constraints.len(),
                            chat_history.len(),
                            &stats,
                            &theme,
                        );
                    }
                }
                Err(err) => println!("  {} {err}\n", theme.red_bold("✗ error:")),
            }
            continue;
        }
        if let Some(rest) = input.strip_prefix("/resume") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            let target = match resolve_target_task(
                parts.first().copied().unwrap_or(""),
                last_task.as_deref(),
                app,
            ) {
                Ok(t) => t,
                Err(e) => {
                    println!("  {} {e}\n", theme.red_bold("✗ error:"));
                    continue;
                }
            };
            let fixture = parts.get(1).copied();
            match resume_interactive(app, &target, fixture, &mut stats, &theme) {
                Ok(task_id) => last_task = Some(task_id),
                Err(err) => println!("  {} {err}\n", theme.red_bold("✗ error:")),
            }
            continue;
        }
        if input == "/status" || input.starts_with("/status ") {
            let arg = input.strip_prefix("/status").unwrap_or("");
            match resolve_target_task(arg, last_task.as_deref(), app) {
                Ok(task_id) => match (
                    app.task_store.load(&task_id),
                    app.task_store.state(&task_id),
                ) {
                    (Ok(task), Ok(state)) => {
                        println!("{}\n", render_task_status_card(&task, &state, &theme));
                    }
                    (Err(e), _) | (_, Err(e)) => println!("  {} {e}\n", theme.red_bold("✗ error:")),
                },
                Err(e) => println!("  {e}\n"),
            }
            continue;
        }
        if input == "/inspect" || input.starts_with("/inspect ") {
            let arg = input.strip_prefix("/inspect").unwrap_or("");
            match resolve_target_task(arg, last_task.as_deref(), app) {
                Ok(task_id) => match (
                    app.task_store.load(&task_id),
                    app.task_store.state(&task_id),
                    app.ledger.events_for(&task_id, 0),
                ) {
                    (Ok(task), Ok(state), Ok(events)) => {
                        println!(
                            "{}\n",
                            render_inspect_timeline(&task, &state, &events, &theme)
                        );
                    }
                    _ => println!("  {} unknown task {task_id}\n", theme.red_bold("✗ error:")),
                },
                Err(e) => println!("  {e}\n"),
            }
            continue;
        }
        if let Some(q) = input
            .strip_prefix("/search ")
            .or_else(|| input.strip_prefix("/context search "))
        {
            match app.retriever.search(q.trim(), None, None, 5) {
                Ok(hits) => println!("{}\n", render_context_search(q.trim(), &hits, &theme)),
                Err(err) => println!("  {} {err}\n", theme.red_bold("✗ error:")),
            }
            continue;
        }
        if let Some(r) = input
            .strip_prefix("/read ")
            .or_else(|| input.strip_prefix("/context read "))
        {
            match app.retriever.read(r.trim(), None, 600) {
                Ok(payload) => println!("{}\n", render_context_read(&payload, &theme)),
                Err(err) => println!("  {} {err}\n", theme.red_bold("✗ error:")),
            }
            continue;
        }
        if input == "/trace" || input.starts_with("/trace ") {
            let arg = input.strip_prefix("/trace").unwrap_or("");
            match resolve_target_task(arg, last_task.as_deref(), app) {
                Ok(task_id) => match app.export_trace(&task_id) {
                    Ok(entries) => {
                        let replay = replay_trace(&entries);
                        println!(
                            "{}\n",
                            render_trace_card(&task_id, &entries, &replay, &theme)
                        );
                    }
                    Err(err) => println!("  {} {err}\n", theme.red_bold("✗ error:")),
                },
                Err(e) => println!("  {e}\n"),
            }
            continue;
        }
        if input == "/wiki" || input.starts_with("/wiki ") {
            let arg = input.strip_prefix("/wiki").unwrap_or("").trim();
            let (compile_first, target_arg) = if let Some(rest) = arg.strip_prefix("compile") {
                (true, rest.trim())
            } else {
                (false, arg)
            };
            match resolve_target_task(target_arg, last_task.as_deref(), app) {
                Ok(task_id) => {
                    if compile_first {
                        let _ = app.wiki.compile_task(&task_id);
                    }
                    match (
                        app.wiki.summary(&task_id),
                        app.wiki.nodes_for(&task_id),
                        app.wiki.edges_for(&task_id),
                    ) {
                        (Ok(summary), Ok(nodes), Ok(edges)) => {
                            println!(
                                "{}\n",
                                render_wiki_view(&task_id, &summary, &nodes, &edges, &theme)
                            );
                        }
                        _ => println!(
                            "  {} failed to read Wiki for {task_id}\n",
                            theme.red_bold("✗ error:")
                        ),
                    }
                }
                Err(e) => println!("  {e}\n"),
            }
            continue;
        }
        if input == "/skills" || input.starts_with("/skills ") {
            let arg = input.strip_prefix("/skills").unwrap_or("").trim();
            if arg.is_empty() {
                let manifests = app.skills.discover("", 50);
                println!("{}\n", render_skills_list(&manifests, &theme));
            } else {
                match app.skills.activate(arg) {
                    Ok(act) => println!("{}\n", render_skill_detail(&act, &theme)),
                    Err(err) => println!("  {} {err}\n", theme.red_bold("✗ error:")),
                }
            }
            continue;
        }
        if input == "/verify" || input.starts_with("/verify ") {
            let arg = input.strip_prefix("/verify").unwrap_or("").trim();
            let parts: Vec<&str> = arg.split_whitespace().collect();
            let target = match resolve_target_task(
                parts.first().copied().unwrap_or(""),
                last_task.as_deref(),
                app,
            ) {
                Ok(t) => t,
                Err(e) => {
                    println!("  {e}\n");
                    continue;
                }
            };
            let explicit_criteria: Vec<String> =
                parts.into_iter().skip(1).map(|s| s.to_string()).collect();
            match app.task_store.load(&target) {
                Ok(task) => {
                    let criteria = if explicit_criteria.is_empty() {
                        task.acceptance_rules
                    } else {
                        explicit_criteria
                    };
                    match app.verifier.verify(&target, &criteria) {
                        Ok(results) => {
                            println!("{}\n", render_verification_card(&target, &results, &theme))
                        }
                        Err(err) => println!("  {} {err}\n", theme.red_bold("✗ error:")),
                    }
                }
                Err(err) => println!("  {} {err}\n", theme.red_bold("✗ error:")),
            }
            continue;
        }

        let goal = input.strip_prefix("/goal ").unwrap_or(input).to_string();
        match run_goal_interactive(
            app,
            &goal,
            &session_constraints,
            &session_acceptance,
            model_mode,
            &mut stats,
            &mut chat_history,
            &mut file_snapshots,
            &theme,
        ) {
            Ok((task_id, mid, notif)) => {
                last_task = Some(task_id);
                middle_content = mid;
                notification = notif;
                if !is_tty {
                    print_dashboard(
                        app,
                        approvals,
                        last_task.as_deref(),
                        model_mode,
                        session_constraints.len(),
                        chat_history.len(),
                        &stats,
                        &theme,
                    );
                }
            }
            Err(err) => println!("  {} {err}\n", theme.red_bold("✗ error:")),
        }
    }
    println!("bye.");
    Ok(0)
}

fn run_fixture_interactive(
    app: &App,
    name: &str,
    stats: &mut SessionStats,
    chat_history: &mut Vec<ChatTurn>,
    file_snapshots: &mut std::collections::BTreeMap<String, (u32, String)>,
    theme: &Theme,
) -> Result<(String, String, String), String> {
    let spec = get_fixture(name).map_err(|e| e.to_string())?;
    let turn_idx = chat_history.len() + 1;
    let user_bubble =
        render_user_chat_bubble(turn_idx, &format!("/run {} — {}", name, spec.goal), theme);
    println!("\n{user_bubble}");
    let task = app
        .create_task(
            &spec.goal,
            spec.constraints.clone(),
            spec.acceptance_rules.clone(),
            Some(spec.budget.clone()),
        )
        .map_err(|e| e.to_string())?;
    if let Some(setup) = spec.setup {
        setup(app, &task).map_err(|e| e.to_string())?;
    }
    let mut model = (spec.build_model)();
    let mut observer = ClaudeStepObserver::with_snapshots(
        TeeWriter::new(),
        *theme,
        std::mem::take(file_snapshots),
    );
    let result = app
        .run_observed(model.as_mut(), &task.task_id, 50, &mut observer)
        .map_err(|e| e.to_string())?;
    *file_snapshots = observer.take_snapshots();
    let step_stream = observer.into_inner().into_string();
    stats.record(&result);
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
    chat_history.push(ChatTurn {
        turn_index: turn_idx,
        task_id: result.task_id.clone(),
        user_message: format!("/run {name}"),
        ai_response: result
            .answer
            .clone()
            .unwrap_or_else(|| format!("Status: {}", result.status.as_str())),
        status: result.status,
        steps: result.steps,
        tool_calls,
        cost_usd,
    });
    let summary_str = render_run_summary(&result, theme);
    print!("{summary_str}");
    let mid = format!("{user_bubble}\n{step_stream}{summary_str}");
    let notif = format!(
        "Task {} {} ({} steps · ${cost_usd:.4})",
        task.task_id,
        result.status.as_str(),
        result.steps
    );
    Ok((task.task_id, mid, notif))
}

fn resume_interactive(
    app: &App,
    task_id: &str,
    fixture: Option<&str>,
    stats: &mut SessionStats,
    theme: &Theme,
) -> Result<String, String> {
    let state = app.resume_state(task_id).map_err(|e| e.to_string())?;
    println!(
        "  {} resumed {} at seq {} (status={})",
        theme.cyan("↺"),
        task_id,
        state.last_seq,
        state.status.as_str()
    );
    let mut model: Box<dyn sfai::core::model_adapter::ModelAdapter> = match fixture {
        Some(name) => (get_fixture(name).map_err(|e| e.to_string())?.build_model)(),
        None => Box::new(OpenAICompatibleAdapter::from_env()),
    };
    let mut observer = ClaudeStepObserver::new(std::io::stdout(), *theme);
    let result = app
        .run_observed(model.as_mut(), task_id, 50, &mut observer)
        .map_err(|e| e.to_string())?;
    stats.record(&result);
    print!("{}", render_run_summary(&result, theme));
    Ok(result.task_id)
}

#[allow(clippy::too_many_arguments)]
fn run_goal_interactive(
    app: &App,
    goal: &str,
    constraints: &[String],
    acceptance_rules: &[String],
    model_mode: ChatModelMode,
    stats: &mut SessionStats,
    chat_history: &mut Vec<ChatTurn>,
    file_snapshots: &mut std::collections::BTreeMap<String, (u32, String)>,
    theme: &Theme,
) -> Result<(String, String, String), String> {
    let turn_idx = chat_history.len() + 1;
    let user_bubble = render_user_chat_bubble(turn_idx, goal, theme);
    println!("\n{user_bubble}");

    let task = app
        .create_task(
            goal,
            constraints.to_vec(),
            acceptance_rules.to_vec(),
            Some(Budget::default()),
        )
        .map_err(|e| e.to_string())?;

    let mut model: Box<dyn sfai::core::model_adapter::ModelAdapter> = match model_mode {
        ChatModelMode::Offline => Box::new(ConversationalDemoModel::new()),
        ChatModelMode::Online => Box::new(OpenAICompatibleAdapter::from_env()),
        ChatModelMode::Auto => {
            let online = OpenAICompatibleAdapter::from_env();
            if !online.base_url.trim().is_empty() && !online.model_name.trim().is_empty() {
                Box::new(online)
            } else {
                Box::new(ConversationalDemoModel::new())
            }
        }
    };

    let mut observer = ClaudeStepObserver::with_snapshots(
        TeeWriter::new(),
        *theme,
        std::mem::take(file_snapshots),
    );
    match app.run_observed(model.as_mut(), &task.task_id, 50, &mut observer) {
        Ok(result) => {
            *file_snapshots = observer.take_snapshots();
            let step_stream = observer.into_inner().into_string();
            stats.record(&result);
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
            chat_history.push(ChatTurn {
                turn_index: turn_idx,
                task_id: result.task_id.clone(),
                user_message: goal.to_string(),
                ai_response: result
                    .answer
                    .clone()
                    .unwrap_or_else(|| format!("Task status: {}", result.status.as_str())),
                status: result.status,
                steps: result.steps,
                tool_calls,
                cost_usd,
            });
            let summary_str = render_run_summary(&result, theme);
            print!("{summary_str}");
            let mid = format!("{user_bubble}\n{step_stream}{summary_str}");
            let notif = format!(
                "Task {} {} ({} steps · ${cost_usd:.4})",
                result.task_id,
                result.status.as_str(),
                result.steps
            );
            Ok((result.task_id, mid, notif))
        }
        Err(err) => Err(format!(
            "{err}\n  (hint: configure SFAI_MODEL_BASE_URL / SFAI_MODEL_API_KEY / SFAI_MODEL_NAME, or use `/model offline`)"
        )),
    }
}
