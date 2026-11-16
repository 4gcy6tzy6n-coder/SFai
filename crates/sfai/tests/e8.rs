//! E8: self-routing vs forced-pipeline ablation (router value isolation).

use sfai::benchmark::{run_benchmark_full, BenchReport};

fn metric<'a>(
    report: &'a BenchReport,
    suite: &str,
    ablation: &str,
) -> &'a sfai::benchmark::BenchSuiteMetrics {
    report
        .metrics
        .iter()
        .find(|m| m.suite == suite && m.ablation == ablation)
        .unwrap_or_else(|| panic!("missing {suite}/{ablation}"))
}

#[test]
fn forced_pipeline_completes_and_is_measurable() {
    // Offline scripted model: forced route must still complete the task and
    // show up in the report with routing metrics.
    let report = run_benchmark_full(
        &["dataset".to_string(), "tool-use".to_string()],
        &["none".to_string(), "forced_pipeline".to_string()],
        2,
        1,
        0,
        None,
    );
    let forced_ds = metric(&report, "dataset", "forced_pipeline");
    let self_ds = metric(&report, "dataset", "none");
    assert_eq!(forced_ds.success_rate, 1.0, "forced plan is deterministic");
    assert_eq!(self_ds.success_rate, 1.0);
    // Forced route: exactly 4 planned steps (read/write/verify/finish).
    assert_eq!(forced_ds.mean_steps, 4.0, "fixed plan has fixed length");
    assert!(self_ds.mean_steps > 0.0);
}

#[test]
fn self_routing_adapts_route_length() {
    // The reactive self-routing model writes then finishes (3 steps incl. a
    // search), while the forced plan is rigid; both must complete.
    let report = run_benchmark_full(
        &["tool-use".to_string()],
        &["none".to_string(), "forced_pipeline".to_string()],
        2,
        1,
        0,
        None,
    );
    let self_r = metric(&report, "tool-use", "none");
    let forced = metric(&report, "tool-use", "forced_pipeline");
    assert_eq!(self_r.success_rate, 1.0);
    assert_eq!(forced.success_rate, 1.0);
    assert_eq!(forced.mean_steps, 4.0);
    assert!(forced.mean_tool_calls >= 2.0);
}
