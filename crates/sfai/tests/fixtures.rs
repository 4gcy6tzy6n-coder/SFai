//! Acceptance fixture suite: direct answer / tool use / failed tool /
//! approval / context retrieval (plan M0 fixture list).

mod common;

use common::{create_and_setup, run_fixture, temp_root, tool_run_rows};
use sfai::app::{build_app, build_app_with_approvals};
use sfai::authority::approvals::ScriptedApprovals;
use sfai::contracts::{TaskStatus, Verdict};
use std::rc::Rc;

#[test]
fn direct_answer_makes_no_tool_calls() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let (task, result) = run_fixture(&app, "direct-answer");

    assert_eq!(result.status, TaskStatus::Completed);
    assert_eq!(result.answer.as_deref(), Some("4"));
    assert_eq!(result.steps, 1);
    assert!(
        app.broker.invocations.borrow().is_empty(),
        "direct answer must not invoke tools"
    );
    assert!(
        app.registry.activations.borrow().is_empty(),
        "direct answer must not activate capabilities"
    );
    assert_eq!(result.cost["tool_calls"].as_u64().unwrap_or(0), 0);
    let _ = task;
}

#[test]
fn tool_use_produces_verifiable_artifact() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let (task, result) = run_fixture(&app, "tool-use");

    assert_eq!(result.status, TaskStatus::Completed);
    let artifact = app
        .objects
        .find_artifact_by_path(&task.task_id, "report.md")
        .expect("query")
        .expect("report artifact");
    let text = app
        .objects
        .read_artifact_text(&artifact.artifact_id)
        .expect("artifact text");
    assert!(text.contains("Citations"));
    assert!(
        !artifact.provenance.is_empty(),
        "artifact must carry provenance refs"
    );

    let verification = app
        .verifier
        .verify(
            &task.task_id,
            &[
                "artifact_exists:report.md".into(),
                "citations_resolve".into(),
                "source_count_at_least:3".into(),
            ],
        )
        .expect("verify");
    assert!(verification.iter().all(|r| r.verdict == Verdict::Pass));

    assert_eq!(
        app.tool_counts
            .borrow()
            .get("file.write")
            .copied()
            .unwrap_or(0),
        1
    );
    assert!(
        app.tool_counts
            .borrow()
            .get("web.search")
            .copied()
            .unwrap_or(0)
            >= 1
    );
    assert!(app
        .side_effect_log
        .borrow()
        .iter()
        .any(|s| s == "file.written:report.md"));
    assert!(!app.export_trace(&task.task_id).expect("trace").is_empty());
}

#[test]
fn failed_tool_blocks_after_bounded_retry() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let (task, result) = run_fixture(&app, "failed-tool");

    assert_eq!(result.status, TaskStatus::Blocked);
    assert_eq!(
        app.tool_counts
            .borrow()
            .get("fail.always")
            .copied()
            .unwrap_or(0),
        3,
        "bounded retry must stop at 3 consecutive errors"
    );
    let state = app.task_store.state(&task.task_id).expect("state");
    assert!(
        !state.unresolved_errors.is_empty(),
        "errors must stay visible"
    );
    assert!(
        state.completed_outputs.is_empty(),
        "blocked task must not claim outputs"
    );
}

#[test]
fn approval_denial_has_no_side_effect() {
    let dir = temp_root();
    let approvals = Rc::new(ScriptedApprovals::new(vec![false]));
    let app = build_app_with_approvals(dir.path(), approvals.clone()).expect("app");
    let (task, result) = run_fixture(&app, "approval");

    assert_eq!(result.status, TaskStatus::WaitingApproval);
    assert_eq!(approvals.requested.borrow().len(), 1);
    assert!(
        app.side_effect_log.borrow().is_empty(),
        "denied action must leave no side effect"
    );
    assert_eq!(
        app.tool_counts
            .borrow()
            .get("mail.send")
            .copied()
            .unwrap_or(0),
        0,
        "handler must never run for a denied action"
    );
    let rows = tool_run_rows(&app, &task.task_id, "mail.send");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["status"], "denied");
    assert_eq!(rows[0]["error_code"], "APPROVAL_DENIED");
}

#[test]
fn context_retrieval_recovers_needle_and_keeps_constraints() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let (spec, task) = create_and_setup(&app, "context-retrieval");
    let mut model = (spec.build_model)();
    let result = app.run(model.as_mut(), &task.task_id, 30).expect("run");

    assert_eq!(result.status, TaskStatus::Completed);
    let answer = result.answer.unwrap_or_default();
    assert!(
        answer.contains(sfai::fixtures::NEEDLE),
        "answer must quote the recovered passage: {answer}"
    );
    let state = app.task_store.state(&task.task_id).expect("state");
    assert_eq!(state.pinned_constraints, spec.constraints);

    let operations: Vec<String> = app
        .ledger
        .events_for(&task.task_id, 0)
        .expect("events")
        .iter()
        .filter(|e| e.event_type == "tool.committed")
        .map(|e| {
            e.payload
                .get("operation_id")
                .and_then(|o| o.as_str())
                .unwrap_or("")
                .to_string()
        })
        .collect();
    assert!(operations.iter().any(|o| o == "context.search"));
    assert!(operations.iter().any(|o| o == "context.read"));
}
