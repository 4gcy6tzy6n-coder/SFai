//! Integration & regression tests for the Claude Code-style CLI UI (`ui.rs` & `bin/sfai.rs`).

mod common;

use common::temp_root;
use std::io::Write;
use std::process::{Command, Stdio};

fn sfai_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sfai")
}

#[test]
fn cli_ui_subcommands_render_claude_code_cards() {
    let dir = temp_root();
    let storage = dir.path().to_str().expect("utf8").to_string();
    let bin = sfai_bin();

    // 1. `sfai --ui --no-color run --fixture tool-use`
    let run_out = Command::new(bin)
        .args([
            "--ui",
            "--no-color",
            "--storage",
            &storage,
            "run",
            "--fixture",
            "tool-use",
        ])
        .output()
        .expect("spawn ui run");
    assert!(
        run_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&run_out.stderr)
    );
    let run_stdout = String::from_utf8_lossy(&run_out.stdout);
    assert!(run_stdout.contains("✻ Task Execution"));
    assert!(run_stdout.contains("◌ Step 1"));
    assert!(run_stdout.contains("● WebSearch("));
    assert!(run_stdout.contains("● Write(report.md"));
    assert!(run_stdout.contains("⎿  ✓"));
    assert!(run_stdout.contains("✻ Response"));
    assert!(run_stdout.contains("✓ completed"));

    // Extract task_id from footer `task=task_... `
    let task_id = run_stdout
        .split("task=")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .expect("task_id in footer")
        .to_string();

    // 2. `sfai --ui --no-color tasks`
    let tasks_out = Command::new(bin)
        .args(["--ui", "--no-color", "--storage", &storage, "tasks"])
        .output()
        .expect("spawn ui tasks");
    assert!(tasks_out.status.success());
    let tasks_stdout = String::from_utf8_lossy(&tasks_out.stdout);
    assert!(tasks_stdout.contains("✻ Recent Tasks"));
    assert!(tasks_stdout.contains(&task_id));

    // 3. `sfai --ui --no-color status <task_id>`
    let status_out = Command::new(bin)
        .args([
            "--ui",
            "--no-color",
            "--storage",
            &storage,
            "status",
            &task_id,
        ])
        .output()
        .expect("spawn ui status");
    assert!(status_out.status.success());
    let status_stdout = String::from_utf8_lossy(&status_out.stdout);
    assert!(status_stdout.contains("✻ Task Authority & State"));
    assert!(status_stdout.contains("✓ completed"));

    // 4. `sfai --ui --no-color inspect <task_id>`
    let inspect_out = Command::new(bin)
        .args([
            "--ui",
            "--no-color",
            "--storage",
            &storage,
            "inspect",
            &task_id,
        ])
        .output()
        .expect("spawn ui inspect");
    assert!(inspect_out.status.success());
    let inspect_stdout = String::from_utf8_lossy(&inspect_out.stdout);
    assert!(inspect_stdout.contains("✻ Append-Only Event Ledger"));
    assert!(inspect_stdout.contains("tool.committed"));

    // 5. `sfai --ui --no-color wiki summary <task_id>`
    let wiki_out = Command::new(bin)
        .args([
            "--ui",
            "--no-color",
            "--storage",
            &storage,
            "wiki",
            "summary",
            &task_id,
        ])
        .output()
        .expect("spawn ui wiki");
    assert!(wiki_out.status.success());
    let wiki_stdout = String::from_utf8_lossy(&wiki_out.stdout);
    assert!(wiki_stdout.contains("✻ Incremental Derived Wiki (M4)"));
    assert!(wiki_stdout.contains("──[supported_by]──▶"));
    assert!(wiki_stdout.contains("[evidence] Paper A"));

    // 6. `sfai --ui --no-color verify <task_id>`
    let verify_out = Command::new(bin)
        .args([
            "--ui",
            "--no-color",
            "--storage",
            &storage,
            "verify",
            &task_id,
        ])
        .output()
        .expect("spawn ui verify");
    assert!(verify_out.status.success());
    let verify_stdout = String::from_utf8_lossy(&verify_out.stdout);
    assert!(verify_stdout.contains("✻ Independent Verify & Debug (M6)"));
    assert!(verify_stdout.contains("✓ PASS  citations_resolve"));

    // 7. `sfai --ui --no-color export-trace <task_id>`
    let trace_out = Command::new(bin)
        .args([
            "--ui",
            "--no-color",
            "--storage",
            &storage,
            "export-trace",
            &task_id,
        ])
        .output()
        .expect("spawn ui export-trace");
    assert!(trace_out.status.success());
    let trace_stdout = String::from_utf8_lossy(&trace_out.stdout);
    assert!(trace_stdout.contains("✻ Execution Trace & Cost Replay"));
    assert!(trace_stdout.contains("✓ REPLAY VERIFIED"));

    // 8. `sfai --no-color doctor`
    let doc_out = Command::new(bin)
        .args(["--no-color", "--storage", &storage, "doctor"])
        .output()
        .expect("spawn doctor");
    assert!(doc_out.status.success());
    let doc_stdout = String::from_utf8_lossy(&doc_out.stdout);
    assert!(doc_stdout.contains("✻ System Doctor"));
    assert!(doc_stdout.contains("SQLite WAL Ledger"));
}

#[test]
fn interactive_repl_supports_approval_prompt_and_completes() {
    let dir = temp_root();
    let storage = dir.path().to_str().expect("utf8").to_string();
    let bin = sfai_bin();

    let mut child = Command::new(bin)
        .args(["--no-color", "--storage", &storage])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn interactive");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin
            .write_all(b"/permissions ask\n/run approval\ny\n/cost\n/quit\n")
            .expect("write commands");
    }

    let out = child.wait_with_output().expect("wait");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("⚠ Permission Required"), "stdout: {stdout}");
    assert!(stdout.contains("✓ Approved by user"), "stdout: {stdout}");
    assert!(
        stdout.contains("Status email sent and verified."),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("status=completed"), "stdout: {stdout}");
    assert!(
        stdout.contains("✻ Session Cost & Usage Meter"),
        "stdout: {stdout}"
    );
}

#[test]
fn interactive_repl_denial_blocks_external_write_with_open_issue() {
    let dir = temp_root();
    let storage = dir.path().to_str().expect("utf8").to_string();
    let bin = sfai_bin();

    let mut child = Command::new(bin)
        .args(["--no-color", "--storage", &storage])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn interactive");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin
            .write_all(b"/permissions ask\n/run approval\nn\n/quit\n")
            .expect("write commands");
    }

    let out = child.wait_with_output().expect("wait");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("⊘ Denied by user"), "stdout: {stdout}");
    assert!(
        stdout.contains("status=waiting_approval"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("May I send this email? The previous attempt was denied."),
        "stdout: {stdout}"
    );
}

#[test]
fn interactive_chat_dialog_box_and_multi_turn_conversation() {
    let dir = temp_root();
    let storage = dir.path().to_str().expect("utf8").to_string();
    let bin = sfai_bin();

    let mut child = Command::new(bin)
        .args(["--no-color", "--storage", &storage])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn interactive chat");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin
            .write_all(
                "你好，介绍一下你自己\n搜索并写一份关于 sparse context 的研究报告\n检索之前的证据来源\n/history\n/quit\n"
                    .as_bytes(),
            )
            .expect("write chat turns");
    }

    let out = child.wait_with_output().expect("wait");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Bottom chat input dialog box & status bar
    assert!(stdout.contains("✻ Chat with sfai AI"), "stdout: {stdout}");
    assert!(stdout.contains("│ ❯ sfai>"), "stdout: {stdout}");
    assert!(stdout.contains("⏵⏵"), "stdout: {stdout}");
    // User & AI conversation bubbles across turns
    assert!(stdout.contains("▸ You · Turn #1"), "stdout: {stdout}");
    assert!(stdout.contains("▸ You · Turn #2"), "stdout: {stdout}");
    assert!(stdout.contains("▸ You · Turn #3"), "stdout: {stdout}");
    assert!(stdout.contains("✻ Response (sfai AI)"), "stdout: {stdout}");
    assert!(
        stdout.contains("Retrieved 3 indexed source(s)"),
        "stdout: {stdout}"
    );
}
