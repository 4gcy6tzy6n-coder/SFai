//! M5 acceptance tests: Skill progressive disclosure.

mod common;

use common::temp_root;
use serde_json::json;
use sfai::app::build_app_with_options;
use sfai::authority::approvals::DenyAllApprovals;
use sfai::contracts::{ActionIntent, ContextEnvelope, IntentKind, ModelResponse, ModelUsage};
use sfai::core::action_intents::intent;
use sfai::core::model_adapter::{ModelAdapter, ModelError};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

struct CaptureModel {
    envelopes: Rc<RefCell<Vec<ContextEnvelope>>>,
    steps: Vec<ActionIntent>,
    cursor: usize,
}

impl CaptureModel {
    fn new(steps: Vec<ActionIntent>, envelopes: Rc<RefCell<Vec<ContextEnvelope>>>) -> Self {
        Self {
            envelopes,
            steps,
            cursor: 0,
        }
    }
}

impl ModelAdapter for CaptureModel {
    fn model_id(&self) -> String {
        "fake/capture".into()
    }

    fn decide(&mut self, envelope: &ContextEnvelope) -> Result<ModelResponse, ModelError> {
        self.envelopes.borrow_mut().push(envelope.clone());
        if self.cursor >= self.steps.len() {
            return Err(ModelError::Provider("capture model exhausted".into()));
        }
        let step = self.steps[self.cursor].clone();
        self.cursor += 1;
        Ok(ModelResponse {
            model_id: "fake/capture".into(),
            intent: step,
            usage: ModelUsage::default(),
            latency_ms: 0.0,
        })
    }
}

fn write_skill(root: &Path, name: &str, instructions: &str) {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).expect("skill dir");
    std::fs::write(
        dir.join("SKILL.md"),
        format!(
            "---\nname: {name}\ndescription: {name} helper skill\nversion: 0.2.0\n---\n{instructions}"
        ),
    )
    .expect("skill file");
}

fn activate(skill: &str) -> ActionIntent {
    intent(
        IntentKind::Activate,
        "unknown",
        "load_skill",
        json!({"skill_id": skill}),
        vec![],
        None,
    )
}

fn respond(text: &str) -> ActionIntent {
    intent(
        IntentKind::Respond,
        "unknown",
        "direct_answer",
        json!({"text": text}),
        vec![],
        None,
    )
}

#[test]
fn inactive_skill_never_reaches_the_envelope() {
    let storage = temp_root();
    let skills_dir = temp_root();
    write_skill(skills_dir.path(), "research", "RESEARCH-ONLY-INSTRUCTIONS");
    let app = build_app_with_options(
        storage.path(),
        Rc::new(DenyAllApprovals),
        Some(skills_dir.path().to_path_buf()),
    )
    .expect("app");
    let task = app
        .create_task("answer directly", vec![], vec![], None)
        .expect("task");

    let captures = Rc::new(RefCell::new(Vec::new()));
    let mut model = CaptureModel::new(vec![respond("hi")], captures.clone());
    app.run(&mut model, &task.task_id, 10).expect("run");

    let envelopes = captures.borrow();
    assert_eq!(envelopes.len(), 1);
    assert!(envelopes[0].active_skills.is_empty());
    let raw = serde_json::to_string(&envelopes[0]).expect("serialize envelope");
    assert!(
        !raw.contains("RESEARCH-ONLY-INSTRUCTIONS"),
        "unused Skill instructions must never enter the prompt"
    );
}

#[test]
fn activation_loads_instructions_into_working_set() {
    let storage = temp_root();
    let skills_dir = temp_root();
    write_skill(skills_dir.path(), "research", "Step 1: cite sources.");
    let app = build_app_with_options(
        storage.path(),
        Rc::new(DenyAllApprovals),
        Some(skills_dir.path().to_path_buf()),
    )
    .expect("app");
    let task = app
        .create_task("research a topic", vec![], vec![], None)
        .expect("task");

    let captures = Rc::new(RefCell::new(Vec::new()));
    let mut model = CaptureModel::new(
        vec![activate("research"), respond("done")],
        captures.clone(),
    );
    app.run(&mut model, &task.task_id, 10).expect("run");

    let envelopes = captures.borrow();
    assert!(envelopes[0].active_skills.is_empty());
    assert_eq!(envelopes[1].active_skills.len(), 1);
    let skill = &envelopes[1].active_skills[0];
    assert_eq!(skill.name, "research");
    assert_eq!(
        skill.version, "0.2.0",
        "activation pins the package version"
    );
    assert!(skill.instructions.contains("Step 1: cite sources."));
    assert_eq!(app.skills.activations.borrow().len(), 1);
}

#[test]
fn a_run_can_switch_between_domains_and_state_is_per_run() {
    let storage = temp_root();
    let skills_dir = temp_root();
    write_skill(skills_dir.path(), "research", "RESEARCH-INSTRUCTIONS");
    write_skill(skills_dir.path(), "writing", "WRITING-INSTRUCTIONS");
    let app = build_app_with_options(
        storage.path(),
        Rc::new(DenyAllApprovals),
        Some(skills_dir.path().to_path_buf()),
    )
    .expect("app");
    let task = app
        .create_task("research then write", vec![], vec![], None)
        .expect("task");

    let captures = Rc::new(RefCell::new(Vec::new()));
    let mut model = CaptureModel::new(
        vec![activate("research"), activate("writing"), respond("report")],
        captures.clone(),
    );
    app.run(&mut model, &task.task_id, 10).expect("run");

    let envelopes = captures.borrow();
    assert_eq!(envelopes.len(), 3);
    let names: Vec<String> = envelopes[2]
        .active_skills
        .iter()
        .map(|s| s.name.clone())
        .collect();
    assert_eq!(names, vec!["research", "writing"]);
    assert!(envelopes[2]
        .active_skills
        .iter()
        .any(|s| s.instructions.contains("RESEARCH-INSTRUCTIONS")));
    assert!(envelopes[2]
        .active_skills
        .iter()
        .any(|s| s.instructions.contains("WRITING-INSTRUCTIONS")));

    // A later run on the same app starts with no activated skills.
    let task2 = app
        .create_task("unrelated question", vec![], vec![], None)
        .expect("task2");
    let captures2 = Rc::new(RefCell::new(Vec::new()));
    let mut model2 = CaptureModel::new(vec![respond("done")], captures2.clone());
    app.run(&mut model2, &task2.task_id, 10).expect("run2");
    assert!(captures2.borrow()[0].active_skills.is_empty());
}

#[test]
fn cli_lists_skill_metadata_only() {
    let storage = temp_root();
    let skills_dir = temp_root();
    write_skill(skills_dir.path(), "research", "CLI-TEST-INSTRUCTIONS");
    let bin = env!("CARGO_BIN_EXE_sfai");
    let out = std::process::Command::new(bin)
        .args([
            "skills",
            "list",
            "--skills",
            skills_dir.path().to_str().expect("utf8"),
            "--storage",
            storage.path().to_str().expect("utf8"),
        ])
        .output()
        .expect("spawn cli");
    assert!(
        out.status.success(),
        "cli failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("research"));
    assert!(
        !stdout.contains("CLI-TEST-INSTRUCTIONS"),
        "skill list must not print instructions"
    );
}
