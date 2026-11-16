//! Working-set selection under a token budget (plan §5.6).
//!
//! Deterministic heuristic baseline — the benchmark a learned indexer must
//! beat. Pinned constraints are excluded from eviction: they always survive.
//! Returns a retrieval receipt so selection is auditable.

use crate::context::retrieval::SearchCandidate;
use crate::contracts::RetrievalReceipt;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct ScoredCandidate {
    pub candidate: SearchCandidate,
    pub score: f64,
    pub components: BTreeMap<String, f64>,
    pub reason_codes: Vec<String>,
}

fn token_estimate(text: &str) -> usize {
    (text.chars().count() / 4).max(1)
}

/// score = lexical + task_state_match + recency - redundancy - staleness
pub fn score_candidates(
    candidates: &[SearchCandidate],
    task_goal: &str,
    active_refs: Option<&BTreeSet<String>>,
) -> Vec<ScoredCandidate> {
    let goal_terms: BTreeSet<String> = task_goal
        .split_whitespace()
        .filter(|t| t.chars().count() > 2)
        .map(|t| t.to_lowercase())
        .collect();
    let mut seen_signatures: BTreeSet<String> = BTreeSet::new();
    let mut scored: Vec<ScoredCandidate> = Vec::new();

    for cand in candidates {
        let mut components: BTreeMap<String, f64> = BTreeMap::new();
        let mut reasons: Vec<String> = Vec::new();

        components.insert("lexical_match".into(), cand.lexical_score.min(10.0));
        if cand.lexical_score > 0.0 {
            reasons.push("lexical_hit".into());
        }

        let title_terms: BTreeSet<String> = cand
            .title
            .replace(':', " ")
            .split_whitespace()
            .filter(|t| t.chars().count() > 2)
            .map(|t| t.to_lowercase())
            .collect();
        let overlap = goal_terms.intersection(&title_terms).count();
        components.insert("task_state_match".into(), overlap as f64);
        if overlap > 0 {
            reasons.push("task_match".into());
        }

        components.insert("recency".into(), 1.0);
        reasons.push("recent".into());

        let signature = format!("{}:{}", cand.kind, cand.title);
        let redundant = !seen_signatures.insert(signature);
        components.insert(
            "redundancy_penalty".into(),
            if redundant { -2.0 } else { 0.0 },
        );
        if redundant {
            reasons.push("redundant".into());
        }

        components.insert("staleness_penalty".into(), 0.0);
        if let Some(active) = active_refs {
            if active.contains(&cand.ref_id) {
                components.insert("active_ref_bonus".into(), 1.0);
                reasons.push("already_active".into());
            }
        }

        let score: f64 = components.values().sum();
        scored.push(ScoredCandidate {
            candidate: cand.clone(),
            score,
            components,
            reason_codes: reasons,
        });
    }

    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.candidate.ref_id.cmp(&b.candidate.ref_id))
    });
    scored
}

pub fn compile_working_set(
    candidates: &[SearchCandidate],
    task_goal: &str,
    budget_tokens: usize,
    pinned_texts: &[String],
    active_refs: Option<&BTreeSet<String>>,
) -> (Vec<SearchCandidate>, RetrievalReceipt) {
    let pinned_tokens: usize = pinned_texts.iter().map(|t| token_estimate(t)).sum();
    let remaining = budget_tokens.saturating_sub(pinned_tokens);

    let scored = score_candidates(candidates, task_goal, active_refs);
    let mut selected: Vec<SearchCandidate> = Vec::new();
    let mut revisions: BTreeMap<String, u32> = BTreeMap::new();
    let mut used = 0usize;
    let mut reason_codes: BTreeSet<String> = BTreeSet::new();
    if !pinned_texts.is_empty() {
        reason_codes.insert("pinned_constraints_kept".into());
    }

    for item in &scored {
        let cost = token_estimate(&item.candidate.snippet);
        if used + cost > remaining {
            continue;
        }
        selected.push(item.candidate.clone());
        revisions.insert(item.candidate.ref_id.clone(), item.candidate.revision);
        used += cost;
        for r in &item.reason_codes {
            reason_codes.insert(r.clone());
        }
    }

    let receipt = RetrievalReceipt {
        selected_refs: selected.iter().map(|c| c.ref_id.clone()).collect(),
        omitted_candidates: candidates.len() - selected.len(),
        revisions,
        estimated_tokens: used + pinned_tokens,
        reason_codes: reason_codes.into_iter().collect(),
    };
    (selected, receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(ref_id: &str, title: &str, snippet: &str, score: f64) -> SearchCandidate {
        SearchCandidate {
            ref_id: ref_id.into(),
            kind: "source".into(),
            title: title.into(),
            snippet: snippet.into(),
            lexical_score: score,
            revision: 1,
            token_estimate: 10,
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn budget_and_pinned_tokens_respected() {
        let candidates = vec![
            cand("source://a@1", "Alpha", "alpha claim", 5.0),
            cand("source://b@1", "Beta", "beta claim", 4.0),
        ];
        let pinned = vec!["Final output must be PDF".to_string()];
        let (selected, receipt) =
            compile_working_set(&candidates, "find alpha evidence", 20, &pinned, None);
        assert!(selected.len() <= 2);
        assert!(receipt.estimated_tokens <= 20);
        assert!(receipt
            .reason_codes
            .contains(&"pinned_constraints_kept".to_string()));
    }

    #[test]
    fn redundancy_penalty_dedupes_same_title() {
        let candidates = vec![
            cand("source://a@1", "Same", "first", 5.0),
            cand("source://b@1", "Same", "second", 5.0),
        ];
        let scored = score_candidates(&candidates, "", None);
        let second = scored
            .iter()
            .find(|s| s.candidate.ref_id == "source://b@1")
            .unwrap();
        assert!(second.components["redundancy_penalty"] < 0.0);
    }
}
