//! Verification diagnostics (plan §7 failure contract).

use crate::contracts::{Verdict, VerificationDiagnostic};

#[allow(clippy::too_many_arguments)]
pub fn diagnostic(
    verdict: Verdict,
    target_ref: &str,
    criterion: &str,
    error_code: &str,
    evidence_refs: Vec<String>,
    repair_hint: &str,
    repair_safe_to_auto_run: bool,
) -> VerificationDiagnostic {
    VerificationDiagnostic {
        verdict,
        target_ref: target_ref.to_string(),
        criterion: criterion.to_string(),
        error_code: error_code.to_string(),
        evidence_refs,
        repair_hint: repair_hint.to_string(),
        repair_safe_to_auto_run,
    }
}
