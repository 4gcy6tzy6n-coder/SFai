//! Composition root (plan §9 app.py): wires the six planes together.

use crate::authority::approvals::{ApprovalProvider, DenyAllApprovals};
use crate::authority::tasks::TaskStore;
use crate::capabilities::registry::CapabilityRegistry;
use crate::capabilities::skills::SkillRegistry;
use crate::context::index::ContextIndex;
use crate::context::memory::MemoryStore;
use crate::context::retrieval::ContextRetriever;
use crate::context::wiki::{StoreResolver, WikiCompiler};
use crate::contracts::{Budget, Task, TaskState, TaskStatus};
use crate::core::context_envelope::EnvelopeBuilder;
use crate::core::decision_kernel::{DecisionKernel, KernelObserver, NoopObserver, TaskRunResult};
use crate::core::model_adapter::ModelAdapter;
use crate::core::termination::AcceptanceEvaluator;
use crate::error::Result;
use crate::ids::new_task_id;
use crate::integrations::fake_tools::{register_fixture_tools, FixtureWiring};
use crate::runtime::broker::ToolBroker;
use crate::runtime::policy::PolicyEngine;
use crate::storage::db::Database;
use crate::storage::event_ledger::EventLedger;
use crate::storage::objects::ObjectStore;
use crate::telemetry::tracing::{export_trace, Tracer};
use crate::verify::engine::VerificationEngine;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub struct App {
    pub storage_root: PathBuf,
    pub db: Database,
    pub ledger: EventLedger,
    pub objects: ObjectStore,
    pub index: ContextIndex,
    pub retriever: ContextRetriever,
    pub registry: Arc<CapabilityRegistry>,
    pub skills: SkillRegistry,
    pub policy: PolicyEngine,
    pub approvals: Arc<dyn ApprovalProvider>,
    pub broker: ToolBroker,
    pub envelope_builder: EnvelopeBuilder,
    pub verifier: VerificationEngine,
    pub acceptance: AcceptanceEvaluator,
    pub task_store: TaskStore,
    pub memory: MemoryStore,
    pub wiki: WikiCompiler,
    pub trace_dir: PathBuf,
    pub side_effect_log: Arc<Mutex<Vec<String>>>,
    pub tool_counts: Arc<Mutex<std::collections::BTreeMap<String, u32>>>,
    /// Ablation switches (default: full framework behavior).
    pub enforce_acceptance: bool,
    pub wiki_compile_enabled: bool,
}

pub fn build_app(storage_root: impl AsRef<Path>) -> Result<App> {
    build_app_with_approvals(storage_root, Arc::new(DenyAllApprovals))
}

pub fn build_app_with_approvals(
    storage_root: impl AsRef<Path>,
    approvals: Arc<dyn ApprovalProvider>,
) -> Result<App> {
    build_app_with_options(storage_root, approvals, None)
}

/// Build with an explicit Skill package directory (defaults to `<storage>/skills`).
pub fn build_app_with_options(
    storage_root: impl AsRef<Path>,
    approvals: Arc<dyn ApprovalProvider>,
    skills_dir: Option<PathBuf>,
) -> Result<App> {
    let storage_root = storage_root.as_ref().to_path_buf();
    let skills_dir = skills_dir.unwrap_or_else(|| storage_root.join("skills"));
    let skills = SkillRegistry::load_from_dir(&skills_dir)?;
    std::fs::create_dir_all(&storage_root)?;
    let db = Database::open(storage_root.join("sfai.sqlite3"))?;
    let ledger = EventLedger::new(db.clone());
    let objects = ObjectStore::new(db.clone(), storage_root.join("blobs"))?;
    let index = ContextIndex::new(db.clone());
    let retriever = ContextRetriever::new(db.clone(), objects.clone());
    let mut registry = CapabilityRegistry::new();
    let FixtureWiring {
        side_effect_log,
        tool_counts,
    } = register_fixture_tools(&mut registry, &retriever);
    let registry = Arc::new(registry);
    let policy = PolicyEngine;
    let broker = ToolBroker::new(
        registry.clone(),
        policy.clone(),
        approvals.clone(),
        objects.clone(),
        ledger.clone(),
        storage_root.join("workspaces"),
    );
    let mut envelope_builder =
        EnvelopeBuilder::new(index.clone(), retriever.clone(), Some(ledger.clone()));
    envelope_builder.memory = Some(MemoryStore::new(db.clone()));
    let verifier =
        VerificationEngine::with_workspace(objects.clone(), storage_root.join("workspaces"));
    let acceptance = AcceptanceEvaluator::new(verifier.clone());
    let task_store = TaskStore::new(db.clone(), ledger.clone());
    let memory = MemoryStore::new(db.clone());
    let wiki = WikiCompiler::new(
        db.clone(),
        ledger.clone(),
        Box::new(StoreResolver {
            db: db.clone(),
            objects: objects.clone(),
        }),
    );
    let trace_dir = storage_root.join("traces");
    Ok(App {
        storage_root,
        db,
        ledger,
        objects,
        index,
        retriever,
        registry,
        skills,
        policy,
        approvals,
        broker,
        envelope_builder,
        verifier,
        acceptance,
        task_store,
        memory,
        wiki,
        trace_dir,
        side_effect_log,
        tool_counts,
        enforce_acceptance: true,
        wiki_compile_enabled: true,
    })
}

impl App {
    pub fn create_task(
        &self,
        goal: &str,
        constraints: Vec<String>,
        acceptance_rules: Vec<String>,
        budget: Option<Budget>,
    ) -> Result<Task> {
        let task = Task {
            task_id: new_task_id(),
            user_id: "user".into(),
            tenant_id: "default".into(),
            goal: goal.to_string(),
            constraints,
            acceptance_rules,
            status: TaskStatus::Created,
            budget: budget.unwrap_or_default(),
            version: 1,
        };
        self.task_store.create(&task)?;
        Ok(task)
    }

    pub fn run(
        &self,
        model: &mut dyn ModelAdapter,
        task_id: &str,
        max_steps: usize,
    ) -> Result<TaskRunResult> {
        let mut noop = NoopObserver;
        self.run_observed(model, task_id, max_steps, &mut noop)
    }

    pub fn run_observed(
        &self,
        model: &mut dyn ModelAdapter,
        task_id: &str,
        max_steps: usize,
        observer: &mut dyn KernelObserver,
    ) -> Result<TaskRunResult> {
        self.run_observed_with_cancel(model, task_id, max_steps, observer, None)
    }

    pub fn run_observed_with_cancel(
        &self,
        model: &mut dyn ModelAdapter,
        task_id: &str,
        max_steps: usize,
        observer: &mut dyn KernelObserver,
        cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> Result<TaskRunResult> {
        let tracer = Tracer::new(&self.trace_dir)?;
        let mut kernel = DecisionKernel::new(
            model,
            &self.broker,
            &self.task_store,
            &self.ledger,
            &self.envelope_builder,
            &self.acceptance,
            &self.verifier,
            &self.retriever,
            self.registry.as_ref(),
            &self.skills,
            &tracer,
        );
        kernel = kernel.with_acceptance_enforcement(self.enforce_acceptance);
        if let Some(flag) = cancel {
            kernel = kernel.with_cancel_flag(flag);
        }
        let result = kernel.run_task_observed(task_id, max_steps, observer)?;
        // M4: incremental Wiki compilation is best-effort and never affects
        // task authority (it writes only to wiki tables).
        if self.wiki_compile_enabled {
            let _ = self.wiki.compile_task(task_id);
        }
        // Auto-memory (Claude Code style): every finished run leaves a
        // task_summary note in the memory layer.
        // "Remember the way of every task" — terminal runs (including blocked
        // ones) all leave a Way entry; mistakes live in the failed runs.
        if matches!(
            result.status,
            crate::contracts::TaskStatus::Completed
                | crate::contracts::TaskStatus::Blocked
                | crate::contracts::TaskStatus::Failed
        ) {
            let state = self.task_store.state(task_id)?;
            let task = self.task_store.load(task_id)?;
            let _ = self.memory.auto_summarize_task(
                task_id,
                &task.goal,
                result.status.as_str(),
                &state.completed_outputs,
                state.open_issues.len(),
            );
            // Way layer: auto-extract what went wrong from the run history —
            // mistakes are the emphasized part of the agent's memory.
            let mut mistakes: Vec<String> = Vec::new();
            for event in self.ledger.events_for(task_id, 0)? {
                if event.event_type == "task.error_opened" {
                    if let Some(code) = event.payload.get("error_code").and_then(|c| c.as_str()) {
                        mistakes.push(format!("tool error: {code}"));
                    }
                }
            }
            for issue in &state.open_issues {
                if let Some(code) = issue.get("error_code").and_then(|c| c.as_str()) {
                    mistakes.push(format!("acceptance issue: {code}"));
                }
            }
            mistakes.sort();
            mistakes.dedup();
            let wins: Vec<String> = state.completed_outputs.clone();
            let lesson = if mistakes.is_empty() {
                "clean run: keep the current method".to_string()
            } else {
                format!("avoid repeating: {}", mistakes.join("; "))
            };
            let method = format!(
                "self-routed Decide-Act loop (status={})",
                result.status.as_str()
            );
            if let Ok(note) = self
                .memory
                .auto_way_from_run(task_id, &method, &wins, &mistakes, &lesson)
            {
                // Wiki bridge: index → wiki → way.
                let _ = self
                    .wiki
                    .ingest_way(task_id, &note.note_id, &note.title, &note.body);
            }
        }
        tracer.record(task_id, "cost", result.cost.clone())?;
        tracer.record(
            task_id,
            "run_complete",
            json!({"status": result.status.as_str(), "steps": result.steps}),
        )?;
        Ok(result)
    }

    /// Crash recovery: rebuild authoritative state from snapshot + events.
    pub fn resume_state(&self, task_id: &str) -> Result<TaskState> {
        self.task_store.resume(task_id)
    }

    pub fn export_trace(&self, task_id: &str) -> Result<Vec<Value>> {
        export_trace(&self.trace_dir, task_id)
    }
}
