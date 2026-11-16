//! Router-block hardening tests (plan §4 loop guard + §13.4 routing metrics).

mod common;

use common::{temp_root, tool_run_rows};
use serde_json::json;
use sfai::app::build_app;
use sfai::contracts::{IntentKind, TaskStatus};
use sfai::core::action_intents::intent;
use sfai::integrations::providers::fake::{ReactiveDemoModel, ScriptedModel, Step};

fn execute(op: &str, key: &str) -> Step {
    Step::Intent(Box::new(intent(
        IntentKind::Execute,
        "unknown",
        "route",
        json!({"operation_id": op, "arguments": {"path": "a.md", "content": "x"}}),
        vec![],
        Some(key.into()),
    )))
}

fn finish() -> Step {
    Step::Intent(Box::new(intent(
        IntentKind::Finish,
        "unknown",
        "done",
        json!({"text": "done"}),
        vec![],
        None,
    )))
}

#[test]
fn loop_guard_refuses_the_third_identical_action() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task("loop test", vec![], vec![], None)
        .expect("task");

    // Same action three times with *different* idempotency keys: the broker
    // would happily execute all three — the loop guard must stop the third.
    let steps = vec![
        execute("file.write", "k1"),
        execute("file.write", "k2"),
        execute("file.write", "k3"),
        finish(),
    ];
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
        2,
        "only the first two identical actions execute"
    );
    let routing = result.routing;
    assert_eq!(routing["loop_guard_trips"], 1);
    assert_eq!(routing["misroutes"], 0);

    // The refusal is auditable and did not trip the error bound.
    let state = app.task_store.state(&task.task_id).expect("state");
    assert!(state.status == TaskStatus::Completed);
    let _ = tool_run_rows(&app, &task.task_id, "file.write");
}

#[test]
fn different_actions_are_not_guarded() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task("vary actions", vec![], vec![], None)
        .expect("task");
    // Distinct payloads -> distinct signatures -> no guard trip.
    let steps = vec![
        Step::Intent(Box::new(intent(
            IntentKind::Execute,
            "unknown",
            "route",
            json!({"operation_id": "file.write", "arguments": {"path": "a.md", "content": "1"}}),
            vec![],
            Some("w1".into()),
        ))),
        Step::Intent(Box::new(intent(
            IntentKind::Execute,
            "unknown",
            "route",
            json!({"operation_id": "file.write", "arguments": {"path": "b.md", "content": "2"}}),
            vec![],
            Some("w2".into()),
        ))),
        Step::Intent(Box::new(intent(
            IntentKind::Execute,
            "unknown",
            "route",
            json!({"operation_id": "file.write", "arguments": {"path": "c.md", "content": "3"}}),
            vec![],
            Some("w3".into()),
        ))),
        finish(),
    ];
    let mut model = ScriptedModel::new(steps);
    let result = app.run(&mut model, &task.task_id, 10).expect("run");
    assert_eq!(result.status, TaskStatus::Completed);
    assert_eq!(result.routing["loop_guard_trips"], 0);
    assert_eq!(
        app.tool_counts
            .lock()
            .unwrap()
            .get("file.write")
            .copied()
            .unwrap_or(0),
        3
    );
}

#[test]
fn misroutes_and_duplicate_replays_are_counted() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task("count misroutes", vec![], vec![], None)
        .expect("task");
    let steps = vec![
        Step::Intent(Box::new(intent(
            IntentKind::Execute,
            "unknown",
            "route",
            json!({"operation_id": "evil.op", "arguments": {}}),
            vec![],
            Some("m1".into()),
        ))),
        execute("file.write", "same-key"),
        execute("file.write", "same-key"),
        finish(),
    ];
    let mut model = ScriptedModel::new(steps);
    let result = app.run(&mut model, &task.task_id, 10).expect("run");
    let routing = result.routing;
    assert_eq!(routing["misroutes"], 1);
    assert_eq!(routing["duplicate_replays"], 1);
    assert_eq!(
        app.tool_counts
            .lock()
            .unwrap()
            .get("file.write")
            .copied()
            .unwrap_or(0),
        1
    );
}

#[test]
fn routing_report_is_recorded_for_self_routed_runs() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task(
            "find sources and write a cited report",
            vec![],
            vec![],
            None,
        )
        .expect("task");
    let mut model = ReactiveDemoModel::new();
    let result = app.run(&mut model, &task.task_id, 20).expect("run");
    assert_eq!(result.status, TaskStatus::Completed);

    let routing = result.routing;
    let kinds = routing["intent_kinds"].as_object().expect("histogram");
    assert!(kinds.get("execute").and_then(|v| v.as_u64()).unwrap_or(0) >= 2);
    assert!(kinds.get("finish").and_then(|v| v.as_u64()).unwrap_or(0) >= 1);
    assert_eq!(routing["misroutes"], 0);
    assert_eq!(routing["loop_guard_trips"], 0);
}

#[test]
fn ask_user_intent_is_counted() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task("ask the user", vec![], vec![], None)
        .expect("task");
    let steps = vec![Step::Intent(Box::new(intent(
        IntentKind::AskUser,
        "unknown",
        "need_input",
        json!({"question": "which variant?"}),
        vec![],
        None,
    )))];
    let mut model = ScriptedModel::new(steps);
    let result = app.run(&mut model, &task.task_id, 5).expect("run");
    assert_eq!(result.status, TaskStatus::WaitingApproval);
    assert_eq!(result.routing["ask_user"], 1);
}
