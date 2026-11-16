//! Deterministic fixture tools (plan M0 fake tools + M1 first capabilities).
//!
//! These implement the first capability families with real side-effect
//! semantics against the stores — sources and artifacts really are persisted,
//! indexed, and verifiable — while remaining fully offline and replayable.

use crate::capabilities::registry::{
    CapabilityRegistry, Handler, ToolExecError, ToolExecutionResult,
};
use crate::context::retrieval::ContextRetriever;
use crate::contracts::{CapabilityManifest, CapabilityOperation, RiskClass, TrustLevel};
use crate::error::SfaiError;
use crate::ids::{artifact_ref, refs_in_text, source_ref};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

pub const CANNED_CLAIMS: [(&str, &str, &str); 3] = [
    (
        "Paper A",
        "https://example.org/a",
        "Sparse context activation reduces end-to-end cost by 30% while preserving task success on long-horizon work.",
    ),
    (
        "Paper B",
        "https://example.org/b",
        "Index-native context retains hard constraints across long sessions; full-history replay is unnecessary for most tasks.",
    ),
    (
        "Paper C",
        "https://example.org/c",
        "Model-native routing matches specialist routers on success rate at lower latency, though evidence is mixed on rare domains.",
    ),
];

pub fn default_results(query: &str) -> Vec<(String, String, String)> {
    let short: String = query.chars().take(24).collect();
    CANNED_CLAIMS
        .iter()
        .map(|(title, url, text)| {
            (
                format!("{title} ({short})"),
                url.to_string(),
                text.to_string(),
            )
        })
        .collect()
}

fn tool_err(e: SfaiError) -> ToolExecError {
    ToolExecError {
        message: e.to_string(),
        error_code: "TOOL_ERROR".into(),
    }
}

pub struct FixtureWiring {
    pub side_effect_log: Arc<Mutex<Vec<String>>>,
    pub tool_counts: Arc<Mutex<BTreeMap<String, u32>>>,
}

pub fn register_fixture_tools(
    registry: &mut CapabilityRegistry,
    retriever: &ContextRetriever,
) -> FixtureWiring {
    let side_effect_log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let tool_counts: Arc<Mutex<BTreeMap<String, u32>>> = Arc::new(Mutex::new(BTreeMap::new()));

    let mut handlers: HashMap<String, Handler> = HashMap::new();

    {
        let retriever_search = ContextRetriever::clone(retriever);
        let counts = tool_counts.clone();
        handlers.insert(
            "web.search".into(),
            Box::new(move |ctx, args| {
                *counts
                    .lock()
                    .expect("counts mutex poisoned")
                    .entry("web.search".to_string())
                    .or_insert(0) += 1;
                let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
                let results = default_results(query);
                let mut refs: Vec<String> = Vec::new();
                let mut data: Vec<Value> = Vec::new();
                for (title, url, text) in &results {
                    let source = ctx
                        .objects
                        .put_source(
                            &ctx.task_id,
                            url,
                            text.as_bytes(),
                            "text/plain",
                            "web",
                            TrustLevel::Untrusted,
                        )
                        .map_err(tool_err)?;
                    let reference = source_ref(&source.source_id, 1);
                    retriever_search
                        .index_document(&reference, &ctx.task_id, "source", title, text, 1)
                        .map_err(tool_err)?;
                    refs.push(reference.clone());
                    data.push(
                        json!({"title": title, "url": url, "text": text, "ref_id": reference}),
                    );
                }
                let titles: Vec<String> = results.iter().map(|(t, _, _)| t.clone()).collect();
                Ok(ToolExecutionResult {
                    preview: format!(
                        "{} result(s) for {:?}: {}",
                        results.len(),
                        query,
                        titles.join("; ")
                    ),
                    result_refs: refs,
                    result_data: json!({"results": data}),
                    ..Default::default()
                })
            }),
        );

        handlers.insert(
            "web.open".into(),
            Box::new(move |ctx, args| {
                let ref_id = args.get("ref_id").and_then(|r| r.as_str()).unwrap_or("");
                let Some((text, provenance)) =
                    ctx.objects.resolve_ref_text(ref_id).map_err(tool_err)?
                else {
                    return Err(ToolExecError {
                        message: format!("cannot open ref: {ref_id}"),
                        error_code: "UNRESOLVABLE_REF".into(),
                    });
                };
                Ok(ToolExecutionResult {
                    preview: text.chars().take(400).collect(),
                    result_refs: vec![ref_id.to_string()],
                    result_data: json!({"text": text, "provenance": provenance}),
                    ..Default::default()
                })
            }),
        );
    }

    {
        let retriever_write = ContextRetriever::clone(retriever);
        let effects = side_effect_log.clone();
        let counts = tool_counts.clone();
        handlers.insert(
            "file.write".into(),
            Box::new(move |ctx, args| {
                *counts
                    .lock()
                    .expect("counts mutex poisoned")
                    .entry("file.write".to_string())
                    .or_insert(0) += 1;
                let path = args
                    .get("path")
                    .and_then(|p| p.as_str())
                    .unwrap_or("output.md");
                let content = args.get("content").and_then(|c| c.as_str()).unwrap_or("");
                let provenance = refs_in_text(content);
                let artifact = ctx
                    .objects
                    .put_artifact(
                        &ctx.task_id,
                        path,
                        content.as_bytes(),
                        "text/markdown",
                        &provenance,
                    )
                    .map_err(tool_err)?;
                let reference = artifact_ref(&artifact.artifact_id, 1);
                retriever_write
                    .index_document(&reference, &ctx.task_id, "artifact", path, content, 1)
                    .map_err(tool_err)?;
                effects
                    .lock()
                    .expect("effects mutex poisoned")
                    .push(format!("file.written:{path}"));
                Ok(ToolExecutionResult {
                    preview: format!(
                        "artifact written: {path} ({} chars)",
                        content.chars().count()
                    ),
                    result_refs: vec![reference.clone()],
                    side_effects: vec![format!("file.written:{path}")],
                    receipt: Some(json!({"path": path, "checksum": artifact.checksum})),
                    state_events: vec![json!({"type": "output_added", "ref": reference})],
                    result_data: json!({"ref_id": reference, "checksum": artifact.checksum}),
                })
            }),
        );
    }

    {
        let counts = tool_counts.clone();
        handlers.insert(
            "file.read".into(),
            Box::new(move |ctx, args| {
                *counts
                    .lock()
                    .expect("counts mutex poisoned")
                    .entry("file.read".to_string())
                    .or_insert(0) += 1;
                let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
                let Some(artifact) = ctx
                    .objects
                    .find_artifact_by_path(&ctx.task_id, path)
                    .map_err(tool_err)?
                else {
                    return Err(ToolExecError {
                        message: format!("no artifact at path: {path}"),
                        error_code: "FILE_NOT_FOUND".into(),
                    });
                };
                let text = ctx
                    .objects
                    .read_artifact_text(&artifact.artifact_id)
                    .map_err(tool_err)?;
                Ok(ToolExecutionResult {
                    preview: text.chars().take(400).collect(),
                    result_refs: vec![artifact_ref(&artifact.artifact_id, artifact.version)],
                    result_data: json!({"text": text}),
                    ..Default::default()
                })
            }),
        );
    }

    {
        let effects = side_effect_log.clone();
        let counts = tool_counts.clone();
        handlers.insert(
            "mail.send".into(),
            Box::new(move |_ctx, args| {
                *counts
                    .lock()
                    .expect("counts mutex poisoned")
                    .entry("mail.send".to_string())
                    .or_insert(0) += 1;
                let to = args.get("to").and_then(|t| t.as_str()).unwrap_or("");
                let body = args.get("body").and_then(|b| b.as_str()).unwrap_or("");
                effects
                    .lock()
                    .expect("effects mutex poisoned")
                    .push(format!("mail.sent:{to}"));
                Ok(ToolExecutionResult {
                    preview: format!("mail sent to {to} ({} chars)", body.chars().count()),
                    side_effects: vec![format!("mail.sent:{to}")],
                    receipt: Some(json!({"to": to, "chars": body.chars().count()})),
                    result_data: json!({"to": to}),
                    ..Default::default()
                })
            }),
        );
    }

    {
        let counts = tool_counts.clone();
        handlers.insert(
            "fail.always".into(),
            Box::new(move |_ctx, _args| {
                *counts
                    .lock()
                    .expect("counts mutex poisoned")
                    .entry("fail.always".to_string())
                    .or_insert(0) += 1;
                Err(ToolExecError {
                    message: "scripted persistent failure".into(),
                    error_code: "SCRIPTED_FAILURE".into(),
                })
            }),
        );
    }

    let manifests = vec![
        (
            CapabilityManifest {
                id: "web.research".into(),
                version: "0.1.0".into(),
                description: "Find, open, and cite web evidence".into(),
                operations: vec![
                    CapabilityOperation {
                        id: "web.search".into(),
                        description: "Search the web; results are stored as sources.".into(),
                        input_schema: json!({"query": "string"}),
                        risk_class: RiskClass::ReadOnly,
                        side_effects: vec![],
                    },
                    CapabilityOperation {
                        id: "web.open".into(),
                        description: "Open a stored source by ref.".into(),
                        input_schema: json!({"ref_id": "string"}),
                        risk_class: RiskClass::ReadOnly,
                        side_effects: vec![],
                    },
                ],
                permissions: BTreeMap::from([("network".to_string(), true)]),
                risk_class: RiskClass::ReadOnly,
                output_contract: BTreeMap::from([
                    ("persistent_raw_output".to_string(), true),
                    ("return_preview_and_ref".to_string(), true),
                ]),
                healthcheck: Some("health/web_research".into()),
            },
            HashMap::from([
                (
                    "web.search".to_string(),
                    take_handler(&mut handlers, "web.search"),
                ),
                (
                    "web.open".to_string(),
                    take_handler(&mut handlers, "web.open"),
                ),
            ]),
        ),
        (
            CapabilityManifest {
                id: "filesystem.artifacts".into(),
                version: "0.1.0".into(),
                description: "Read/write output artifacts with provenance".into(),
                operations: vec![
                    CapabilityOperation {
                        id: "file.write".into(),
                        description: "Write an artifact into the task workspace.".into(),
                        input_schema: json!({"path": "string", "content": "string"}),
                        risk_class: RiskClass::ReversibleWrite,
                        side_effects: vec!["file.written".into()],
                    },
                    CapabilityOperation {
                        id: "file.read".into(),
                        description: "Read a previously written artifact.".into(),
                        input_schema: json!({"path": "string"}),
                        risk_class: RiskClass::ReadOnly,
                        side_effects: vec![],
                    },
                ],
                permissions: BTreeMap::from([("filesystem".to_string(), true)]),
                risk_class: RiskClass::ReversibleWrite,
                output_contract: BTreeMap::from([
                    ("persistent_raw_output".to_string(), true),
                    ("return_preview_and_ref".to_string(), true),
                ]),
                healthcheck: None,
            },
            HashMap::from([
                (
                    "file.write".to_string(),
                    take_handler(&mut handlers, "file.write"),
                ),
                (
                    "file.read".to_string(),
                    take_handler(&mut handlers, "file.read"),
                ),
            ]),
        ),
        (
            CapabilityManifest {
                id: "external_apps".into(),
                version: "0.1.0".into(),
                description: "External side-effecting connectors (explicit consent required)"
                    .into(),
                operations: vec![CapabilityOperation {
                    id: "mail.send".into(),
                    description: "Send an email. Requires explicit user approval.".into(),
                    input_schema: json!({"to": "string", "body": "string"}),
                    risk_class: RiskClass::ExternalWrite,
                    side_effects: vec!["mail.sent".into()],
                }],
                permissions: BTreeMap::from([("network".to_string(), true)]),
                risk_class: RiskClass::ExternalWrite,
                output_contract: BTreeMap::new(),
                healthcheck: None,
            },
            HashMap::from([(
                "mail.send".to_string(),
                take_handler(&mut handlers, "mail.send"),
            )]),
        ),
        (
            CapabilityManifest {
                id: "debug.fail".into(),
                version: "0.1.0".into(),
                description: "Deterministic failing operation for retry/blocked-state tests".into(),
                operations: vec![CapabilityOperation {
                    id: "fail.always".into(),
                    description: "Always fails with SCRIPTED_FAILURE.".into(),
                    input_schema: json!({}),
                    risk_class: RiskClass::ReadOnly,
                    side_effects: vec![],
                }],
                permissions: BTreeMap::new(),
                risk_class: RiskClass::ReadOnly,
                output_contract: BTreeMap::new(),
                healthcheck: None,
            },
            HashMap::from([(
                "fail.always".to_string(),
                take_handler(&mut handlers, "fail.always"),
            )]),
        ),
    ];

    for (manifest, handlers) in manifests {
        registry.register(manifest, handlers);
    }

    FixtureWiring {
        side_effect_log,
        tool_counts,
    }
}

fn take_handler(handlers: &mut HashMap<String, Handler>, operation: &str) -> Handler {
    handlers
        .remove(operation)
        .unwrap_or_else(|| panic!("missing handler for {operation}"))
}
