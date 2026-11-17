//! HumanEval independent audit (P0, ICML sprint) — #[ignore], real network.
//!
//! Answers: why is HumanEval correctness 0.00 across models? Evidence per run:
//! - what artifacts the agent actually wrote (paths + contents)
//! - per-criterion acceptance results
//! - the OFFICIAL test harness executed manually on the written deliverable
//!
//! Run: BAILIAN_API_KEY=... cargo test --release --test he_audit -- --ignored --nocapture

use std::path::PathBuf;
use std::process::Command;

use sfai::app::build_app;
use sfai::contracts::{IntentKind, Verdict};
use sfai::integrations::providers::openai_compat::OpenAICompatibleAdapter;

fn tasks_db() -> serde_json::Value {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evals/public/tasks.json");
    let raw = std::fs::read_to_string(p).expect("tasks.json");
    serde_json::from_str(&raw).expect("parse tasks.json")
}

#[test]
#[ignore]
fn audit_humaneval_first_two_tasks() {
    let key = std::env::var("BAILIAN_API_KEY").expect("BAILIAN_API_KEY required for audit");
    let base = "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1";
    let model = std::env::var("SFAI_AUDIT_MODEL").unwrap_or_else(|_| "deepseek-v4.1-flash".into());

    let db = tasks_db();
    let tasks: Vec<&serde_json::Value> = db
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["suite"] == "public-humaneval")
        .take(2)
        .collect();

    let dir = tempfile::tempdir().expect("tempdir");
    let app = build_app(dir.path()).expect("app");
    let audit_out = PathBuf::from("/tmp/he-audit");
    std::fs::create_dir_all(&audit_out).ok();

    for spec in tasks {
        let id = spec["id"].as_str().unwrap();
        println!("\n================ {id} ================");
        let goal = spec["goal"].as_str().unwrap();
        let rules: Vec<String> = spec["acceptance_rules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_str().unwrap().to_string())
            .collect();
        let task = app
            .create_task(goal, vec![], rules.clone(), None)
            .expect("create task");
        // seed task files exactly like the bench does
        for (path, content) in spec["files"].as_object().unwrap() {
            app.objects
                .put_artifact(
                    &task.task_id,
                    path,
                    content.as_str().unwrap().as_bytes(),
                    "text/plain",
                    &[],
                )
                .expect("seed file");
        }

        let mut model = OpenAICompatibleAdapter::new(base, &key, &model);
        let result = app.run(&mut model, &task.task_id, 25).expect("run");
        println!(">> status={:?} steps={}", result.status, result.steps);

        // 1) what did the agent actually write? (probe expected paths + file.write events)
        let task_dir = audit_out.join(id);
        std::fs::create_dir_all(&task_dir).ok();
        let mut written: Vec<String> = Vec::new();
        for path in [
            "solution.py",
            "test_solution.py",
            "answer.json",
            "report.md",
        ] {
            if let Some(a) = app
                .objects
                .find_artifact_by_path(&task.task_id, path)
                .expect("find artifact")
            {
                let bytes = app.objects.get_blob(&a.checksum).expect("blob");
                println!("   - {path} v{} ({} bytes)", a.version, bytes.len());
                let preview: String = String::from_utf8_lossy(&bytes).chars().take(300).collect();
                println!("     | {}", preview.replace('\n', "\n     | "));
                std::fs::write(task_dir.join(path), &bytes).ok();
                written.push(path.to_string());
            }
        }
        // any other file.write calls?
        for e in app.ledger.events_for(&task.task_id, 0).unwrap_or_default() {
            if e.event_type == "tool.committed" {
                if let Some(op) = e.payload.get("operation_id").and_then(|o| o.as_str()) {
                    if op == "file.write" || op == "workspace.exec" {
                        println!(
                            "   [event] {op} preview={:?}",
                            e.payload
                                .get("result_preview")
                                .and_then(|p| p.as_str())
                                .unwrap_or("")
                        );
                    }
                }
            }
        }
        println!(">> artifacts written: {written:?}");

        // 2) per-criterion acceptance evidence
        let protected: Vec<String> = spec["protected_files"]
            .as_array()
            .map(|a| a.iter().map(|x| x.as_str().unwrap().to_string()).collect())
            .unwrap_or_default();
        let results = app
            .verifier
            .verify_protected(&task.task_id, &rules, &protected)
            .expect("verify");
        for r in &results {
            println!(
                ">> criterion {} -> {:?} findings={:?}",
                r.criterion, r.verdict, r.findings
            );
        }

        // 3) run the OFFICIAL test harness manually on the written deliverable
        //    (test_solution.py comes from the task spec if the agent didn't write it)
        let sol = task_dir.join("solution.py");
        let test = task_dir.join("test_solution.py");
        if !test.exists() {
            if let Some(t) = spec["files"].get("test_solution.py") {
                std::fs::write(&test, t.as_str().unwrap()).ok();
            }
        }
        if sol.exists() && test.exists() {
            let out = Command::new("python3")
                .arg("test_solution.py")
                .current_dir(&task_dir)
                .output()
                .expect("run python");
            println!(
                ">> official harness: exit={} stdout={} stderr={}",
                out.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&out.stdout)
                    .chars()
                    .take(400)
                    .collect::<String>(),
                String::from_utf8_lossy(&out.stderr)
                    .chars()
                    .take(800)
                    .collect::<String>(),
            );
        } else {
            println!(
                ">> official harness: SKIPPED — deliverable missing (solution={} test={})",
                sol.exists(),
                test.exists()
            );
        }

        let intents: Vec<String> = app
            .ledger
            .events_for(&task.task_id, 0)
            .unwrap_or_default()
            .iter()
            .filter(|e| e.event_type == "task.intent_started")
            .map(|e| {
                e.payload
                    .get("intent_kind")
                    .and_then(|k| k.as_str())
                    .unwrap_or("?")
                    .to_string()
            })
            .collect();
        println!(">> intent sequence: {intents:?}");
        let _ = IntentKind::Finish; // silence unused import when adapted
        let _ = Verdict::Pass;
    }
    panic!["audit run complete — inspect printed evidence (intentional fail to surface output)"];
}
