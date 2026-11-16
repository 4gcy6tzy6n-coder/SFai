//! CLI: run, resume, status, inspect, context, capabilities, verify, export-trace.

use clap::{Parser, Subcommand};
use serde_json::json;
use std::path::PathBuf;
use std::process::ExitCode;

use sfai::app::build_app;
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
    #[command(subcommand)]
    cmd: Cmd,
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
    let app = build_app(&cli.storage).map_err(|e| e.to_string())?;
    match cli.cmd {
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
    }
}
