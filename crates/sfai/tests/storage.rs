//! Storage semantics: idempotent replay, crash recovery, event replay
//! equality, ledger dedupe (plan M2 exit gate).

mod common;

use common::{temp_root, tool_run_rows};
use serde_json::{json, Value};
use sfai::app::build_app;
use sfai::contracts::{IntentKind, TaskStatus};
use sfai::core::action_intents::intent;
use sfai::integrations::providers::fake::{ScriptedModel, Step};
use sfai::storage::db::Database;
use sfai::storage::event_ledger::EventLedger;

fn write_step(key: &str) -> Step {
    Step::Intent(Box::new(intent(
        IntentKind::Execute,
        "unknown",
        "write_artifact",
        json!({"operation_id": "file.write", "arguments": {"path": "a.md", "content": "hello world"}}),
        vec![],
        Some(key.into()),
    )))
}

fn search_step(key: &str) -> Step {
    Step::Intent(Box::new(intent(
        IntentKind::Execute,
        "unknown",
        "search",
        json!({"operation_id": "web.search", "arguments": {"query": "needle"}}),
        vec![],
        Some(key.into()),
    )))
}

fn finish_step() -> Step {
    Step::Intent(Box::new(intent(
        IntentKind::Finish,
        "unknown",
        "done",
        json!({"text": "ok"}),
        vec![],
        None,
    )))
}

#[test]
fn duplicate_idempotency_key_commits_once() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task("write a file twice with the same key", vec![], vec![], None)
        .expect("task");

    let steps = vec![write_step("w1"), write_step("w1"), finish_step()];
    let mut model = ScriptedModel::new(steps);
    let result = app.run(&mut model, &task.task_id, 10).expect("run");

    assert_eq!(result.status, TaskStatus::Completed);
    assert_eq!(
        app.tool_counts
            .lock()
            .unwrap()
            .get("file.write")
            .copied()
            .unwrap_or(0),
        1,
        "second intent must replay, not re-execute"
    );
    let state = app.task_store.state(&task.task_id).expect("state");
    assert_eq!(state.completed_outputs.len(), 1);
    assert_eq!(tool_run_rows(&app, &task.task_id, "file.write").len(), 1);
}

#[test]
fn crash_mid_run_resumes_without_duplicates() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task(
            "survive a crash",
            vec!["pinned constraint survives".into()],
            vec![],
            None,
        )
        .expect("task");

    let steps = vec![search_step("s1"), write_step("w1"), write_step("w2")];
    let mut crashing = ScriptedModel::new(steps).with_crash(3);
    assert!(
        app.run(&mut crashing, &task.task_id, 10).is_err(),
        "simulated crash must surface"
    );

    let state = app.task_store.state(&task.task_id).expect("state");
    assert_eq!(state.status, TaskStatus::Active);
    assert_eq!(state.tool_calls_made, 2);
    assert_eq!(state.completed_outputs.len(), 1);
    assert_eq!(
        state.pinned_constraints,
        vec!["pinned constraint survives".to_string()]
    );

    let resumed = app.resume_state(&task.task_id).expect("resume");
    assert_eq!(resumed.tool_calls_made, 2);

    // Resume: replay the already-committed write by key, then the new one.
    let steps2 = vec![write_step("w1"), write_step("w2"), finish_step()];
    let mut model = ScriptedModel::new(steps2);
    let result = app.run(&mut model, &task.task_id, 10).expect("resume run");

    assert_eq!(result.status, TaskStatus::Completed);
    assert_eq!(
        app.tool_counts
            .lock()
            .unwrap()
            .get("file.write")
            .copied()
            .unwrap_or(0),
        2,
        "replayed write must not execute again"
    );
    let state_after = app.task_store.state(&task.task_id).expect("state");
    assert_eq!(state_after.completed_outputs.len(), 2);
    assert_eq!(tool_run_rows(&app, &task.task_id, "file.write").len(), 2);
}

#[test]
fn events_replay_into_same_task_state() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task(
            "research and report",
            vec!["cite sources".into()],
            vec![],
            None,
        )
        .expect("task");
    let mut model = sfai::integrations::providers::fake::ReactiveDemoModel::new();
    // Give the reactive model an acceptance boundary by writing the artifact
    // itself; this test only needs a realistic event stream.
    let mut model = {
        let _ = &mut model;
        let steps = vec![
            Step::Intent(Box::new(intent(
                IntentKind::Execute,
                "unknown",
                "search",
                json!({"operation_id": "web.search", "arguments": {"query": "context"}}),
                vec![],
                Some("s1".into()),
            ))),
            Step::Intent(Box::new(intent(
                IntentKind::Execute,
                "unknown",
                "write",
                json!({"operation_id": "file.write", "arguments": {"path": "r.md", "content": "draft"}}),
                vec![],
                Some("w1".into()),
            ))),
            finish_step(),
        ];
        ScriptedModel::new(steps)
    };
    let result = app.run(&mut model, &task.task_id, 10).expect("run");
    assert_eq!(result.status, TaskStatus::Completed);

    // A second app over the same storage replays to the identical state.
    let app2 = build_app(dir.path()).expect("app2");
    let s1 = app.task_store.state(&task.task_id).expect("state1");
    let s2 = app2.task_store.state(&task.task_id).expect("state2");
    assert_eq!(s1, s2, "event replay must reproduce the same task state");
}

#[test]
fn ledger_dedupes_by_idempotency_key() {
    let db = Database::open_in_memory().expect("db");
    let ledger = EventLedger::new(db);
    let first: Value = json!({"status": "created"});
    ledger
        .append("t1", "system", "task.created", first.clone(), Some("k1"))
        .expect("append");
    let seq = ledger.last_seq("t1").expect("seq");
    ledger
        .append("t1", "system", "task.created", first, Some("k1"))
        .expect("duplicate append");
    assert_eq!(ledger.last_seq("t1").expect("seq"), seq);
    assert_eq!(ledger.events_for("t1", 0).expect("events").len(), 1);
}
