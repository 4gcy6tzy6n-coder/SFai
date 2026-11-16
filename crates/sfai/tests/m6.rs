//! M6 acceptance tests: independent verification, mandatory validation
//! boundaries, bounded repair, and deliberate-error detection.

mod common;

use common::{temp_root, tool_run_rows};
use serde_json::{json, Value};
use sfai::app::{build_app, build_app_with_approvals, App};
use sfai::authority::approvals::AutoApprove;
use sfai::contracts::{IntentKind, TaskStatus, Verdict};
use sfai::core::action_intents::intent;
use sfai::integrations::providers::fake::{ScriptedModel, Step};
use std::rc::Rc;

fn execute(op: &str, args: Value, key: &str) -> Step {
    Step::Intent(Box::new(intent(
        IntentKind::Execute,
        "unknown",
        "deliver",
        json!({"operation_id": op, "arguments": args}),
        vec![],
        Some(key.into()),
    )))
}

fn verify(criteria: &[&str]) -> Step {
    Step::Intent(Box::new(intent(
        IntentKind::Verify,
        "unknown",
        "validate",
        json!({"criteria": criteria}),
        vec![],
        None,
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

fn write_report(content: &str, key: &str) -> Step {
    execute(
        "file.write",
        json!({"path": "report.md", "content": content}),
        key,
    )
}

fn find_issue_codes(app: &App, task_id: &str) -> Vec<String> {
    let state = app.task_store.state(task_id).expect("state");
    state
        .open_issues
        .iter()
        .filter_map(|i| {
            i.get("error_code")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string())
        })
        .collect()
}

#[test]
fn code_checks_run_inside_the_task_workspace() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task("code check", vec![], vec![], None)
        .expect("task");

    let results = app
        .verifier
        .verify(
            &task.task_id,
            &[
                "code_tests_pass:sh -c 'exit 0'".into(),
                "code_tests_pass:sh -c 'echo boom; exit 7'".into(),
            ],
        )
        .expect("verify");

    assert_eq!(results[0].verdict, Verdict::Pass);
    assert_eq!(results[1].verdict, Verdict::Fail);
    assert_eq!(results[1].diagnostics[0].error_code, "CODE_TESTS_FAILED");
    assert!(results[1].diagnostics[0].repair_hint.contains("boom"));
    let workspace = dir.path().join("workspaces").join(&task.task_id);
    assert!(workspace.is_dir(), "task workspace must exist");
}

#[test]
fn json_and_placeholder_checks_detect_deliberate_errors() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task("validate documents", vec![], vec![], None)
        .expect("task");
    app.objects
        .put_artifact(
            &task.task_id,
            "ok.json",
            b"{\"a\": 1}",
            "application/json",
            &[],
        )
        .expect("ok json");
    app.objects
        .put_artifact(
            &task.task_id,
            "bad.json",
            b"{not json",
            "application/json",
            &[],
        )
        .expect("bad json");
    app.objects
        .put_artifact(
            &task.task_id,
            "draft.md",
            b"# Draft\n\nTODO: finish this section",
            "text/markdown",
            &[],
        )
        .expect("draft");
    app.objects
        .put_artifact(
            &task.task_id,
            "clean.md",
            b"# Final\n\nAll claims are backed by stored sources.",
            "text/markdown",
            &[],
        )
        .expect("clean");

    let results = app
        .verifier
        .verify(
            &task.task_id,
            &[
                "json_valid:ok.json".into(),
                "json_valid:bad.json".into(),
                "no_placeholders:draft.md".into(),
                "no_placeholders:clean.md".into(),
            ],
        )
        .expect("verify");
    let verdicts: Vec<Verdict> = results.iter().map(|r| r.verdict).collect();
    assert_eq!(
        verdicts,
        vec![Verdict::Pass, Verdict::Fail, Verdict::Fail, Verdict::Pass]
    );
    assert_eq!(results[1].diagnostics[0].error_code, "JSON_INVALID");
    assert_eq!(
        results[2].diagnostics[0].error_code,
        "PLACEHOLDER_TEXT_FOUND"
    );
    assert!(results[2].diagnostics[0]
        .repair_hint
        .contains("placeholder"));
}

#[test]
fn acceptance_failures_are_visible_and_bounded() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task(
            "deliver a clean cited report",
            vec![],
            vec![
                "artifact_exists:report.md".into(),
                "citations_resolve".into(),
                "no_placeholders:report.md".into(),
            ],
            None,
        )
        .expect("task");

    // Deliberately broken draft: placeholder + unresolvable citation.
    let broken = "# Draft\n\nTODO: fill in the analysis.\n\n- source: source://missing@1\n";
    let steps = vec![
        write_report(broken, "w1"),
        finish(),
        write_report(broken, "w2"),
        finish(),
        write_report(broken, "w3"),
        finish(),
    ];
    let mut model = ScriptedModel::new(steps);
    let result = app.run(&mut model, &task.task_id, 20).expect("run");

    assert_eq!(result.status, TaskStatus::Blocked);
    let codes = find_issue_codes(&app, &task.task_id);
    assert!(
        codes.iter().any(|c| c == "PLACEHOLDER_TEXT_FOUND"),
        "placeholder must be detected: {codes:?}"
    );
    assert!(
        codes.iter().any(|c| c == "BROKEN_CITATION_REF"),
        "broken citation must be detected: {codes:?}"
    );

    // Repair turn: a clean, cited report completes the same task.
    let source = app
        .objects
        .put_source(
            &task.task_id,
            "https://example.org/evidence",
            b"verified evidence",
            "text/plain",
            "web",
            sfai::contracts::TrustLevel::Untrusted,
        )
        .expect("source");
    let reference = sfai::ids::source_ref(&source.source_id, 1);
    app.retriever
        .index_document(
            &reference,
            &task.task_id,
            "source",
            "Evidence",
            "verified evidence",
            1,
        )
        .expect("index");
    let clean = format!("# Report\n\nAll claims are backed.\n\n- source: {reference}\n");
    let steps2 = vec![write_report(&clean, "w-clean"), finish()];
    let mut model2 = ScriptedModel::new(steps2);
    let result2 = app.run(&mut model2, &task.task_id, 10).expect("repair run");

    assert_eq!(result2.status, TaskStatus::Completed);
}

#[test]
fn consequential_actions_require_a_later_verification() {
    let dir = temp_root();
    let app = build_app_with_approvals(dir.path(), Rc::new(AutoApprove)).expect("app");

    // Without verification: completion is blocked with a visible issue.
    let task = app
        .create_task("send mail and report", vec![], vec![], None)
        .expect("task");
    let steps = vec![
        execute(
            "mail.send",
            json!({"to": "user@example.org", "body": "status"}),
            "mail-1",
        ),
        finish(),
        finish(),
        finish(),
    ];
    let mut model = ScriptedModel::new(steps);
    let result = app.run(&mut model, &task.task_id, 10).expect("run");
    assert_eq!(result.status, TaskStatus::Blocked);
    assert!(find_issue_codes(&app, &task.task_id)
        .iter()
        .any(|c| c == "MANDATORY_VERIFICATION_MISSING"));

    // With a recorded verification after the action: completion is allowed.
    let task2 = app
        .create_task("send mail and verify", vec![], vec![], None)
        .expect("task2");
    let steps2 = vec![
        execute(
            "mail.send",
            json!({"to": "user@example.org", "body": "status"}),
            "mail-2",
        ),
        write_report("# Status\n\nMail sent after approval.\n", "rep-2"),
        verify(&["artifact_exists:report.md"]),
        finish(),
    ];
    let mut model2 = ScriptedModel::new(steps2);
    let result2 = app.run(&mut model2, &task2.task_id, 10).expect("run2");
    assert_eq!(result2.status, TaskStatus::Completed);
    let state = app.task_store.state(&task2.task_id).expect("state");
    assert!(
        !state.verifications.is_empty(),
        "verification results must be recorded in task state"
    );
    assert!(
        tool_run_rows(&app, &task2.task_id, "mail.send")
            .iter()
            .any(|r| r["status"] == "ok"),
        "approved action must execute"
    );
}

#[test]
fn verification_events_compile_into_the_wiki() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task("verify and document", vec![], vec![], None)
        .expect("task");
    let steps = vec![
        write_report("# Report\n\nbody\n", "w-wiki"),
        verify(&["artifact_exists:report.md"]),
        finish(),
    ];
    let mut model = ScriptedModel::new(steps);
    app.run(&mut model, &task.task_id, 10).expect("run");

    let nodes = app.wiki.nodes_for(&task.task_id).expect("nodes");
    assert!(
        nodes.iter().any(|n| n["kind"] == "verification"),
        "verification node must exist"
    );
    let edges = app.wiki.edges_for(&task.task_id).expect("edges");
    assert!(
        edges.iter().any(|e| e["edge_type"] == "verified_by"),
        "passing verification must produce a verified_by edge"
    );
}
