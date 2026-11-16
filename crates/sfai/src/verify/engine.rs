//! Deterministic verification engine (plan §7 mode 1 + acceptance rules).
//!
//! Supported criteria:
//!
//! - `artifact_exists:<path>`     artifact produced by this task exists
//! - `file_type:<ext>`            artifact path carries the expected extension
//! - `citations_resolve`          every `source://`/`artifact://` ref mentioned in
//!   the produced artifacts resolves in the stores (no fabricated references)
//! - `hash_matches:<checksum>`    artifact checksum matches
//! - `source_count_at_least:<n>`  task has at least n stored sources
//!
//! Verdicts: pass / fail / uncertain. Failures carry machine-readable
//! diagnostics with repair hints; they never silently mark a task complete.

use crate::contracts::{Verdict, VerificationResult};
use crate::error::Result;
use crate::ids::refs_in_text;
use crate::storage::objects::ObjectStore;
use crate::verify::diagnostics::diagnostic;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const CODE_TEST_TIMEOUT: Duration = Duration::from_secs(120);
pub const MAX_OUTPUT_CHARS: usize = 1200;
pub const PLACEHOLDERS: [&str; 5] = ["todo", "tbd", "fixme", "lorem ipsum", "placeholder"];

#[derive(Clone)]
pub struct VerificationEngine {
    pub objects: ObjectStore,
    pub workspace_base: Option<PathBuf>,
}

impl VerificationEngine {
    pub fn new(objects: ObjectStore) -> Self {
        Self {
            objects,
            workspace_base: None,
        }
    }

    /// Enable workspace-scoped checks (`code_tests_pass`) under `base/<task_id>`.
    pub fn with_workspace(objects: ObjectStore, base: impl AsRef<Path>) -> Self {
        Self {
            objects,
            workspace_base: Some(base.as_ref().to_path_buf()),
        }
    }

    pub fn verify(&self, task_id: &str, criteria: &[String]) -> Result<Vec<VerificationResult>> {
        let mut results = Vec::new();
        for criterion in criteria {
            results.push(self.verify_one(task_id, criterion)?);
        }
        Ok(results)
    }

    fn verify_one(&self, task_id: &str, criterion: &str) -> Result<VerificationResult> {
        let (name, arg) = match criterion.split_once(':') {
            Some((n, a)) => (n, a),
            None => (criterion, ""),
        };
        match name {
            "artifact_exists" => {
                if let Some(artifact) = self.objects.find_artifact_by_path(task_id, arg)? {
                    let reference =
                        format!("artifact://{}@{}", artifact.artifact_id, artifact.version);
                    Ok(VerificationResult {
                        target_ref: reference.clone(),
                        criterion: criterion.to_string(),
                        verdict: Verdict::Pass,
                        findings: format!("artifact present: {arg}"),
                        evidence_refs: vec![reference],
                        diagnostics: Vec::new(),
                    })
                } else {
                    Ok(self.fail(
                        &format!("artifact://{task_id}/{arg}@0"),
                        criterion,
                        "ARTIFACT_MISSING",
                        &format!("required artifact not produced: {arg}"),
                        "Produce the artifact before finishing.",
                        Vec::new(),
                    ))
                }
            }
            "file_type" => {
                let artifacts = self.objects.task_artifacts(task_id)?;
                if let Some(a) = artifacts.iter().rev().find(|a| a.path.ends_with(arg)) {
                    Ok(VerificationResult {
                        target_ref: format!("artifact://{}@{}", a.artifact_id, a.version),
                        criterion: criterion.to_string(),
                        verdict: Verdict::Pass,
                        findings: format!("file type {arg} satisfied by {}", a.path),
                        evidence_refs: Vec::new(),
                        diagnostics: Vec::new(),
                    })
                } else {
                    Ok(self.fail(
                        &format!("artifact://{task_id}/?@0"),
                        criterion,
                        "FILE_TYPE_MISMATCH",
                        &format!("no artifact with extension {arg}"),
                        "Produce an artifact with the required file type.",
                        Vec::new(),
                    ))
                }
            }
            "citations_resolve" => self.verify_citations(task_id, criterion),
            "code_tests_pass" => self.verify_code_tests(task_id, criterion, arg),
            "json_valid" => self.verify_json(task_id, criterion, arg),
            "no_placeholders" => self.verify_placeholders(task_id, criterion, arg),
            "artifact_contains" => self.verify_contains(task_id, criterion, arg),
            "hash_matches" => {
                let artifacts = self.objects.task_artifacts(task_id)?;
                if let Some(a) = artifacts.iter().find(|a| a.checksum == arg) {
                    Ok(VerificationResult {
                        target_ref: format!("artifact://{}@{}", a.artifact_id, a.version),
                        criterion: criterion.to_string(),
                        verdict: Verdict::Pass,
                        findings: "checksum matches".into(),
                        evidence_refs: Vec::new(),
                        diagnostics: Vec::new(),
                    })
                } else {
                    Ok(self.fail(
                        &format!("artifact://{task_id}/?@0"),
                        criterion,
                        "HASH_MISMATCH",
                        "no artifact with the expected checksum",
                        "Recompute the artifact and rerun verification.",
                        Vec::new(),
                    ))
                }
            }
            "source_count_at_least" => {
                let refs = self.objects.source_refs_for_task(task_id)?;
                let need: usize = arg.parse().unwrap_or(0);
                if refs.len() >= need {
                    Ok(VerificationResult {
                        target_ref: format!("task://{task_id}/index@1"),
                        criterion: criterion.to_string(),
                        verdict: Verdict::Pass,
                        findings: format!("{} sources stored", refs.len()),
                        evidence_refs: refs,
                        diagnostics: Vec::new(),
                    })
                } else {
                    Ok(self.fail(
                        &format!("task://{task_id}/index@1"),
                        criterion,
                        "NOT_ENOUGH_SOURCES",
                        &format!("stored {} sources, need {need}", refs.len()),
                        "Gather more sources before finishing.",
                        refs,
                    ))
                }
            }
            _ => Ok(VerificationResult {
                target_ref: format!("task://{task_id}/index@1"),
                criterion: criterion.to_string(),
                verdict: Verdict::Uncertain,
                findings: format!("unknown criterion: {criterion}"),
                evidence_refs: Vec::new(),
                diagnostics: vec![diagnostic(
                    Verdict::Uncertain,
                    &format!("task://{task_id}/index@1"),
                    criterion,
                    "UNKNOWN_CRITERION",
                    Vec::new(),
                    "Implement this criterion or fix the rule.",
                    false,
                )],
            }),
        }
    }

    fn verify_code_tests(
        &self,
        task_id: &str,
        criterion: &str,
        command: &str,
    ) -> Result<VerificationResult> {
        let Some(base) = &self.workspace_base else {
            return Ok(VerificationResult {
                target_ref: format!("task://{task_id}/workspace@1"),
                criterion: criterion.to_string(),
                verdict: Verdict::Uncertain,
                findings: "no workspace configured for code checks".into(),
                evidence_refs: Vec::new(),
                diagnostics: vec![diagnostic(
                    Verdict::Uncertain,
                    &format!("task://{task_id}/workspace@1"),
                    criterion,
                    "WORKSPACE_UNAVAILABLE",
                    Vec::new(),
                    "Configure a workspace root before running code checks.",
                    false,
                )],
            });
        };
        let workspace = base.join(task_id);
        std::fs::create_dir_all(&workspace)?;
        let (code, output) = run_sandboxed_command(command, &workspace, CODE_TEST_TIMEOUT)?;
        let target = format!("task://{task_id}/workspace@1");
        let snippet: String = output.chars().take(MAX_OUTPUT_CHARS).collect();
        if code == 0 {
            Ok(VerificationResult {
                target_ref: target,
                criterion: criterion.to_string(),
                verdict: Verdict::Pass,
                findings: format!("command succeeded: {command}"),
                evidence_refs: Vec::new(),
                diagnostics: Vec::new(),
            })
        } else {
            Ok(VerificationResult {
                target_ref: target.clone(),
                criterion: criterion.to_string(),
                verdict: Verdict::Fail,
                findings: format!("command failed (exit {code}): {command}"),
                evidence_refs: Vec::new(),
                diagnostics: vec![diagnostic(
                    Verdict::Fail,
                    &target,
                    criterion,
                    "CODE_TESTS_FAILED",
                    Vec::new(),
                    &format!("Command output: {snippet}"),
                    true,
                )],
            })
        }
    }

    fn verify_json(
        &self,
        task_id: &str,
        criterion: &str,
        path: &str,
    ) -> Result<VerificationResult> {
        let Some(artifact) = self.objects.find_artifact_by_path(task_id, path)? else {
            return Ok(self.fail(
                &format!("artifact://{task_id}/{path}@0"),
                criterion,
                "ARTIFACT_MISSING",
                &format!("no artifact at {path}"),
                "Produce the artifact before validating it.",
                Vec::new(),
            ));
        };
        let target = format!("artifact://{}@{}", artifact.artifact_id, artifact.version);
        let text = self.objects.read_artifact_text(&artifact.artifact_id)?;
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(_) => Ok(VerificationResult {
                target_ref: target,
                criterion: criterion.to_string(),
                verdict: Verdict::Pass,
                findings: format!("{path} is valid JSON"),
                evidence_refs: Vec::new(),
                diagnostics: Vec::new(),
            }),
            Err(exc) => Ok(VerificationResult {
                target_ref: target.clone(),
                criterion: criterion.to_string(),
                verdict: Verdict::Fail,
                findings: format!("{path} is not valid JSON"),
                evidence_refs: Vec::new(),
                diagnostics: vec![diagnostic(
                    Verdict::Fail,
                    &target,
                    criterion,
                    "JSON_INVALID",
                    Vec::new(),
                    &format!("Fix the JSON syntax: {exc}"),
                    true,
                )],
            }),
        }
    }

    fn verify_placeholders(
        &self,
        task_id: &str,
        criterion: &str,
        path: &str,
    ) -> Result<VerificationResult> {
        let Some(artifact) = self.objects.find_artifact_by_path(task_id, path)? else {
            return Ok(self.fail(
                &format!("artifact://{task_id}/{path}@0"),
                criterion,
                "ARTIFACT_MISSING",
                &format!("no artifact at {path}"),
                "Produce the artifact before validating it.",
                Vec::new(),
            ));
        };
        let target = format!("artifact://{}@{}", artifact.artifact_id, artifact.version);
        let text = self.objects.read_artifact_text(&artifact.artifact_id)?;
        let lowered = text.to_lowercase();
        let found: Vec<String> = PLACEHOLDERS
            .iter()
            .filter(|term| lowered.contains(**term))
            .map(|term| term.to_string())
            .collect();
        if found.is_empty() {
            Ok(VerificationResult {
                target_ref: target,
                criterion: criterion.to_string(),
                verdict: Verdict::Pass,
                findings: "no placeholder text found".into(),
                evidence_refs: Vec::new(),
                diagnostics: Vec::new(),
            })
        } else {
            Ok(VerificationResult {
                target_ref: target.clone(),
                criterion: criterion.to_string(),
                verdict: Verdict::Fail,
                findings: format!("placeholder text found: {}", found.join(", ")),
                evidence_refs: found.clone(),
                diagnostics: vec![diagnostic(
                    Verdict::Fail,
                    &target,
                    criterion,
                    "PLACEHOLDER_TEXT_FOUND",
                    found,
                    "Replace placeholder text with real content.",
                    true,
                )],
            })
        }
    }

    fn verify_contains(
        &self,
        task_id: &str,
        criterion: &str,
        arg: &str,
    ) -> Result<VerificationResult> {
        let (path, needle) = match arg.split_once(':') {
            Some((p, n)) => (p, n),
            None => (arg, ""),
        };
        let Some(artifact) = self.objects.find_artifact_by_path(task_id, path)? else {
            return Ok(self.fail(
                &format!("artifact://{task_id}/{path}@0"),
                criterion,
                "ARTIFACT_MISSING",
                &format!("no artifact at {path}"),
                "Produce the artifact before validating it.",
                Vec::new(),
            ));
        };
        let target = format!("artifact://{}@{}", artifact.artifact_id, artifact.version);
        let text = self.objects.read_artifact_text(&artifact.artifact_id)?;
        if text.contains(needle) {
            Ok(VerificationResult {
                target_ref: target,
                criterion: criterion.to_string(),
                verdict: Verdict::Pass,
                findings: format!("{path} contains expected content"),
                evidence_refs: Vec::new(),
                diagnostics: Vec::new(),
            })
        } else {
            Ok(VerificationResult {
                target_ref: target.clone(),
                criterion: criterion.to_string(),
                verdict: Verdict::Fail,
                findings: format!("{path} is missing expected content {needle:?}"),
                evidence_refs: Vec::new(),
                diagnostics: vec![diagnostic(
                    Verdict::Fail,
                    &target,
                    criterion,
                    "GROUND_TRUTH_MISMATCH",
                    Vec::new(),
                    "Produce the expected answer content and rerun verification.",
                    true,
                )],
            })
        }
    }

    fn verify_citations(&self, task_id: &str, criterion: &str) -> Result<VerificationResult> {
        // Only the latest version of each artifact path is a deliverable;
        // older broken drafts must not poison the check after a repair.
        let mut latest_by_path: std::collections::BTreeMap<String, crate::contracts::Artifact> =
            std::collections::BTreeMap::new();
        for artifact in self.objects.task_artifacts(task_id)? {
            latest_by_path
                .entry(artifact.path.clone())
                .and_modify(|existing| {
                    if artifact.version > existing.version {
                        *existing = artifact.clone();
                    }
                })
                .or_insert(artifact);
        }
        let artifacts: Vec<crate::contracts::Artifact> = latest_by_path.into_values().collect();
        if artifacts.is_empty() {
            return Ok(self.fail(
                &format!("task://{task_id}/index@1"),
                criterion,
                "NO_ARTIFACT_TO_CHECK",
                "no artifacts to check citations in",
                "Produce the artifact first.",
                Vec::new(),
            ));
        }
        let mut broken: Vec<String> = Vec::new();
        let mut checked = 0usize;
        for artifact in &artifacts {
            let text = self.objects.read_artifact_text(&artifact.artifact_id)?;
            for reference in refs_in_text(&text) {
                checked += 1;
                if self.objects.resolve_ref_text(&reference)?.is_none() {
                    broken.push(reference);
                }
            }
        }
        let last = artifacts.last().unwrap();
        let target = format!("artifact://{}@{}", last.artifact_id, last.version);
        if !broken.is_empty() {
            let mut diag = diagnostic(
                Verdict::Fail,
                &target,
                criterion,
                "BROKEN_CITATION_REF",
                broken.clone(),
                "Rebuild citation mapping and rerun verification.",
                true,
            );
            diag.evidence_refs = broken.clone();
            return Ok(VerificationResult {
                target_ref: target,
                criterion: criterion.to_string(),
                verdict: Verdict::Fail,
                findings: format!("{} unresolved citation(s)", broken.len()),
                evidence_refs: broken,
                diagnostics: vec![diag],
            });
        }
        Ok(VerificationResult {
            target_ref: target,
            criterion: criterion.to_string(),
            verdict: Verdict::Pass,
            findings: format!("all {checked} citation ref(s) resolve"),
            evidence_refs: Vec::new(),
            diagnostics: Vec::new(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn fail(
        &self,
        target_ref: &str,
        criterion: &str,
        error_code: &str,
        findings: &str,
        repair_hint: &str,
        evidence_refs: Vec<String>,
    ) -> VerificationResult {
        VerificationResult {
            target_ref: target_ref.to_string(),
            criterion: criterion.to_string(),
            verdict: Verdict::Fail,
            findings: findings.to_string(),
            evidence_refs: evidence_refs.clone(),
            diagnostics: vec![diagnostic(
                Verdict::Fail,
                target_ref,
                criterion,
                error_code,
                evidence_refs,
                repair_hint,
                false,
            )],
        }
    }
}

/// Run a shell command inside a workspace directory with a hard timeout.
/// Output is captured and bounded; the child is killed on timeout.
fn run_sandboxed_command(command: &str, cwd: &Path, timeout: Duration) -> Result<(i32, String)> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| crate::error::SfaiError::Storage(format!("cannot spawn command: {e}")))?;
    let start = Instant::now();
    loop {
        match child.try_wait()? {
            Some(status) => {
                let mut output = String::new();
                if let Some(mut stdout) = child.stdout.take() {
                    let _ = stdout.read_to_string(&mut output);
                }
                if let Some(mut stderr) = child.stderr.take() {
                    let mut err = String::new();
                    let _ = stderr.read_to_string(&mut err);
                    output.push_str(&err);
                }
                return Ok((status.code().unwrap_or(-1), output));
            }
            None => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Ok((-1, "timeout".into()));
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
    }
}
