//! Public benchmark migration tests (GSM8K / HumanEval subsets).

use sfai::benchmark::{generate_public, generate_tasks_seeded, run_benchmark_full};

#[test]
fn public_task_database_loads() {
    let gsm = generate_public("public-gsm8k");
    let he = generate_public("public-humaneval");
    assert_eq!(gsm.len(), 20, "GSM8K subset ships 20 tasks");
    assert_eq!(he.len(), 20, "HumanEval subset ships 20 tasks");
    for t in gsm.iter().chain(he.iter()) {
        assert!(!t.goal.is_empty());
        assert!(!t.acceptance_rules.is_empty());
        assert!(!t.ground_truth_kind.is_empty());
    }
    // HumanEval ships its test suite as a workspace file and judges by it.
    assert!(he[0].files.contains_key("test_solution.py"));
    assert_eq!(he[0].ground_truth_kind, "acceptance");
    assert!(he[0].acceptance_rules[0].starts_with("code_tests_pass:"));
}

#[test]
fn public_suites_run_end_to_end_offline() {
    // Plumbing smoke: scripted model cannot solve these, but the suite must
    // load, seed workspace files, run, and report every task without crashing.
    let report = run_benchmark_full(
        &["public-gsm8k".to_string(), "public-humaneval".to_string()],
        &["none".to_string()],
        3,
        1,
        0,
        None,
    );
    assert_eq!(report.metrics.len(), 2);
    for m in &report.metrics {
        assert_eq!(m.tasks, 3, "size cap respected");
    }
    assert_eq!(report.tasks.len(), 6);
}

#[test]
fn acceptance_kind_correctness_equals_acceptance() {
    // Unit-level: for ground_truth_kind=acceptance, correct == acceptance.
    let report = run_benchmark_full(
        &["public-humaneval".to_string()],
        &["none".to_string()],
        2,
        1,
        0,
        None,
    );
    for t in &report.tasks {
        assert_eq!(t.correct, t.acceptance_passed);
    }
}

#[test]
fn size_cap_applies_to_public_suites() {
    let tasks = generate_tasks_seeded("public-gsm8k", 5, 0);
    assert_eq!(tasks.len(), 5);
}
