//! Memory-layer tests (Claude Code-aligned: project/topic/task_summary/log).

mod common;

use common::temp_root;
use serde_json::json;
use sfai::app::build_app;
use sfai::context::memory::MemoryKind;
use sfai::contracts::IntentKind;
use sfai::core::action_intents::intent;

#[test]
fn notes_are_revisioned_not_duplicated() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let n1 = app
        .memory
        .write_note(
            MemoryKind::Topic,
            None,
            "style",
            "report style",
            "use citations",
        )
        .expect("note1");
    let n2 = app
        .memory
        .write_note(
            MemoryKind::Topic,
            None,
            "style",
            "report style",
            "use citations v2",
        )
        .expect("note2");
    assert_eq!(n1.note_id, n2.note_id);
    assert_eq!(n2.revision, 2, "same topic+title revises, never duplicates");
    assert_eq!(app.memory.list_notes(None, 10).expect("list").len(), 1);
}

#[test]
fn completed_tasks_leave_auto_memory() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let (task, result) = common::run_fixture(&app, "tool-use");
    assert_eq!(result.status, sfai::contracts::TaskStatus::Completed);
    let notes = app
        .memory
        .list_notes(Some("task_summary"), 10)
        .expect("notes");
    assert_eq!(notes.len(), 1);
    assert!(notes[0].body.contains(&task.task_id));
    assert!(notes[0].body.contains("outputs"));
}

#[test]
fn memory_relevant_to_goal_reaches_envelope_across_tasks() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    // Task 1 writes a topic note about "quokka retention figures".
    app.memory
        .write_note(
            MemoryKind::Topic,
            None,
            "zoology",
            "quokka retention",
            "quokka retention figure is 42 percent under sparse context",
        )
        .expect("note");

    // Task 2: goal mentions quokka — the note must ride into the envelope.
    let task2 = app
        .create_task("report the quokka retention figure", vec![], vec![], None)
        .expect("task2");
    let captures = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    {
        use sfai::contracts::{ContextEnvelope, ModelResponse, ModelUsage};
        use sfai::core::model_adapter::{ModelAdapter, ModelError};
        struct Capture(std::rc::Rc<std::cell::RefCell<Vec<String>>>);
        impl ModelAdapter for Capture {
            fn model_id(&self) -> String {
                "fake/capture".into()
            }
            fn decide(&mut self, envelope: &ContextEnvelope) -> Result<ModelResponse, ModelError> {
                self.0
                    .borrow_mut()
                    .push(serde_json::to_string(&envelope.memory).unwrap_or_default());
                Ok(ModelResponse {
                    model_id: "fake/capture".into(),
                    intent: intent(
                        IntentKind::Respond,
                        "x",
                        "answer",
                        json!({"text": "42 percent"}),
                        vec![],
                        None,
                    ),
                    usage: ModelUsage::default(),
                    latency_ms: 0.0,
                })
            }
        }
        let mut model = Capture(captures.clone());
        let result = app.run(&mut model, &task2.task_id, 5).expect("run");
        assert_eq!(result.status, sfai::contracts::TaskStatus::Completed);
    }
    let captured = captures.borrow().clone();
    assert!(!captured.is_empty());
    assert!(
        captured[0].contains("quokka"),
        "memory note must reach the envelope: {}",
        captured[0]
    );
}

#[test]
fn memory_index_is_materialized() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    app.memory
        .write_note(
            MemoryKind::Project,
            None,
            "conventions",
            "testing",
            "use offline fixtures",
        )
        .expect("note");
    let idx = app.memory.index_markdown().expect("index");
    assert!(idx.contains("MEMORY.md"));
    assert!(idx.contains("testing"));
}

#[test]
fn memory_notes_are_searchable() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    app.memory
        .write_note(
            MemoryKind::Topic,
            None,
            "billing",
            "invoice format",
            "invoices use EUR currency",
        )
        .expect("note");
    let hits = app
        .retriever
        .search("invoice currency", None, Some("memory"), 5)
        .expect("search");
    assert!(
        !hits.is_empty(),
        "memory notes must be findable via context.search"
    );
    assert_eq!(hits[0].kind, "memory");
}

#[test]
fn way_layer_records_mistakes_from_failed_runs() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    // failed-tool fixture ends blocked with SCRIPTED_FAILURE errors.
    let (_task, result) = common::run_fixture(&app, "failed-tool");
    assert_eq!(result.status, sfai::contracts::TaskStatus::Blocked);
    let ways = app.memory.list_notes(Some("way"), 10).expect("ways");
    assert_eq!(ways.len(), 1, "every terminal run leaves a way entry");
    assert!(
        ways[0].body.contains("mistakes"),
        "way body must have a mistakes section"
    );
    assert!(
        ways[0].body.contains("SCRIPTED_FAILURE"),
        "mistakes must carry the actual error codes: {}",
        ways[0].body
    );
}

#[test]
fn model_can_self_author_way_notes() {
    use serde_json::json;
    use sfai::contracts::IntentKind;
    use sfai::core::action_intents::intent;
    use sfai::integrations::providers::fake::{ScriptedModel, Step};

    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task("reflect on work", vec![], vec![], None)
        .expect("task");
    let steps = vec![
        Step::Intent(Box::new(intent(
            IntentKind::Execute,
            "unknown",
            "reflect",
            json!({"operation_id": "way.write", "arguments": {
                "method": "verify before finishing",
                "wins": ["fast search"],
                "mistakes": ["skipped citation check once"],
                "lesson": "always run citations_resolve"
            }}),
            vec![],
            Some("way-1".into()),
        ))),
        Step::Intent(Box::new(intent(
            IntentKind::Finish,
            "unknown",
            "done",
            json!({"text": "reflected"}),
            vec![],
            None,
        ))),
    ];
    let mut model = ScriptedModel::new(steps);
    let result = app.run(&mut model, &task.task_id, 10).expect("run");
    assert_eq!(result.status, sfai::contracts::TaskStatus::Completed);
    let ways = app.memory.list_notes(Some("way"), 10).expect("ways");
    assert!(ways.len() >= 2, "self-authored + auto way");
    assert!(ways
        .iter()
        .any(|w| w.body.contains("always run citations_resolve")));
}

#[test]
fn wiki_bridges_way_notes() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let (_task, result) = common::run_fixture(&app, "tool-use");
    assert_eq!(result.status, sfai::contracts::TaskStatus::Completed);
    let nodes = app.wiki.nodes_for(&_task.task_id).expect("nodes");
    assert!(
        nodes.iter().any(|n| n["kind"] == "way"),
        "way notes must be bridged into the wiki"
    );
    let edges = app.wiki.edges_for(&_task.task_id).expect("edges");
    assert!(edges.iter().any(|e| e["edge_type"] == "relevant_to"
        && e["to_node"].as_str().unwrap_or("").starts_with("wn_")));
}
