//! Agent framework benchmark (v0.1) — "Agent Artificial Analysis"-style.
//!
//! Measures, per task suite and per framework ablation: task completion,
//! ground-truth correctness, wall-clock latency (p50/p95), input/output tokens,
//! cost, false-success rate, and cost/token per success. Runs end-to-end
//! through the real kernel (stores, broker, verification, wiki) with
//! deterministic scripted models so results are reproducible offline.
//!
//! v0 measures *framework* behaviour; real-model runs (v1) plug the same
//! suites into the provider adapter. See `docs/benchmark-spec.md`.

use crate::app::{build_app, App};
use crate::contracts::{Budget, IntentKind, TaskStatus, TrustLevel};
use crate::core::action_intents::intent;
use crate::core::model_adapter::ModelAdapter;
use crate::integrations::providers::fake::{ScriptedModel, Step};
use crate::storage::event_ledger::now;
use serde::Serialize;
use serde_json::{json, Value};
use std::time::Instant;

pub const BENCH_VERSION: &str = "0.1.0";
pub const DEFAULT_SIZE: usize = 6;
pub const SUITES: [&str; 4] = ["tool-use", "dataset", "long-horizon", "edge-case"];
pub const ABLATIONS: [&str; 5] = [
    "none",
    "full_context",
    "no_pinned",
    "no_verification",
    "no_wiki",
];

#[derive(Clone, Debug, Serialize)]
pub struct BenchSpec {
    pub id: String,
    pub suite: String,
    pub goal: String,
    pub constraints: Vec<String>,
    pub acceptance_rules: Vec<String>,
    /// Ground-truth marker that a correct deliverable must contain.
    pub ground_truth: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct BenchTaskResult {
    pub task_id: String,
    pub spec_id: String,
    pub suite: String,
    pub ablation: String,
    pub status: String,
    pub acceptance_passed: bool,
    pub correct: bool,
    pub false_success: bool,
    pub latency_ms: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub tool_calls: u64,
    pub cost_usd: f64,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct BenchSuiteMetrics {
    pub suite: String,
    pub ablation: String,
    pub tasks: usize,
    pub success_rate: f64,
    pub correctness: f64,
    pub false_success_rate: f64,
    pub mean_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub mean_input_tokens: f64,
    pub mean_output_tokens: f64,
    pub mean_cost_usd: f64,
    pub cost_per_success_usd: f64,
    pub tokens_per_success: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct BenchReport {
    pub version: String,
    pub generated_at: String,
    pub suites: Vec<String>,
    pub ablations: Vec<String>,
    pub size: usize,
    pub metrics: Vec<BenchSuiteMetrics>,
    pub tasks: Vec<BenchTaskResult>,
    pub notes: String,
}

impl BenchReport {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn render_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "# Agent framework benchmark — v{}\n\n- generated_at: {}\n- tasks per suite: {}\n\n",
            self.version, self.generated_at, self.size
        ));
        out.push_str(
            "| suite | ablation | n | success | correct | false-success | p50 ms | p95 ms | in tokens | out tokens | cost USD | cost/success USD |\n",
        );
        out.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|\n");
        for m in &self.metrics {
            out.push_str(&format!(
                "| {} | {} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.0} | {:.0} | {:.6} | {:.6} |\n",
                m.suite,
                m.ablation,
                m.tasks,
                m.success_rate,
                m.correctness,
                m.false_success_rate,
                m.p50_latency_ms,
                m.p95_latency_ms,
                m.mean_input_tokens,
                m.mean_output_tokens,
                m.mean_cost_usd,
                m.cost_per_success_usd
            ));
        }
        out.push_str("\n## Notes\n\n");
        out.push_str(&self.notes);
        out.push('\n');
        out
    }
}

fn dataset_total(i: usize) -> u64 {
    let mut total = 0u64;
    for row in 0..8 {
        total += ((i * 7 + row * 13) % 97 + 10) as u64;
    }
    total
}

pub fn generate_tasks(suite: &str, size: usize) -> Vec<BenchSpec> {
    match suite {
        "tool-use" => (0..size)
            .map(|i| BenchSpec {
                id: format!("tool-{i:02}"),
                suite: "tool-use".into(),
                goal: format!(
                    "Find three recent sources on sparse context activation and write a cited report (case {i})."
                ),
                constraints: vec!["Final output must be a markdown report with citations.".into()],
                acceptance_rules: vec![
                    "artifact_exists:report.md".into(),
                    "citations_resolve".into(),
                    "source_count_at_least:3".into(),
                ],
                ground_truth: "Citations".into(),
            })
            .collect(),
        "dataset" => (0..size)
            .map(|i| {
                let total = dataset_total(i);
                BenchSpec {
                    id: format!("data-{i:02}"),
                    suite: "dataset".into(),
                    goal: format!("Compute the total revenue from sales.csv and write answer.json (case {i})."),
                    constraints: vec!["Answer must be JSON with the computed total.".into()],
                    acceptance_rules: vec![
                        "artifact_exists:answer.json".into(),
                        "json_valid:answer.json".into(),
                        format!("artifact_contains:answer.json:{total}"),
                    ],
                    ground_truth: total.to_string(),
                }
            })
            .collect(),
        "long-horizon" => (0..size)
            .map(|i| {
                let marker = format!("LH-{i:02}-NEEDLE");
                BenchSpec {
                    id: format!("long-{i:02}"),
                    suite: "long-horizon".into(),
                    goal: format!(
                        "Recover the earlier retention finding requiring {marker} and write a short report quoting it."
                    ),
                    constraints: vec!["Quote the earlier finding verbatim.".into()],
                    acceptance_rules: vec![
                        "artifact_exists:report.md".into(),
                        format!("artifact_contains:report.md:{marker}"),
                    ],
                    ground_truth: marker,
                }
            })
            .collect(),
        "edge-case" => (0..size)
            .map(|i| BenchSpec {
                id: format!("edge-{i:02}"),
                suite: "edge-case".into(),
                goal: format!("Write a report citing a source (case {i})."),
                constraints: vec!["Every citation must resolve.".into()],
                acceptance_rules: vec![
                    "artifact_exists:report.md".into(),
                    "citations_resolve".into(),
                ],
                // Deliberately broken deliverable: unresolvable citation.
                ground_truth: "source://missing@1".into(),
            })
            .collect(),
        other => panic!("unknown suite: {other}"),
    }
}

fn task_id_of(envelope: &crate::contracts::ContextEnvelope) -> String {
    envelope
        .root_index
        .get("task")
        .and_then(|r| r.strip_prefix("task://"))
        .and_then(|r| r.split('/').next())
        .unwrap_or("unknown")
        .to_string()
}

fn build_model(
    suite: &str,
    spec: &BenchSpec,
    provider: Option<&crate::integrations::providers::profiles::ProviderProfile>,
) -> Box<dyn ModelAdapter> {
    if let Some(profile) = provider {
        let adapter =
            crate::integrations::providers::openai_compat::OpenAICompatibleAdapter::from_profile(
                profile,
            )
            .unwrap_or_else(|e| panic!("invalid provider profile: {e}"));
        return Box::new(adapter);
    }
    match suite {
        "tool-use" => Box::new(crate::integrations::providers::fake::ReactiveDemoModel::new()),
        "dataset" => {
            let content =
                json!({"total": spec.ground_truth.parse::<u64>().unwrap_or(0)}).to_string();
            Box::new(ScriptedModel::new(vec![
                Step::Intent(Box::new(intent(
                    IntentKind::Execute,
                    "unknown",
                    "deliver",
                    json!({"operation_id": "file.write", "arguments": {"path": "answer.json", "content": content}}),
                    vec![],
                    Some(format!("{}-answer", spec.id)),
                ))),
                Step::Intent(Box::new(intent(
                    IntentKind::Finish,
                    "unknown",
                    "done",
                    json!({"text": "answer written"}),
                    vec![],
                    None,
                ))),
            ]))
        }
        "long-horizon" => {
            let marker = spec.ground_truth.clone();
            let id = spec.id.clone();
            Box::new(ScriptedModel::new(vec![
                Step::Intent(Box::new(intent(
                    IntentKind::Retrieve,
                    "unknown",
                    "find",
                    json!({"op": "search", "query": marker, "limit": 3}),
                    vec![],
                    None,
                ))),
                Step::Dyn(Box::new(|envelope| {
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
                                .filter(|s| s.starts_with("source://"))
                                .collect()
                        })
                        .unwrap_or_default();
                    intent(
                        IntentKind::Retrieve,
                        &task_id_of(envelope),
                        "read",
                        json!({"op": "read", "ref_id": refs.first().cloned().unwrap_or_default(), "max_tokens": 200}),
                        vec![],
                        None,
                    )
                })),
                Step::Dyn(Box::new(move |envelope| {
                    let excerpt = envelope
                        .recent_results
                        .last()
                        .and_then(|r| r.get("result_preview"))
                        .and_then(|p| p.as_str())
                        .unwrap_or("")
                        .to_string();
                    intent(
                        IntentKind::Execute,
                        &task_id_of(envelope),
                        "deliver",
                        json!({"operation_id": "file.write", "arguments": {"path": "report.md", "content": format!("# Report\n\n{excerpt}\n")}}),
                        vec![],
                        Some(format!("{id}-report")),
                    )
                })),
                Step::Intent(Box::new(intent(
                    IntentKind::Finish,
                    "unknown",
                    "done",
                    json!({"text": "report written"}),
                    vec![],
                    None,
                ))),
            ]))
        }
        "edge-case" => Box::new(ScriptedModel::new(vec![
            Step::Intent(Box::new(intent(
                IntentKind::Execute,
                "unknown",
                "deliver",
                json!({"operation_id": "file.write", "arguments": {"path": "report.md", "content": "# Report\n\n- source: source://missing@1\n"}}),
                vec![],
                Some(format!("{}-broken-report", spec.id)),
            ))),
            Step::Intent(Box::new(intent(
                IntentKind::Finish,
                "unknown",
                "done",
                json!({"text": "report written"}),
                vec![],
                None,
            ))),
            Step::Intent(Box::new(intent(
                IntentKind::Finish,
                "unknown",
                "done",
                json!({"text": "report written"}),
                vec![],
                None,
            ))),
            Step::Intent(Box::new(intent(
                IntentKind::Finish,
                "unknown",
                "done",
                json!({"text": "report written"}),
                vec![],
                None,
            ))),
        ])),
        other => panic!("unknown suite: {other}"),
    }
}

fn seed_task(app: &App, spec: &BenchSpec, task_id: &str) {
    match spec.suite.as_str() {
        "dataset" => {
            let mut csv = String::from("row,amount\n");
            for row in 0..8 {
                csv.push_str(&format!(
                    "{row},{}\n",
                    (spec_index(spec) * 7 + row * 13) % 97 + 10
                ));
            }
            let _ = app
                .objects
                .put_artifact(task_id, "sales.csv", csv.as_bytes(), "text/csv", &[]);
        }
        "long-horizon" => {
            let needle_pos = 20;
            for i in 0..40 {
                let body = if i == needle_pos {
                    format!(
                        "Earlier finding {}: the retention figure holds under the sparse-context policy.",
                        spec.ground_truth
                    )
                } else {
                    format!("Routine note {i}: intermediate observations, no material finding yet.")
                };
                if let Ok(source) = app.objects.put_source(
                    task_id,
                    &format!("note://{i}"),
                    body.as_bytes(),
                    "text/plain",
                    "system",
                    TrustLevel::Trusted,
                ) {
                    let _ = app.retriever.index_document(
                        &crate::ids::source_ref(&source.source_id, 1),
                        task_id,
                        "source",
                        &format!("Note {i}"),
                        &body,
                        1,
                    );
                }
            }
            let _ = app
                .task_store
                .checkpoint(task_id, "history seeded", Some(&[]));
        }
        _ => {}
    }
}

fn expected_artifact_paths(spec: &BenchSpec) -> Vec<String> {
    spec.acceptance_rules
        .iter()
        .filter_map(|rule| {
            if let Some(path) = rule.strip_prefix("artifact_exists:") {
                Some(path.to_string())
            } else if let Some(rest) = rule.strip_prefix("artifact_contains:") {
                rest.split(':').next().map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn spec_index(spec: &BenchSpec) -> usize {
    spec.id
        .rsplit('-')
        .next()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0)
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

pub fn run_benchmark(suites: &[String], ablations: &[String], size: usize) -> BenchReport {
    run_benchmark_with_provider(suites, ablations, size, None)
}

pub fn run_benchmark_with_provider(
    suites: &[String],
    ablations: &[String],
    size: usize,
    provider: Option<&crate::integrations::providers::profiles::ProviderProfile>,
) -> BenchReport {
    let mut tasks: Vec<BenchTaskResult> = Vec::new();
    for suite in suites {
        let specs = generate_tasks(suite, size);
        for ablation in ablations {
            for spec in &specs {
                let dir = match tempfile::tempdir() {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                let mut app = match build_app(dir.path()) {
                    Ok(a) => a,
                    Err(_) => continue,
                };
                match ablation.as_str() {
                    "full_context" => {
                        app.envelope_builder.active_context_tokens = 100_000;
                        app.envelope_builder.candidate_limit = 1_000;
                    }
                    "no_pinned" => app.envelope_builder.pin_constraints = false,
                    "no_verification" => app.enforce_acceptance = false,
                    "no_wiki" => app.wiki_compile_enabled = false,
                    _ => {}
                }
                let task = match app.create_task(
                    &spec.goal,
                    spec.constraints.clone(),
                    spec.acceptance_rules.clone(),
                    Some(Budget::default()),
                ) {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                seed_task(&app, spec, &task.task_id);
                let mut model = build_model(suite, spec, provider);
                let mut observer = crate::core::decision_kernel::NoopObserver;
                let started = Instant::now();
                let result = app.run_observed(model.as_mut(), &task.task_id, 50, &mut observer);
                let latency_ms = started.elapsed().as_secs_f64() * 1000.0;

                let status = match &result {
                    Ok(r) => r.status.as_str().to_string(),
                    Err(_) => "error".to_string(),
                };
                let (acceptance_passed, correct) = match &result {
                    Ok(_) => {
                        let acceptance = app
                            .verifier
                            .verify(&task.task_id, &spec.acceptance_rules)
                            .map(|results| {
                                results
                                    .iter()
                                    .all(|r| r.verdict == crate::contracts::Verdict::Pass)
                            })
                            .unwrap_or(false);
                        let expected_paths = expected_artifact_paths(spec);
                        let correct = app
                            .objects
                            .task_artifacts(&task.task_id)
                            .map(|artifacts| {
                                artifacts
                                    .iter()
                                    .filter(|a| expected_paths.iter().any(|p| p == &a.path))
                                    .any(|a| {
                                        app.objects
                                            .read_artifact_text(&a.artifact_id)
                                            .map(|text| text.contains(&spec.ground_truth))
                                            .unwrap_or(false)
                                    })
                            })
                            .unwrap_or(false);
                        (acceptance, correct)
                    }
                    Err(_) => (false, false),
                };
                let completed = status == "completed";
                let false_success = completed && !(acceptance_passed && correct);
                let cost = match &result {
                    Ok(r) => r.cost.clone(),
                    Err(_) => Value::Null,
                };
                let num = |key: &str| cost.get(key).and_then(|v| v.as_u64()).unwrap_or(0);
                tasks.push(BenchTaskResult {
                    task_id: task.task_id.clone(),
                    spec_id: spec.id.clone(),
                    suite: suite.clone(),
                    ablation: ablation.clone(),
                    status,
                    acceptance_passed,
                    correct,
                    false_success,
                    latency_ms,
                    input_tokens: num("input_tokens"),
                    output_tokens: num("output_tokens"),
                    tool_calls: num("tool_calls"),
                    cost_usd: cost.get("cost_usd").and_then(|v| v.as_f64()).unwrap_or(0.0),
                });
            }
        }
    }

    let mut metrics: Vec<BenchSuiteMetrics> = Vec::new();
    let mut keys: Vec<(String, String)> = Vec::new();
    for task in &tasks {
        let key = (task.suite.clone(), task.ablation.clone());
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys.sort();
    for (suite, ablation) in keys {
        let group: Vec<&BenchTaskResult> = tasks
            .iter()
            .filter(|t| t.suite == suite && t.ablation == ablation)
            .collect();
        let n = group.len().max(1) as f64;
        let successes = group.iter().filter(|t| t.status == "completed").count() as f64;
        let correct = group.iter().filter(|t| t.correct).count() as f64;
        let false_success = group.iter().filter(|t| t.false_success).count() as f64;
        let mut latencies: Vec<f64> = group.iter().map(|t| t.latency_ms).collect();
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mean_cost = group.iter().map(|t| t.cost_usd).sum::<f64>() / n;
        let mean_in = group.iter().map(|t| t.input_tokens as f64).sum::<f64>() / n;
        let mean_out = group.iter().map(|t| t.output_tokens as f64).sum::<f64>() / n;
        metrics.push(BenchSuiteMetrics {
            suite,
            ablation,
            tasks: group.len(),
            success_rate: successes / n,
            correctness: correct / n,
            false_success_rate: false_success / n,
            mean_latency_ms: group.iter().map(|t| t.latency_ms).sum::<f64>() / n,
            p50_latency_ms: percentile(&latencies, 0.50),
            p95_latency_ms: percentile(&latencies, 0.95),
            mean_input_tokens: mean_in,
            mean_output_tokens: mean_out,
            mean_cost_usd: mean_cost,
            cost_per_success_usd: if successes > 0.0 {
                mean_cost / (successes / n)
            } else {
                0.0
            },
            tokens_per_success: if successes > 0.0 {
                (mean_in + mean_out) / (successes / n)
            } else {
                0.0
            },
        });
    }

    BenchReport {
        version: BENCH_VERSION.to_string(),
        generated_at: now(),
        suites: suites.to_vec(),
        ablations: ablations.to_vec(),
        size,
        metrics,
        tasks,
        notes: "Scripted deterministic models end-to-end through the real kernel; v0 measures \
                framework behaviour (success/correctness/latency/tokens/cost), not model quality. \
                Real-model runs are v1; see docs/benchmark-spec.md."
            .to_string(),
    }
}

#[allow(dead_code)]
fn _status_marker() -> TaskStatus {
    TaskStatus::Active
}
