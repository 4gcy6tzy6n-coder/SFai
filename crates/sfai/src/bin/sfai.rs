//! CLI: run, resume, status, inspect, context, capabilities, verify, export-trace.

use clap::{Parser, Subcommand};
use serde_json::json;
use std::path::PathBuf;
use std::process::ExitCode;

use sfai::app::{build_app_with_options, App};
use sfai::contracts::Budget;
use sfai::fixtures::{get_fixture, FIXTURE_NAMES};
use sfai::integrations::providers::openai_compat::OpenAICompatibleAdapter;

#[derive(Parser)]
#[command(
    name = "sfai",
    about = "System for future ai CLI (offline fixtures supported)"
)]
struct Cli {
    #[arg(long, default_value = ".sfai", global = true)]
    storage: PathBuf,
    #[arg(long, global = true)]
    skills: Option<PathBuf>,
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
        #[arg(long, default_value_t = 50)]
        max_steps: usize,
    },
    /// Resume an interrupted task
    Resume {
        task_id: String,
        #[arg(long)]
        fixture: Option<String>,
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

fn dispatch(cli: Cli) -> Result<u8, String> {
    let app: App = build_app_with_options(
        &cli.storage,
        std::rc::Rc::new(sfai::authority::approvals::DenyAllApprovals),
        cli.skills.clone(),
    )
    .map_err(|e| e.to_string())?;
    let Some(cmd) = cli.cmd else {
        return interactive(&app);
    };
    match cmd {
        Cmd::Run {
            fixture,
            goal,
            max_steps,
        } => {
            let task = if let Some(name) = fixture {
                let spec = get_fixture(&name).map_err(|e| e.to_string())?;
                let task = app
                    .create_task(
                        &spec.goal,
                        spec.constraints.clone(),
                        spec.acceptance_rules.clone(),
                        Some(spec.budget.clone()),
                    )
                    .map_err(|e| e.to_string())?;
                if let Some(setup) = spec.setup {
                    setup(&app, &task).map_err(|e| e.to_string())?;
                }
                (task, (spec.build_model)())
            } else if let Some(goal) = goal {
                let task = app
                    .create_task(&goal, vec![], vec![], Some(Budget::default()))
                    .map_err(|e| e.to_string())?;
                (
                    task,
                    Box::new(OpenAICompatibleAdapter::from_env())
                        as Box<dyn sfai::core::model_adapter::ModelAdapter>,
                )
            } else {
                return Err(format!(
                    "provide --fixture <{}|...> or --goal TEXT",
                    FIXTURE_NAMES.join("|")
                ));
            };
            let (task, mut model) = task;
            let result = app
                .run(model.as_mut(), &task.task_id, max_steps)
                .map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "task_id": result.task_id,
                    "status": result.status.as_str(),
                    "answer": result.answer,
                    "steps": result.steps,
                    "cost": result.cost,
                    "open_issues": result.open_issues,
                }))
                .map_err(|e| e.to_string())?
            );
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
            let result = app
                .run(model.as_mut(), &task_id, 50)
                .map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "task_id": result.task_id,
                    "status": result.status.as_str(),
                    "steps": result.steps,
                }))
                .map_err(|e| e.to_string())?
            );
            Ok(if result.status == sfai::contracts::TaskStatus::Completed {
                0
            } else {
                1
            })
        }
        Cmd::Status { task_id } => {
            let task = app.task_store.load(&task_id).map_err(|e| e.to_string())?;
            let state = app.task_store.state(&task_id).map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({"task": task, "state": state}))
                    .map_err(|e| e.to_string())?
            );
            Ok(0)
        }
        Cmd::Inspect { task_id } => {
            let state = app.task_store.state(&task_id).map_err(|e| e.to_string())?;
            let events = app
                .ledger
                .events_for(&task_id, 0)
                .map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({"state": state, "events": events}))
                    .map_err(|e| e.to_string())?
            );
            Ok(0)
        }
        Cmd::Context { ctx } => {
            match ctx {
                CtxCmd::Search { query, limit } => {
                    let hits = app
                        .retriever
                        .search(&query, None, None, limit.unwrap_or(5))
                        .map_err(|e| e.to_string())?;
                    println!(
                    "{}",
                    serde_json::to_string_pretty(&json!(hits
                        .iter()
                        .map(|h| json!({"ref_id": h.ref_id, "title": h.title, "snippet": h.snippet}))
                        .collect::<Vec<_>>()))
                    .map_err(|e| e.to_string())?
                );
                    Ok(0)
                }
                CtxCmd::Read { ref_id, max_tokens } => {
                    let payload = app
                        .retriever
                        .read(&ref_id, None, max_tokens.unwrap_or(600))
                        .map_err(|e| e.to_string())?;
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?
                    );
                    Ok(0)
                }
            }
        }
        Cmd::Capabilities => {
            let manifests = app.registry.discover("", 50, None);
            println!(
                "{}",
                serde_json::to_string_pretty(&manifests).map_err(|e| e.to_string())?
            );
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
            println!(
                "{}",
                serde_json::to_string_pretty(&results).map_err(|e| e.to_string())?
            );
            Ok(if all_pass { 0 } else { 1 })
        }
        Cmd::ExportTrace { task_id } => {
            let entries = app.export_trace(&task_id).map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?
            );
            Ok(0)
        }
        Cmd::Skills { skills } => match skills {
            SkillsCmd::List { query } => {
                let manifests = app.skills.discover(&query, 50);
                println!(
                    "{}",
                    serde_json::to_string_pretty(&manifests).map_err(|e| e.to_string())?
                );
                Ok(0)
            }
            SkillsCmd::Show { name } => {
                let activation = app.skills.activate(&name).map_err(|e| e.to_string())?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&activation).map_err(|e| e.to_string())?
                );
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
                println!(
                    "{}",
                    serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?
                );
                Ok(0)
            }
        },
    }
}

// ---- interactive session (bare `sfai`) ----

const HELP: &str = "\nCommands:\n  /run <fixture>   run an offline fixture (direct-answer|tool-use|failed-tool|approval|context-retrieval)\n  /fixtures        list fixtures\n  /goal <text>     run a goal with the configured provider (or just type the goal)\n  /status          last task status\n  /trace <task>    export trace for a task\n  /wiki <task>     derived Wiki summary\n  /skills          list installed skills\n  /help            this help\n  /quit            exit\n";

fn interactive(app: &App) -> Result<u8, String> {
    use std::io::{BufRead, Write};
    println!("System for future ai — interactive session");
    println!("Type a goal and press Enter to run it, or /help for commands.");
    println!("Offline demo: /run tool-use\n");

    let mut last_task: Option<String> = None;
    let stdin = std::io::stdin();
    loop {
        print!("sfai> ");
        std::io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        let read = stdin
            .lock()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if read == 0 {
            println!();
            break;
        }
        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if matches!(input, "/quit" | "/exit" | "quit" | "exit") {
            break;
        }
        if input == "/help" {
            println!("{HELP}");
            continue;
        }
        if input == "/fixtures" {
            println!("fixtures: {}", FIXTURE_NAMES.join(", "));
            continue;
        }
        if let Some(name) = input.strip_prefix("/run ") {
            match run_fixture_interactive(app, name.trim()) {
                Ok(task_id) => last_task = Some(task_id),
                Err(err) => println!("error: {err}"),
            }
            continue;
        }
        if input == "/status" {
            match &last_task {
                Some(task_id) => match app.task_store.state(task_id) {
                    Ok(state) => println!(
                        "  task={task_id}  status={}  outputs={}  issues={}",
                        state.status.as_str(),
                        state.completed_outputs.len(),
                        state.open_issues.len()
                    ),
                    Err(err) => println!("error: {err}"),
                },
                None => println!("no task yet; run a goal or /run <fixture>"),
            }
            continue;
        }
        if let Some(task_id) = input.strip_prefix("/trace ") {
            match app.export_trace(task_id.trim()) {
                Ok(entries) => println!("  trace entries: {}", entries.len()),
                Err(err) => println!("error: {err}"),
            }
            continue;
        }
        if let Some(task_id) = input.strip_prefix("/wiki ") {
            match app.wiki.summary(task_id.trim()) {
                Ok(summary) => println!(
                    "{}",
                    serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?
                ),
                Err(err) => println!("error: {err}"),
            }
            continue;
        }
        if input == "/skills" {
            let manifests = app.skills.discover("", 50);
            println!(
                "{}",
                serde_json::to_string_pretty(&manifests).map_err(|e| e.to_string())?
            );
            continue;
        }
        let goal = input.strip_prefix("/goal ").unwrap_or(input).to_string();
        match run_goal_interactive(app, &goal) {
            Ok(task_id) => last_task = Some(task_id),
            Err(err) => println!("error: {err}"),
        }
    }
    println!("bye.");
    Ok(0)
}

fn run_fixture_interactive(app: &App, name: &str) -> Result<String, String> {
    let spec = get_fixture(name).map_err(|e| e.to_string())?;
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
    let result = app
        .run(model.as_mut(), &task.task_id, 50)
        .map_err(|e| e.to_string())?;
    print_summary(&result);
    Ok(task.task_id)
}

fn run_goal_interactive(app: &App, goal: &str) -> Result<String, String> {
    let task = app
        .create_task(goal, vec![], vec![], Some(Budget::default()))
        .map_err(|e| e.to_string())?;
    let mut model = OpenAICompatibleAdapter::from_env();
    match app.run(&mut model, &task.task_id, 50) {
        Ok(result) => {
            print_summary(&result);
            Ok(result.task_id)
        }
        Err(err) => Err(format!(
            "{err}\n(hint: configure SFAI_MODEL_BASE_URL / SFAI_MODEL_API_KEY / SFAI_MODEL_NAME, or use /run <fixture> offline)"
        )),
    }
}

fn print_summary(result: &sfai::core::decision_kernel::TaskRunResult) {
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
    println!(
        "  task={}  status={}  steps={}  tool_calls={}  cost=${cost_usd:.6}",
        result.task_id,
        result.status.as_str(),
        result.steps,
        tool_calls
    );
}
