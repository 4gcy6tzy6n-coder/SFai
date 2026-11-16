//! Shared helpers for integration tests.

#![allow(dead_code)]

use sfai::app::App;
use sfai::contracts::Task;
use sfai::core::decision_kernel::TaskRunResult;
use sfai::fixtures::{get_fixture, FixtureSpec};

pub fn temp_root() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

pub fn create_and_setup(app: &App, name: &str) -> (FixtureSpec, Task) {
    let spec = get_fixture(name).expect("fixture exists");
    let task = app
        .create_task(
            &spec.goal,
            spec.constraints.clone(),
            spec.acceptance_rules.clone(),
            Some(spec.budget.clone()),
        )
        .expect("create task");
    if let Some(setup) = spec.setup {
        setup(app, &task).expect("fixture setup");
    }
    (spec, task)
}

pub fn run_fixture(app: &App, name: &str) -> (Task, TaskRunResult) {
    let (spec, task) = create_and_setup(app, name);
    let mut model = (spec.build_model)();
    let result = app.run(model.as_mut(), &task.task_id, 30).expect("run");
    (task, result)
}

pub fn open_errors(app: &App, task_id: &str) -> Vec<serde_json::Value> {
    app.task_store
        .state(task_id)
        .expect("state")
        .unresolved_errors
}

pub fn tool_run_rows(app: &App, task_id: &str, operation_id: &str) -> Vec<serde_json::Value> {
    let db = &app.db;
    db.query(
        "SELECT result_json FROM tool_runs WHERE task_id = ?1 AND operation_id = ?2",
        [task_id, operation_id],
    )
    .expect("query tool_runs")
    .into_iter()
    .map(|r| {
        let result_json = r
            .get("result_json")
            .and_then(|v| v.as_str())
            .unwrap_or("{}");
        serde_json::from_str::<serde_json::Value>(result_json).unwrap_or_default()
    })
    .collect()
}
