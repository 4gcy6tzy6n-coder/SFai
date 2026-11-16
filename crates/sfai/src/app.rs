//! Composition root (plan §9 app.py): wires the six planes together.

use crate::authority::approvals::{ApprovalProvider, DenyAllApprovals};
use crate::authority::tasks::TaskStore;
use crate::capabilities::registry::CapabilityRegistry;
use crate::capabilities::skills::SkillRegistry;
use crate::context::index::ContextIndex;
use crate::context::retrieval::ContextRetriever;
use crate::context::wiki::{StoreResolver, WikiCompiler};
use crate::contracts::{Budget, Task, TaskState, TaskStatus};
use crate::core::context_envelope::EnvelopeBuilder;
use crate::core::decision_kernel::{DecisionKernel, TaskRunResult};
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
use std::rc::Rc;

pub struct App {
    pub storage_root: PathBuf,
    pub db: Database,
    pub ledger: EventLedger,
    pub objects: ObjectStore,
    pub index: ContextIndex,
    pub retriever: ContextRetriever,
    pub registry: Rc<CapabilityRegistry>,
    pub skills: SkillRegistry,
    pub policy: PolicyEngine,
    pub approvals: Rc<dyn ApprovalProvider>,
    pub broker: ToolBroker,
    pub envelope_builder: EnvelopeBuilder,
    pub verifier: VerificationEngine,
    pub acceptance: AcceptanceEvaluator,
    pub task_store: TaskStore,
    pub wiki: WikiCompiler,
    pub trace_dir: PathBuf,
    pub side_effect_log: Rc<std::cell::RefCell<Vec<String>>>,
    pub tool_counts: Rc<std::cell::RefCell<std::collections::BTreeMap<String, u32>>>,
}

pub fn build_app(storage_root: impl AsRef<Path>) -> Result<App> {
    build_app_with_approvals(storage_root, Rc::new(DenyAllApprovals))
}

pub fn build_app_with_approvals(
    storage_root: impl AsRef<Path>,
    approvals: Rc<dyn ApprovalProvider>,
) -> Result<App> {
    build_app_with_options(storage_root, approvals, None)
}

/// Build with an explicit Skill package directory (defaults to `<storage>/skills`).
pub fn build_app_with_options(
    storage_root: impl AsRef<Path>,
    approvals: Rc<dyn ApprovalProvider>,
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
    let registry = Rc::new(registry);
    let policy = PolicyEngine;
    let broker = ToolBroker::new(
        registry.clone(),
        policy.clone(),
        approvals.clone(),
        objects.clone(),
        ledger.clone(),
    );
    let envelope_builder =
        EnvelopeBuilder::new(index.clone(), retriever.clone(), Some(ledger.clone()));
    let verifier = VerificationEngine::new(objects.clone());
    let acceptance = AcceptanceEvaluator::new(verifier.clone());
    let task_store = TaskStore::new(db.clone(), ledger.clone());
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
        wiki,
        trace_dir,
        side_effect_log,
        tool_counts,
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
        let result = kernel.run_task(task_id, max_steps)?;
        // M4: incremental Wiki compilation is best-effort and never affects
        // task authority (it writes only to wiki tables).
        let _ = self.wiki.compile_task(task_id);
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
