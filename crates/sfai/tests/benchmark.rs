//! Benchmark tests: dataset correctness, false-success under ablated
//! verification, determinism, report rendering.

use sfai::benchmark::{
    generate_tasks_seeded, run_benchmark, run_benchmark_full, run_from_config, BenchConfig,
    BenchReport, BenchSuiteMetrics,
};

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
                obj.remove("std_latency_ms");
                obj.remove("ci95_latency_ms");
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
    assert_eq!(report.version, "0.2.0");
}

#[test]
fn long_horizon_recovers_needle_end_to_end() {
    let report = run_benchmark(&["long-horizon".to_string()], &["none".to_string()], 2);
    let m = metric(&report, "long-horizon", "none");
    assert_eq!(m.success_rate, 1.0);
    assert_eq!(m.correctness, 1.0);
}

#[test]
fn repetitions_expand_runs_and_metrics() {
    let report = run_benchmark_full(
        &["dataset".to_string()],
        &["none".to_string()],
        2,
        2,
        7,
        None,
    );
    assert_eq!(report.repetitions, 2);
    assert_eq!(report.seed, 7);
    let m = metric(&report, "dataset", "none");
    assert_eq!(m.tasks, 4, "2 tasks × 2 repetitions");
    assert_eq!(m.repetitions, 2);
    let seeds: Vec<u64> = report.tasks.iter().map(|t| t.seed).collect();
    assert!(seeds.contains(&7) && seeds.contains(&8));
}

#[test]
fn confidence_intervals_are_sane() {
    let report = run_benchmark_full(
        &["dataset".to_string()],
        &["none".to_string()],
        3,
        2,
        0,
        None,
    );
    let m = metric(&report, "dataset", "none");
    assert!(
        m.success_ci95[0] <= m.success_rate + 1e-9 && m.success_rate <= m.success_ci95[1] + 1e-9,
        "success ci={:?} rate={}",
        m.success_ci95,
        m.success_rate
    );
    assert!(
        m.correctness_ci95[0] <= m.correctness + 1e-9
            && m.correctness <= m.correctness_ci95[1] + 1e-9,
        "correctness ci={:?} rate={}",
        m.correctness_ci95,
        m.correctness
    );
    assert!(m.ci95_latency_ms >= 0.0);
    assert!(m.ci95_cost_usd >= 0.0);
}

#[test]
fn seeds_vary_task_instances() {
    let a = generate_tasks_seeded("dataset", 1, 0);
    let b = generate_tasks_seeded("dataset", 1, 1);
    assert_ne!(a[0].ground_truth, b[0].ground_truth);
}

#[test]
fn config_parses_and_runs() {
    let cfg: BenchConfig = serde_json::from_str(
        r#"{"name":"e1-baseline","suites":["dataset"],"ablations":["none"],"size":1,"repetitions":1,"seed":3}"#,
    )
    .unwrap();
    let report = run_from_config(&cfg, None);
    assert_eq!(report.config.as_ref().unwrap().name, "e1-baseline");
    assert_eq!(report.seed, 3);
    assert_eq!(report.repetitions, 1);
}
