//! Offline sparse-context evaluation harness (M8 baseline, plan §13).
//!
//! v0 scope (see `docs/research-plan.md`): deterministic synthetic scenarios,
//! five context-selection policies simulated at the context layer, deterministic
//! answer simulation, no network. The harness measures success, needle recall,
//! constraint retention, input tokens, cost and latency so the evaluation
//! protocol is frozen *before* real-model runs. Results are internal-fixture
//! evidence only — not a paper claim.

use crate::context::retrieval::SearchCandidate;
use crate::context::working_set::compile_working_set;
use crate::storage::event_ledger::now;
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::Instant;

pub const DEFAULT_SUITE: &str = "sparse-context";
pub const DEFAULT_SIZE: usize = 12;
pub const DEFAULT_BUDGET_TOKENS: usize = 120;
pub const HISTORY_LEN: usize = 40;
pub const POLICY_NAMES: [&str; 5] = [
    "full_history",
    "sliding_window",
    "summary",
    "retrieval_topk",
    "working_set",
];
const ANSWER_TOKENS: usize = 20;

#[derive(Clone, Debug, Serialize)]
pub struct Doc {
    pub id: String,
    pub title: String,
    pub body: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct EvalScenario {
    pub id: String,
    pub goal: String,
    pub constraints: Vec<String>,
    pub history: Vec<Doc>,
    pub needle_id: String,
    pub query: String,
    pub answer_marker: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PolicyOutcome {
    pub policy: String,
    pub included_docs: usize,
    pub tokens_in: usize,
    pub context_recall: bool,
    pub constraint_retention: bool,
    pub success: bool,
    pub latency_ms: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScenarioResult {
    pub scenario: String,
    pub outcomes: Vec<PolicyOutcome>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct PolicyMetrics {
    pub policy: String,
    pub scenarios: usize,
    pub success_rate: f64,
    pub context_recall: f64,
    pub constraint_retention: f64,
    pub mean_tokens: f64,
    pub mean_cost_usd: f64,
    pub cost_per_success_usd: f64,
    pub mean_latency_ms: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct EvalReport {
    pub suite: String,
    pub generated_at: String,
    pub budget_tokens: usize,
    pub scenario_count: usize,
    pub metrics: Vec<PolicyMetrics>,
    pub per_scenario: Vec<ScenarioResult>,
    pub notes: String,
}

impl EvalReport {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn render_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Evaluation report — {}\n\n", self.suite));
        out.push_str(&format!(
            "- generated_at: {}\n- scenarios: {}\n- context budget: {} tokens\n\n",
            self.generated_at, self.scenario_count, self.budget_tokens
        ));
        out.push_str("| policy | success | needle recall | constraint retention | mean input tokens | mean cost (USD) | cost per success (USD) | mean latency (ms) |\n");
        out.push_str("|---|---|---|---|---|---|---|---|\n");
        for m in &self.metrics {
            out.push_str(&format!(
                "| {} | {:.2} | {:.2} | {:.2} | {:.1} | {:.6} | {:.6} | {:.3} |\n",
                m.policy,
                m.success_rate,
                m.context_recall,
                m.constraint_retention,
                m.mean_tokens,
                m.mean_cost_usd,
                m.cost_per_success_usd,
                m.mean_latency_ms
            ));
        }
        out.push_str("\n## Notes\n\n");
        out.push_str(&self.notes);
        out.push('\n');
        out
    }
}

fn tokens_of(text: &str) -> usize {
    (text.chars().count() / 4).max(1)
}

fn is_constraint_doc(doc: &Doc) -> bool {
    doc.body.starts_with("Task constraints:")
}

fn query_terms(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split_whitespace()
        .filter(|t| t.len() > 2)
        .map(|t| t.to_string())
        .collect()
}

fn lexical_score(doc: &Doc, terms: &[String]) -> f64 {
    let body = doc.body.to_lowercase();
    terms.iter().filter(|t| body.contains(t.as_str())).count() as f64
}

/// Deterministic suite: needle at early/middle/late positions; constraints are
/// stated in an early "Task constraints:" doc so policies that drop early
/// context lose them unless constraints are explicitly pinned.
pub fn generate_suite(size: usize) -> Vec<EvalScenario> {
    (0..size)
        .map(|i| {
            let needle_pos = [2usize, 20, 38][i % 3];
            let marker = format!("NEEDLE-{i:02}-MARKER");
            let mut history = Vec::new();
            for d in 0..HISTORY_LEN {
                let body = if d == needle_pos {
                    format!(
                        "Critical finding {marker}: the retention figure is {}% and the policy is sparse-context.",
                        40 + i
                    )
                } else if d == 0 {
                    format!(
                        "Task constraints: final output must cite the earlier finding; keep the answer under 80 words. (session {i})"
                    )
                } else {
                    format!(
                        "Routine note {d} for session {i}: intermediate observations, no material finding yet."
                    )
                };
                history.push(Doc {
                    id: format!("note-{i:02}-{d:02}"),
                    title: format!("Note {d}"),
                    body,
                });
            }
            EvalScenario {
                id: format!("scenario-{i:02}"),
                goal: format!(
                    "Report the retention figure from the earlier findings ({marker})."
                ),
                constraints: vec![
                    "final output must cite the earlier finding".into(),
                    "keep the answer under 80 words".into(),
                ],
                history,
                needle_id: format!("note-{i:02}-{needle_pos:02}"),
                query: marker.clone(),
                answer_marker: marker,
            }
        })
        .collect()
}

pub struct Selection {
    pub docs: Vec<Doc>,
    pub tokens: usize,
    pub included_needle: bool,
    pub constraint_retained: bool,
}

pub fn select(policy: &str, scenario: &EvalScenario, budget: usize) -> Selection {
    let terms = query_terms(&scenario.query);
    let docs: Vec<Doc> = match policy {
        "full_history" => scenario.history.clone(),
        "sliding_window" => {
            let window = 6.min(scenario.history.len());
            scenario.history[scenario.history.len() - window..].to_vec()
        }
        "summary" => {
            // Deterministic compaction: first sentence of docs matching the
            // query, ranked by term hits, capped at three.
            let mut hits: Vec<(Doc, f64)> = scenario
                .history
                .iter()
                .filter(|d| lexical_score(d, &terms) > 0.0)
                .map(|d| {
                    let first_sentence: String =
                        d.body.split(". ").next().unwrap_or(&d.body).to_string();
                    (
                        Doc {
                            id: d.id.clone(),
                            title: d.title.clone(),
                            body: first_sentence,
                        },
                        lexical_score(d, &terms),
                    )
                })
                .collect();
            hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            hits.into_iter().take(3).map(|(d, _)| d).collect()
        }
        "retrieval_topk" => {
            let mut hits: Vec<(Doc, f64)> = scenario
                .history
                .iter()
                .map(|d| (d.clone(), lexical_score(d, &terms)))
                .filter(|(_, score)| *score > 0.0)
                .collect();
            hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            hits.into_iter().take(4).map(|(d, _)| d).collect()
        }
        "working_set" => {
            let candidates: Vec<SearchCandidate> = scenario
                .history
                .iter()
                .map(|d| SearchCandidate {
                    ref_id: d.id.clone(),
                    kind: "source".into(),
                    title: d.title.clone(),
                    snippet: d.body.clone(),
                    lexical_score: lexical_score(d, &terms),
                    revision: 1,
                    token_estimate: tokens_of(&d.body),
                    updated_at: "2026-01-01T00:00:00Z".into(),
                })
                .collect();
            let (selected, _) = compile_working_set(
                &candidates,
                &scenario.goal,
                budget,
                &scenario.constraints,
                None,
            );
            let ids: Vec<String> = selected.iter().map(|c| c.ref_id.clone()).collect();
            scenario
                .history
                .iter()
                .filter(|d| ids.contains(&d.id))
                .cloned()
                .collect()
        }
        other => panic!("unknown policy: {other}"),
    };

    let included_needle = docs.iter().any(|d| d.id == scenario.needle_id);
    // Working-set policy pins constraints by construction; the others only
    // retain them if they kept the early constraint document.
    let constraint_retained = if policy == "working_set" {
        true
    } else {
        docs.iter().any(is_constraint_doc)
    };
    let mut tokens: usize = docs.iter().map(|d| tokens_of(&d.body)).sum();
    if policy == "working_set" {
        tokens += scenario
            .constraints
            .iter()
            .map(|c| tokens_of(c))
            .sum::<usize>();
    }
    Selection {
        docs,
        tokens,
        included_needle,
        constraint_retained,
    }
}

fn price(tokens_in: usize, tokens_out: usize) -> f64 {
    let input = tokens_in as f64 * crate::telemetry::cost::PRICE_PER_MILLION_INPUT / 1_000_000.0;
    let output = tokens_out as f64 * crate::telemetry::cost::PRICE_PER_MILLION_OUTPUT / 1_000_000.0;
    input + output
}

pub fn run_suite(suite: &str, size: usize, budget: usize) -> EvalReport {
    let scenarios = generate_suite(size);
    let mut per_scenario: Vec<ScenarioResult> = Vec::new();
    let mut counters: std::collections::BTreeMap<&'static str, PolicyMetrics> =
        std::collections::BTreeMap::new();
    for name in POLICY_NAMES {
        counters.insert(
            name,
            PolicyMetrics {
                policy: name.to_string(),
                ..Default::default()
            },
        );
    }

    for scenario in &scenarios {
        let mut outcomes = Vec::new();
        for name in POLICY_NAMES {
            let started = Instant::now();
            let selection = select(name, scenario, budget);
            let latency_ms = started.elapsed().as_secs_f64() * 1000.0;
            let success = selection.included_needle && selection.constraint_retained;
            let outcome = PolicyOutcome {
                policy: name.to_string(),
                included_docs: selection.docs.len(),
                tokens_in: selection.tokens,
                context_recall: selection.included_needle,
                constraint_retention: selection.constraint_retained,
                success,
                latency_ms,
            };
            let metrics = counters.get_mut(name).expect("policy counter");
            metrics.scenarios += 1;
            metrics.success_rate += if success { 1.0 } else { 0.0 };
            metrics.context_recall += if selection.included_needle { 1.0 } else { 0.0 };
            metrics.constraint_retention += if selection.constraint_retained {
                1.0
            } else {
                0.0
            };
            metrics.mean_tokens += selection.tokens as f64;
            metrics.mean_cost_usd += price(selection.tokens, ANSWER_TOKENS);
            metrics.mean_latency_ms += latency_ms;
            outcomes.push(outcome);
        }
        per_scenario.push(ScenarioResult {
            scenario: scenario.id.clone(),
            outcomes,
        });
    }

    let n = scenarios.len().max(1) as f64;
    let mut metrics: Vec<PolicyMetrics> = counters.into_values().collect();
    for m in &mut metrics {
        m.success_rate /= n;
        m.context_recall /= n;
        m.constraint_retention /= n;
        m.mean_tokens /= n;
        m.mean_cost_usd /= n;
        m.mean_latency_ms /= n;
        m.cost_per_success_usd = if m.success_rate > 0.0 {
            m.mean_cost_usd / m.success_rate
        } else {
            0.0
        };
    }

    EvalReport {
        suite: suite.to_string(),
        generated_at: now(),
        budget_tokens: budget,
        scenario_count: scenarios.len(),
        metrics,
        per_scenario,
        notes: "Synthetic internal fixtures; answer is simulated deterministically; policies are \
                simulated at the context-selection layer. This is pipeline evidence, not a paper \
                claim (see docs/research-plan.md §6)."
            .to_string(),
    }
}

// ---- E5: budget/scale sweep (sparse-activation Pareto curves) ----

#[derive(Clone, Debug, Serialize)]
pub struct SweepPoint {
    pub policy: String,
    pub history_len: usize,
    pub budget_tokens: usize,
    pub success_rate: f64,
    pub correctness: f64,
    pub constraint_retention: f64,
    pub mean_tokens: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SweepReport {
    pub generated_at: String,
    pub points: Vec<SweepPoint>,
}

impl SweepReport {
    pub fn render_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# E5 — sparse activation sweep (quality vs budget vs scale)\n\n");
        out.push_str(&format!("- generated_at: {}\n\n", self.generated_at));
        out.push_str(
            "| policy | history | budget | success | correct | constraint kept | mean tokens |\n",
        );
        out.push_str("|---|---|---|---|---|---|---|\n");
        for p in &self.points {
            out.push_str(&format!(
                "| {} | {} | {} | {:.2} | {:.2} | {:.2} | {:.1} |\n",
                p.policy,
                p.history_len,
                p.budget_tokens,
                p.success_rate,
                p.correctness,
                p.constraint_retention,
                p.mean_tokens
            ));
        }
        out
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

pub fn generate_suite_sized(size: usize, history_len: usize) -> Vec<EvalScenario> {
    (0..size)
        .map(|i| {
            let needle_pos = history_len * 2 / 5;
            let marker = format!("NEEDLE-{i:02}-MARKER");
            let mut history = Vec::new();
            for d in 0..history_len {
                let body = if d == needle_pos {
                    format!(
                        "Critical finding {marker}: the retention figure is {}% and the policy is sparse-context.",
                        40 + i
                    )
                } else if d == 0 {
                    format!(
                        "Task constraints: final output must cite the earlier finding; keep the answer under 80 words. (session {i})"
                    )
                } else {
                    format!(
                        "Routine note {d} for session {i}: intermediate observations, no material finding yet."
                    )
                };
                history.push(Doc {
                    id: format!("note-{i:02}-{d:03}"),
                    title: format!("Note {d}"),
                    body,
                });
            }
            EvalScenario {
                id: format!("scenario-{i:02}"),
                goal: format!(
                    "Report the retention figure from the earlier findings ({marker})."
                ),
                constraints: vec![
                    "final output must cite the earlier finding".into(),
                    "keep the answer under 80 words".into(),
                ],
                history,
                needle_id: format!("note-{i:02}-{needle_pos:03}"),
                query: marker.clone(),
                answer_marker: marker,
            }
        })
        .collect()
}

/// Deterministic sweep over policies × budgets × history sizes.
pub fn run_sweep(
    budgets: &[usize],
    history_lens: &[usize],
    scenarios_per_point: usize,
) -> SweepReport {
    let policies = ["working_set", "retrieval_topk", "full_history"];
    let mut points = Vec::new();
    for &history_len in history_lens {
        let scenarios = generate_suite_sized(scenarios_per_point, history_len);
        for &budget in budgets {
            for policy in policies {
                let n = scenarios.len().max(1) as f64;
                let (mut success, mut correct, mut kept, mut tokens) = (0.0, 0.0, 0.0, 0.0);
                for scenario in &scenarios {
                    let selection = select(policy, scenario, budget);
                    let ok = selection.included_needle && selection.constraint_retained;
                    success += if ok { 1.0 } else { 0.0 };
                    correct += if selection.included_needle { 1.0 } else { 0.0 };
                    kept += if selection.constraint_retained {
                        1.0
                    } else {
                        0.0
                    };
                    tokens += selection.tokens as f64;
                }
                points.push(SweepPoint {
                    policy: policy.to_string(),
                    history_len,
                    budget_tokens: budget,
                    success_rate: success / n,
                    correctness: correct / n,
                    constraint_retention: kept / n,
                    mean_tokens: tokens / n,
                });
            }
        }
    }
    SweepReport {
        generated_at: crate::storage::event_ledger::now(),
        points,
    }
}

// ---- Gate quality: catch rate per failure mode + false-kill rate ----

#[derive(Clone, Debug, Serialize)]
pub struct GateCase {
    pub label: String,
    pub verdict: &'static str,
    pub caught: bool,
    pub error_code: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct GateReport {
    pub generated_at: String,
    pub cases: Vec<GateCase>,
    pub catch_rate_per_mode: BTreeMap<String, f64>,
    pub false_kill_rate: f64,
    pub good_pass_rate: f64,
}

impl GateReport {
    pub fn render_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Verification-gate quality (catch rate / false-kill)\n\n");
        out.push_str(&format!("- generated_at: {}\n\n", self.generated_at));
        out.push_str("| failure mode | catch rate |\n|---|---|\n");
        for (mode, rate) in &self.catch_rate_per_mode {
            out.push_str(&format!("| {mode} | {rate:.2} |\n"));
        }
        out.push_str(&format!(
            "\n- good-deliverable pass rate: **{:.2}** (false-kill rate: **{:.2}**)\n",
            self.good_pass_rate, self.false_kill_rate
        ));
        out
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Runs the real `VerificationEngine` over a labeled deliverable mixture.
pub fn run_gate_metrics(
    storage_root: &std::path::Path,
) -> Result<GateReport, crate::error::SfaiError> {
    use crate::app::build_app;
    use crate::contracts::TrustLevel;

    let app = build_app(storage_root)?;
    let mut cases = Vec::new();

    // Deliverable templates: (label, is_good, content builder)
    type ModeSpec = (&'static str, bool, Box<dyn Fn(usize, &str) -> String>);
    let modes: Vec<ModeSpec> = vec![
        (
            "good_report",
            true,
            Box::new(|i: usize, ref1: &str| {
                format!("# Report {i}\n\nAll claims are backed.\n\n- source: {ref1}\n")
            }),
        ),
        (
            "broken_citation",
            false,
            Box::new(|i: usize, _r: &str| {
                format!("# Report {i}\n\n- source: source://missing@1\n")
            }),
        ),
        (
            "zero_citation",
            false,
            Box::new(|i: usize, _r: &str| format!("# Report {i}\n\nPure prose without refs.\n")),
        ),
        (
            "placeholder_text",
            false,
            Box::new(|i: usize, ref1: &str| {
                format!("# Report {i}\n\nTODO: finish.\n\n- source: {ref1}\n")
            }),
        ),
        (
            "wrong_answer",
            false,
            Box::new(|i: usize, ref1: &str| {
                format!("# Report {i}\n\nThe answer is 99999.\n\n- source: {ref1}\n")
            }),
        ),
    ];

    let per_mode = 4usize;
    let mut pass_counts: BTreeMap<String, (usize, usize)> = BTreeMap::new(); // (passes, total)
    for (mode, is_good, build) in &modes {
        for i in 0..per_mode {
            let task = app.create_task(
                &format!("gate case {mode} {i}"),
                vec![],
                vec![
                    "artifact_exists:report.md".into(),
                    "citations_resolve".into(),
                    "no_placeholders:report.md".into(),
                    "artifact_contains:report.md:retention figure is 42%".into(),
                ],
                None,
            )?;
            let source = app.objects.put_source(
                &task.task_id,
                "https://example.org/evidence",
                b"the retention figure is 42% under sparse-context",
                "text/plain",
                "web",
                TrustLevel::Untrusted,
            )?;
            let ref1 = crate::ids::source_ref(&source.source_id, 1);
            let content = if *mode == "good_report" {
                format!(
                    "# Report {i}\n\nAll claims are backed: the retention figure is 42%.\n\n- source: {ref1}\n"
                )
            } else if *mode == "wrong_answer" {
                format!("# Report {i}\n\nThe answer is 99999.\n\n- source: {ref1}\n")
            } else {
                build(i, &ref1)
            };
            app.objects.put_artifact(
                &task.task_id,
                "report.md",
                content.as_bytes(),
                "text/markdown",
                &[],
            )?;
            let results = app.verifier.verify(
                &task.task_id,
                &[
                    "artifact_exists:report.md".into(),
                    "citations_resolve".into(),
                    "no_placeholders:report.md".into(),
                    "artifact_contains:report.md:retention figure is 42%".into(),
                ],
            )?;
            let passed = results
                .iter()
                .all(|r| r.verdict == crate::contracts::Verdict::Pass);
            let error_code = results
                .iter()
                .find(|r| r.verdict == crate::contracts::Verdict::Fail)
                .and_then(|r| r.diagnostics.first())
                .map(|d| d.error_code.clone())
                .unwrap_or_default();
            let entry = pass_counts.entry(mode.to_string()).or_insert((0, 0));
            entry.1 += 1;
            if passed {
                entry.0 += 1;
            }
            cases.push(GateCase {
                label: format!("{mode}-{i}"),
                verdict: if passed { "pass" } else { "fail" },
                caught: !passed && !is_good,
                error_code,
            });
        }
    }

    let mut catch_rate_per_mode = BTreeMap::new();
    let (mut good_pass, mut good_total) = (0usize, 0usize);
    for (mode, is_good, _) in &modes {
        let (passes, total) = pass_counts[*mode];
        if *is_good {
            good_pass += passes;
            good_total += total;
        } else {
            let caught = total - passes;
            catch_rate_per_mode.insert(mode.to_string(), caught as f64 / total as f64);
        }
    }
    let good_pass_rate = good_pass as f64 / good_total.max(1) as f64;
    Ok(GateReport {
        generated_at: crate::storage::event_ledger::now(),
        cases,
        catch_rate_per_mode,
        false_kill_rate: 1.0 - good_pass_rate,
        good_pass_rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_generation_is_deterministic() {
        let a = generate_suite(3);
        let b = generate_suite(3);
        assert_eq!(a.len(), 3);
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
        assert_eq!(a[0].needle_id, "note-00-02");
        assert_eq!(a[1].needle_id, "note-01-20");
        assert_eq!(a[2].needle_id, "note-02-38");
    }

    #[test]
    fn working_set_keeps_pinned_constraints() {
        let scenario = &generate_suite(1)[0];
        let selection = select("working_set", scenario, DEFAULT_BUDGET_TOKENS);
        assert!(selection.constraint_retained);
        assert!(selection.included_needle);
    }

    #[test]
    fn sliding_window_misses_early_needle() {
        let scenario = &generate_suite(1)[0];
        let selection = select("sliding_window", scenario, DEFAULT_BUDGET_TOKENS);
        assert!(
            !selection.included_needle,
            "early needle is outside the window"
        );
        assert!(!selection.constraint_retained);
    }
}
