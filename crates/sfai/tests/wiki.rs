//! M4 acceptance tests: incremental Wiki compiler.

mod common;

use common::temp_root;
use serde_json::{json, Value};
use sfai::app::{build_app, App};
use sfai::context::wiki::{RefResolver, ResolvedRef, WikiCompiler};
use sfai::contracts::TrustLevel;
use sfai::error::{Result, SfaiError};

fn append_tool_committed(app: &App, task_id: &str, refs: &[&str]) {
    app.ledger
        .append(
            task_id,
            "tool:web.search",
            "tool.committed",
            json!({
                "operation_id": "web.search",
                "status": "ok",
                "result_preview": "results",
                "result_refs": refs,
                "side_effects": [],
                "idempotency_key": null,
            }),
            None,
        )
        .expect("append event");
}

fn node_count(app: &App, task_id: &str) -> usize {
    app.wiki.nodes_for(task_id).expect("nodes").len()
}

#[test]
fn repeated_compilation_does_not_duplicate_facts() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task("research", vec![], vec![], None)
        .expect("task");
    let source = app
        .objects
        .put_source(
            &task.task_id,
            "https://example.org/s1",
            b"evidence one",
            "text/plain",
            "web",
            TrustLevel::Untrusted,
        )
        .expect("source");
    let reference = sfai::ids::source_ref(&source.source_id, 1);
    app.retriever
        .index_document(
            &reference,
            &task.task_id,
            "source",
            "Claim One",
            "evidence one",
            1,
        )
        .expect("index");
    append_tool_committed(&app, &task.task_id, &[&reference]);

    let first = app.wiki.compile_task(&task.task_id).expect("first compile");
    assert!(first.events_processed >= 1);
    assert!(first.nodes_created >= 2, "task node + evidence node");
    assert!(first.edges_added >= 1);

    let second = app
        .wiki
        .compile_task(&task.task_id)
        .expect("second compile");
    assert_eq!(
        second.events_processed, 0,
        "incremental compile must not reprocess events"
    );

    // Even after forcing a full rescan, nothing duplicates: state is stable.
    let before = node_count(&app, &task.task_id);
    app.wiki
        .db
        .execute("DELETE FROM wiki_state WHERE task_id = ?1", [&task.task_id])
        .expect("reset state");
    let rescan = app.wiki.compile_task(&task.task_id).expect("rescan");
    assert_eq!(
        rescan.nodes_created, 0,
        "identical facts must not duplicate"
    );
    assert_eq!(node_count(&app, &task.task_id), before);
}

#[test]
fn conflicting_evidence_keeps_contradiction_edges() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task("compare claims", vec![], vec![], None)
        .expect("task");

    let mut refs = Vec::new();
    for (i, origin) in ["https://a.example/claim", "https://b.example/claim"]
        .iter()
        .enumerate()
    {
        let source = app
            .objects
            .put_source(
                &task.task_id,
                origin,
                format!("version {i} of the claim").as_bytes(),
                "text/plain",
                "web",
                TrustLevel::Untrusted,
            )
            .expect("source");
        let reference = sfai::ids::source_ref(&source.source_id, 1);
        app.retriever
            .index_document(
                &reference,
                &task.task_id,
                "source",
                "Same Claim Title",
                &format!("version {i} of the claim"),
                1,
            )
            .expect("index");
        refs.push(reference);
    }
    append_tool_committed(
        &app,
        &task.task_id,
        &refs.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    );

    let report = app.wiki.compile_task(&task.task_id).expect("compile");
    assert!(
        report.conflicts >= 1,
        "same title + different content = conflict"
    );

    let nodes = app.wiki.nodes_for(&task.task_id).expect("nodes");
    let evidence: Vec<&Value> = nodes.iter().filter(|n| n["kind"] == "evidence").collect();
    assert_eq!(
        evidence.len(),
        2,
        "both pieces of evidence must be retained"
    );
    for node in evidence {
        assert_eq!(node["status"], "disputed");
        assert!(!node["provenance"].as_array().unwrap().is_empty());
    }
    let edges = app.wiki.edges_for(&task.task_id).expect("edges");
    let contradicts = edges
        .iter()
        .filter(|e| e["edge_type"] == "contradicts")
        .count();
    assert_eq!(
        contradicts, 2,
        "contradiction is recorded in both directions"
    );
}

#[test]
fn stale_ref_is_flagged_not_dropped() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task("reference missing source", vec![], vec![], None)
        .expect("task");
    append_tool_committed(&app, &task.task_id, &["source://does_not_exist@1"]);

    let report = app.wiki.compile_task(&task.task_id).expect("compile");
    assert_eq!(report.stale_refs, 1);
    let nodes = app.wiki.nodes_for(&task.task_id).expect("nodes");
    let stale: Vec<&Value> = nodes.iter().filter(|n| n["status"] == "stale").collect();
    assert_eq!(stale.len(), 1, "stale refs remain visible for repair");
    let summary = app.wiki.summary(&task.task_id).expect("summary");
    assert_eq!(summary["stale_refs"], 1);
}

struct FailingResolver;

impl RefResolver for FailingResolver {
    fn resolve(&self, _ref_id: &str) -> Result<Option<ResolvedRef>> {
        Err(SfaiError::Storage("simulated resolver failure".into()))
    }
}

#[test]
fn failed_compilation_cannot_corrupt_task_authority() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task(
            "compile will fail",
            vec!["hard constraint".into()],
            vec![],
            None,
        )
        .expect("task");
    append_tool_committed(&app, &task.task_id, &["source://whatever@1"]);

    let events_before = app
        .ledger
        .events_for(&task.task_id, 0)
        .expect("events")
        .len();
    let state_before = app.task_store.state(&task.task_id).expect("state");

    let broken = WikiCompiler::new(
        app.db.clone(),
        app.ledger.clone(),
        Box::new(FailingResolver),
    );
    assert!(broken.compile_task(&task.task_id).is_err());

    // Task authority untouched: same events, same state, resume still works.
    let events_after = app
        .ledger
        .events_for(&task.task_id, 0)
        .expect("events")
        .len();
    assert_eq!(events_before, events_after);
    let state_after = app.task_store.state(&task.task_id).expect("state");
    assert_eq!(state_before, state_after);
    assert!(app.task_store.resume(&task.task_id).is_ok());

    // Failure is recorded for retry.
    let failed_logs: i64 = app
        .db
        .query_one(
            "SELECT COUNT(*) AS n FROM wiki_compile_log WHERE task_id = ?1 AND status = 'failed'",
            [&task.task_id],
        )
        .expect("query")
        .map(|r| r.get("n").and_then(|v| v.as_i64()).unwrap_or(0))
        .unwrap_or(0);
    assert_eq!(failed_logs, 1);
    // Progress is not advanced past the failure, so a retry reprocesses it.
    let progress: i64 = app
        .db
        .query_one(
            "SELECT last_compiled_seq FROM wiki_state WHERE task_id = ?1",
            [&task.task_id],
        )
        .expect("query")
        .map(|r| {
            r.get("last_compiled_seq")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
        })
        .unwrap_or(0);
    assert_eq!(progress, 0, "failed compile must not advance the cursor");
}

#[test]
fn non_material_events_are_not_compiled() {
    let dir = temp_root();
    let app = build_app(dir.path()).expect("app");
    let task = app
        .create_task("chat only", vec![], vec![], None)
        .expect("task");
    // task.created is material; drain it first.
    app.wiki
        .compile_task(&task.task_id)
        .expect("initial compile");
    app.task_store
        .record_decision_started(&task.task_id, "respond", "chit_chat")
        .expect("decision event");
    app.task_store
        .checkpoint(&task.task_id, "note", Some(&[]))
        .expect("checkpoint");

    let report = app.wiki.compile_task(&task.task_id).expect("compile");
    assert_eq!(
        report.events_processed, 0,
        "intent/checkpoint events are not material changes"
    );
}
