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

#[derive(Clone)]
pub struct VerificationEngine {
    pub objects: ObjectStore,
}

impl VerificationEngine {
    pub fn new(objects: ObjectStore) -> Self {
        Self { objects }
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

    fn verify_citations(&self, task_id: &str, criterion: &str) -> Result<VerificationResult> {
        let artifacts = self.objects.task_artifacts(task_id)?;
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
