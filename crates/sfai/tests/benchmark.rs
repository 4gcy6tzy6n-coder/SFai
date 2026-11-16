//! Benchmark tests: dataset correctness, false-success under ablated
//! verification, determinism, report rendering.

use sfai::benchmark::{run_benchmark, BenchReport, BenchSuiteMetrics};

fn metric<'a>(report: &'a BenchReport, suite: &str, ablation: &str) -> &'a BenchSuiteMetrics {
    report
        .metrics
        .iter()
        .find(|m| m.suite == suite && m.ablation == ablation)
        .unwrap_or_else(|| panic!("missing metrics for {suite}/{ablation}"))
}

fn normalize(report: &BenchReport) -> Vec<String> {
    report
        .metrics
        .iter()
        .map(|m| {
            let mut v = serde_json::to_value(m).unwrap();
            if let Some(obj) = v.as_object_mut() {
                obj.remove("mean_latency_ms");
                obj.remove("p50_latency_ms");
                obj.remove("p95_latency_ms");
            }
            v.to_string()
        })
        .collect()
}

#[test]
fn dataset_tasks_are_completed_and_correct() {
    let report = run_benchmark(&["dataset".to_string()], &["none".to_string()], 3);
    let m = metric(&report, "dataset", "none");
    assert_eq!(m.tasks, 3);
    assert_eq!(m.success_rate, 1.0, "dataset tasks must complete");
    assert_eq!(m.correctness, 1.0, "answers must match ground truth");
    assert_eq!(m.false_success_rate, 0.0);
    assert!(m.mean_input_tokens > 0.0);
    assert!(m.mean_cost_usd > 0.0);
}

#[test]
fn verification_gate_detects_broken_deliverable() {
    // With enforcement: broken citations block completion.
    let enforced = run_benchmark(&["edge-case".to_string()], &["none".to_string()], 2);
    let m = metric(&enforced, "edge-case", "none");
    assert_eq!(m.success_rate, 0.0, "broken citation must not complete");
    assert_eq!(m.false_success_rate, 0.0, "blocked is not a false success");

    // Ablated verification: the task "completes" but the deliverable is bad.
    let ablated = run_benchmark(
        &["edge-case".to_string()],
        &["no_verification".to_string()],
        2,
    );
    let m2 = metric(&ablated, "edge-case", "no_verification");
    assert_eq!(m2.success_rate, 1.0);
    assert_eq!(
        m2.false_success_rate, 1.0,
        "without verification the framework would report false success"
    );
}

#[test]
fn benchmark_report_is_deterministic_apart_from_latency() {
    let a = run_benchmark(&["dataset".to_string()], &["none".to_string()], 2);
    let b = run_benchmark(&["dataset".to_string()], &["none".to_string()], 2);
    assert_eq!(normalize(&a), normalize(&b));
}

#[test]
fn report_renders_all_suites_and_ablations() {
    let report = run_benchmark(
        &["dataset".to_string()],
        &["none".to_string(), "no_wiki".to_string()],
        2,
    );
    assert_eq!(report.metrics.len(), 2);
    let markdown = report.render_markdown();
    assert!(markdown.contains("| suite |"));
    assert!(markdown.contains("dataset"));
    assert!(markdown.contains("no_wiki"));
    assert!(markdown.contains("false-success"));
    assert!(report.to_json().contains("\"metrics\""));
    assert_eq!(report.version, "0.1.0");
}

#[test]
fn long_horizon_recovers_needle_end_to_end() {
    let report = run_benchmark(&["long-horizon".to_string()], &["none".to_string()], 2);
    let m = metric(&report, "long-horizon", "none");
    assert_eq!(m.success_rate, 1.0);
    assert_eq!(m.correctness, 1.0);
}
