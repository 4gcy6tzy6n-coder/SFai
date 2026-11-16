//! M8 baseline harness tests: determinism, policy invariants, report rendering.

use sfai::eval::{run_suite, DEFAULT_BUDGET_TOKENS};

fn metric<'a>(report: &'a sfai::eval::EvalReport, policy: &str) -> &'a sfai::eval::PolicyMetrics {
    report
        .metrics
        .iter()
        .find(|m| m.policy == policy)
        .expect("policy metric")
}

#[test]
fn suite_metrics_are_deterministic() {
    let a = run_suite("sparse-context", 6, DEFAULT_BUDGET_TOKENS);
    let b = run_suite("sparse-context", 6, DEFAULT_BUDGET_TOKENS);
    // Wall-clock latency is the only non-deterministic metric.
    let normalize = |report: &sfai::eval::EvalReport| -> Vec<String> {
        report
            .metrics
            .iter()
            .map(|m| {
                let mut value = serde_json::to_value(m).unwrap();
                if let Some(obj) = value.as_object_mut() {
                    obj.remove("mean_latency_ms");
                }
                value.to_string()
            })
            .collect()
    };
    assert_eq!(
        normalize(&a),
        normalize(&b),
        "same inputs must produce identical metrics"
    );
    assert_eq!(a.scenario_count, 6);
    assert_eq!(a.metrics.len(), 5);
}

#[test]
fn working_set_matches_full_history_success_at_lower_token_cost() {
    let report = run_suite("sparse-context", 12, DEFAULT_BUDGET_TOKENS);
    let full = metric(&report, "full_history");
    let ws = metric(&report, "working_set");

    assert_eq!(full.success_rate, 1.0, "full history keeps everything");
    assert_eq!(
        ws.success_rate, 1.0,
        "working set keeps needle + pinned constraints"
    );
    assert_eq!(
        ws.constraint_retention, 1.0,
        "pinned constraints never evicted"
    );
    assert!(
        ws.mean_tokens * 2.0 < full.mean_tokens,
        "working set should be far cheaper: ws={} full={}",
        ws.mean_tokens,
        full.mean_tokens
    );
}

#[test]
fn unpinned_policies_lose_constraints_or_needle() {
    let report = run_suite("sparse-context", 12, DEFAULT_BUDGET_TOKENS);
    let sliding = metric(&report, "sliding_window");
    let summary = metric(&report, "summary");
    let retrieval = metric(&report, "retrieval_topk");

    assert!(
        sliding.context_recall < 1.0,
        "an early needle must fall outside a fixed window"
    );
    assert!(
        summary.constraint_retention < 1.0,
        "compaction drops early constraint statements"
    );
    assert!(
        retrieval.constraint_retention < 1.0,
        "lexical top-k drops early constraint statements"
    );
    assert_eq!(summary.success_rate, 0.0);
    assert_eq!(retrieval.success_rate, 0.0);
}

#[test]
fn report_renders_markdown_table() {
    let report = run_suite("sparse-context", 3, DEFAULT_BUDGET_TOKENS);
    let markdown = report.render_markdown();
    for policy in sfai::eval::POLICY_NAMES {
        assert!(markdown.contains(policy), "missing {policy} in report");
    }
    assert!(markdown.contains("| policy |"));
    assert!(markdown.contains("constraint retention"));
    assert!(report.to_json().contains("\"metrics\""));
}

#[test]
fn cost_per_success_is_consistent() {
    let report = run_suite("sparse-context", 6, DEFAULT_BUDGET_TOKENS);
    for m in &report.metrics {
        if m.success_rate > 0.0 {
            let expected = m.mean_cost_usd / m.success_rate;
            assert!(
                (m.cost_per_success_usd - expected).abs() < 1e-12,
                "cost per success mismatch for {}",
                m.policy
            );
        }
    }
}
