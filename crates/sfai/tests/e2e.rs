//! End-to-end demo (plan §15 first project demo) and CLI smoke test.

mod common;

use common::{run_fixture, temp_root};
use serde_json::{json, Value};
use sfai::app::build_app;
use sfai::contracts::{ContextEnvelope, IntentKind, TaskStatus};
use sfai::core::action_intents::intent;
use sfai::integrations::providers::fake::{ScriptedModel, Step};
use sfai::telemetry::replay::replay_trace;

#[test]
fn demo_report_and_followup_turn() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");

    // Demo turn 1: search -> evidence -> report artifact with citations.
    let (task, result) = run_fixture(&app, "tool-use");
    assert_eq!(result.status, TaskStatus::Completed);

    // Every run yields a trace whose cost totals replay exactly.
    let trace = app.export_trace(&task.task_id).expect("trace");
    let report = replay_trace(&trace);
    assert!(
        report.ok(),
        "trace replay anomalies: {:?}",
        report.anomalies
    );
    assert!(report.decisions >= 3);
    assert!(report.recorded_cost.is_some());

    // Demo turn 2: follow-up retrieves prior indexed sources by ref without
    // resending the earlier history (bounded envelope, two steps).
    let task2 = app
        .create_task(
            "Which prior sources mention cost savings? Cite them.",
            vec![],
            vec![],
            None,
        )
        .expect("task2");
    let steps = vec![
        Step::Intent(Box::new(intent(
            IntentKind::Retrieve,
            "unknown",
            "information_needed",
            json!({"op": "search", "query": "sparse context activation", "task_scope": "all", "limit": 5}),
            vec![],
            None,
        ))),
        Step::Dyn(Box::new(|envelope: &ContextEnvelope| {
            let recent = envelope
                .recent_results
                .last()
                .cloned()
                .unwrap_or(Value::Null);
            let refs: Vec<String> = recent
                .get("result_refs")
                .and_then(|r| r.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let citations: Vec<String> = refs
                .iter()
                .take(2)
                .map(|r| format!("- source: {r}"))
                .collect();
            intent(
                IntentKind::Respond,
                &sfai::fixtures::task_id_of(envelope),
                "direct_answer",
                json!({"text": format!("Prior sources:\n{}", citations.join("\n"))}),
                vec![],
                None,
            )
        })),
    ];
    let mut model = ScriptedModel::new(steps);
    let result2 = app.run(&mut model, &task2.task_id, 10).expect("follow-up");
    assert_eq!(result2.status, TaskStatus::Completed);
    assert_eq!(result2.steps, 2, "follow-up must not replay full history");
    let answer = result2.answer.unwrap_or_default();
    assert!(
        answer.contains("source://"),
        "follow-up must cite indexed refs: {answer}"
    );
}

#[test]
fn cli_runs_offline_fixture() {
    let dir = temp_root();
    let storage = dir.path().to_str().expect("utf8").to_string();
    let bin = env!("CARGO_BIN_EXE_sfai");

    let out = std::process::Command::new(bin)
        .args(["run", "--fixture", "direct-answer", "--storage", &storage])
        .output()
        .expect("spawn run");
    assert!(
        out.status.success(),
        "cli run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: Value = serde_json::from_str(&stdout).expect("json output");
    assert_eq!(parsed["status"], "completed");
    let task_id = parsed["task_id"].as_str().expect("task id").to_string();

    let out2 = std::process::Command::new(bin)
        .args(["status", &task_id, "--storage", &storage])
        .output()
        .expect("spawn status");
    assert!(out2.status.success());
    assert!(String::from_utf8_lossy(&out2.stdout).contains("completed"));

    let out3 = std::process::Command::new(bin)
        .args(["export-trace", &task_id, "--storage", &storage])
        .output()
        .expect("spawn export-trace");
    assert!(out3.status.success());
    let trace: Value = serde_json::from_slice(&out3.stdout).expect("trace json");
    assert!(!trace.as_array().unwrap_or(&vec![]).is_empty());
}
