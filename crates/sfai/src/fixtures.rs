//! Offline task fixtures (plan M0: direct response, tool use, failed tool,
//! approval, context retrieval) used by tests, `sfai run --fixture`, and demos.

use crate::app::App;
use crate::contracts::{ActionIntent, Budget, ContextEnvelope, IntentKind, Task};
use crate::core::action_intents::intent;
use crate::core::model_adapter::ModelAdapter;
use crate::error::Result;
use crate::integrations::providers::fake::{ReactiveDemoModel, ScriptedModel, Step};
use serde_json::{json, Value};

pub const NEEDLE: &str =
    "the sparse-context policy preserved every hard constraint at 40% of full-history cost";

pub struct FixtureSpec {
    pub name: String,
    pub goal: String,
    pub constraints: Vec<String>,
    pub acceptance_rules: Vec<String>,
    pub budget: Budget,
    pub build_model: Box<dyn Fn() -> Box<dyn ModelAdapter>>,
    pub setup: Option<fn(&App, &Task) -> Result<()>>,
    pub description: String,
}

pub fn task_id_of(envelope: &ContextEnvelope) -> String {
    envelope
        .root_index
        .get("task")
        .and_then(|r| r.strip_prefix("task://"))
        .and_then(|r| r.split('/').next())
        .unwrap_or("unknown")
        .to_string()
}

fn setup_long_history(app: &App, task: &Task) -> Result<()> {
    for i in 0..40 {
        let body = if i == 20 {
            format!("Earlier findings (note {i}): {NEEDLE}.")
        } else {
            format!("Routine progress note {i}: intermediate work, no material findings yet.")
        };
        let source = app.objects.put_source(
            &task.task_id,
            &format!("note://{i}"),
            body.as_bytes(),
            "text/plain",
            "system",
            crate::contracts::TrustLevel::Trusted,
        )?;
        app.retriever.index_document(
            &crate::ids::source_ref(&source.source_id, 1),
            &task.task_id,
            "source",
            &format!("Note {i}"),
            &body,
            1,
        )?;
    }
    app.task_store
        .checkpoint(&task.task_id, "history seeded", Some(&[]))?;
    Ok(())
}

pub fn fixture_direct_answer() -> FixtureSpec {
    FixtureSpec {
        name: "direct-answer".into(),
        goal: "What is 2 + 2? Answer directly.".into(),
        constraints: vec!["Do not use tools for trivial arithmetic.".into()],
        acceptance_rules: vec![],
        budget: Budget::default(),
        build_model: Box::new(|| {
            Box::new(ScriptedModel::new(vec![Step::Intent(Box::new(intent(
                IntentKind::Respond,
                "unknown",
                "direct_answer",
                json!({"text": "4"}),
                vec![],
                None,
            )))]))
        }),
        setup: None,
        description: "Direct answer with zero discovery calls (no forced router).".into(),
    }
}

pub fn fixture_tool_use() -> FixtureSpec {
    FixtureSpec {
        name: "tool-use".into(),
        goal: "Find three recent sources on sparse context activation, compare their claims, \
               and create a short report with verifiable citations."
            .into(),
        constraints: vec!["Final output must be a markdown report with citations.".into()],
        acceptance_rules: vec![
            "artifact_exists:report.md".into(),
            "citations_resolve".into(),
            "source_count_at_least:3".into(),
        ],
        budget: Budget::default(),
        build_model: Box::new(|| Box::new(ReactiveDemoModel::new())),
        setup: None,
        description: "Search -> evidence -> artifact -> acceptance verification.".into(),
    }
}

pub fn fixture_failed_tool() -> FixtureSpec {
    let build = || -> Box<dyn ModelAdapter> {
        let steps = vec![
            Step::Intent(Box::new(intent(
                IntentKind::Execute,
                "unknown",
                "try_tool",
                json!({"operation_id": "fail.always", "arguments": {}}),
                vec![],
                Some("fail-1".into()),
            ))),
            Step::Intent(Box::new(intent(
                IntentKind::Execute,
                "unknown",
                "retry",
                json!({"operation_id": "fail.always", "arguments": {}}),
                vec![],
                Some("fail-2".into()),
            ))),
            Step::Intent(Box::new(intent(
                IntentKind::Execute,
                "unknown",
                "retry_again",
                json!({"operation_id": "fail.always", "arguments": {}}),
                vec![],
                Some("fail-3".into()),
            ))),
            Step::Intent(Box::new(intent(
                IntentKind::Finish,
                "unknown",
                "giving_up",
                json!({"text": "Tool keeps failing; reporting blocked."}),
                vec![],
                None,
            ))),
        ];
        Box::new(ScriptedModel::new(steps))
    };
    FixtureSpec {
        name: "failed-tool".into(),
        goal: "Run the failing diagnostic tool and report the outcome honestly.".into(),
        constraints: vec![],
        acceptance_rules: vec![],
        budget: Budget::default(),
        build_model: Box::new(build),
        setup: None,
        description: "Bounded retry on tool errors; visible blocked state, no silent success."
            .into(),
    }
}

pub fn fixture_approval() -> FixtureSpec {
    fn after_send(envelope: &ContextEnvelope) -> ActionIntent {
        let last_status = envelope
            .recent_results
            .last()
            .and_then(|r| r.get("status"))
            .and_then(|s| s.as_str())
            .unwrap_or("denied");
        if last_status == "ok" {
            intent(
                IntentKind::Verify,
                &task_id_of(envelope),
                "verify_external_action",
                json!({"criteria": []}),
                vec![],
                None,
            )
        } else {
            intent(
                IntentKind::AskUser,
                &task_id_of(envelope),
                "needs_authorization",
                json!({"question": "May I send this email? The previous attempt was denied."}),
                vec![],
                None,
            )
        }
    }

    let build = || -> Box<dyn ModelAdapter> {
        let steps = vec![
            Step::Intent(Box::new(intent(
                IntentKind::Execute,
                "unknown",
                "external_action",
                json!({"operation_id": "mail.send", "arguments": {"to": "user@example.org", "body": "hello"}}),
                vec![],
                Some("mail-1".into()),
            ))),
            Step::Dyn(Box::new(after_send)),
            Step::Intent(Box::new(intent(
                IntentKind::Finish,
                "unknown",
                "done",
                json!({"text": "Status email sent and verified."}),
                vec![],
                None,
            ))),
        ];
        Box::new(ScriptedModel::new(steps))
    };
    FixtureSpec {
        name: "approval".into(),
        goal: "Send a status email to the user.".into(),
        constraints: vec!["Never send external messages without explicit approval.".into()],
        acceptance_rules: vec![],
        budget: Budget::default(),
        build_model: Box::new(build),
        setup: None,
        description: "External write requires approval; denial produces no side effect.".into(),
    }
}

pub fn fixture_context_retrieval() -> FixtureSpec {
    fn read_top_result(envelope: &ContextEnvelope) -> ActionIntent {
        let recent = envelope
            .recent_results
            .last()
            .cloned()
            .unwrap_or(Value::Null);
        let mut refs: Vec<String> = recent
            .get("result_refs")
            .and_then(|r| r.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .filter(|s| s.starts_with("source://"))
                    .collect()
            })
            .unwrap_or_default();
        if refs.is_empty() {
            refs = envelope
                .active_refs
                .iter()
                .filter(|r| r.starts_with("source://"))
                .cloned()
                .collect();
        }
        intent(
            IntentKind::Retrieve,
            &task_id_of(envelope),
            "read_evidence",
            json!({"op": "read", "ref_id": refs.first().cloned().unwrap_or_default(), "max_tokens": 200}),
            vec![],
            None,
        )
    }

    fn respond_with_quote(envelope: &ContextEnvelope) -> ActionIntent {
        let excerpt = envelope
            .recent_results
            .last()
            .and_then(|r| r.get("result_preview"))
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string();
        intent(
            IntentKind::Respond,
            &task_id_of(envelope),
            "direct_answer",
            json!({"text": format!("Recovered the key finding: {excerpt}")}),
            vec![],
            None,
        )
    }

    let build = || -> Box<dyn ModelAdapter> {
        let steps = vec![
            Step::Intent(Box::new(intent(
                IntentKind::Retrieve,
                "unknown",
                "information_needed",
                json!({"op": "search", "query": "sparse context policy preserved hard constraint", "limit": 5}),
                vec![],
                None,
            ))),
            Step::Dyn(Box::new(read_top_result)),
            Step::Dyn(Box::new(respond_with_quote)),
        ];
        Box::new(ScriptedModel::new(steps))
    };
    FixtureSpec {
        name: "context-retrieval".into(),
        goal: "What did the earlier work conclude about sparse-context cost and constraints?"
            .into(),
        constraints: vec!["Final answer must quote the earlier finding verbatim.".into()],
        acceptance_rules: vec![],
        budget: Budget::default(),
        build_model: Box::new(build),
        setup: Some(setup_long_history),
        description: "Exact passage retrieval from long past history; pinned constraints survive."
            .into(),
    }
}

pub fn get_fixture(name: &str) -> Result<FixtureSpec> {
    match name {
        "direct-answer" => Ok(fixture_direct_answer()),
        "tool-use" => Ok(fixture_tool_use()),
        "failed-tool" => Ok(fixture_failed_tool()),
        "approval" => Ok(fixture_approval()),
        "context-retrieval" => Ok(fixture_context_retrieval()),
        other => Err(crate::error::SfaiError::NotFound(format!(
            "unknown fixture {other:?}; choose from direct-answer, tool-use, failed-tool, approval, context-retrieval"
        ))),
    }
}

pub const FIXTURE_NAMES: [&str; 5] = [
    "direct-answer",
    "tool-use",
    "failed-tool",
    "approval",
    "context-retrieval",
];
