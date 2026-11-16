//! Stable identifiers and addressable references.
//!
//! Ref formats (plan §3.5 / §5.4):
//!
//! - `task://<task_id>/index@<rev>`      task root index entrypoint
//! - `source://<source_id>@<rev>`        immutable original source
//! - `artifact://<artifact_id>@<rev>`    produced artifact
//! - `skill://<skill_id>@<rev>`          skill package
//! - `wiki://<task_id>/<node>@<rev>`     derived wiki node (M4)
//!
//! Every ref carries a revision so a context reference can never silently
//! resolve to a different version than the one the model saw.

use sha2::{Digest, Sha256};

pub const REF_KINDS: &[&str] = &["task", "source", "artifact", "skill", "wiki", "event"];

pub fn new_id(prefix: &str) -> String {
    format!(
        "{}_{}",
        prefix,
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    )
}

pub fn new_task_id() -> String {
    new_id("task")
}

pub fn new_decision_id() -> String {
    new_id("dec")
}

pub fn new_tool_run_id() -> String {
    new_id("run")
}

pub fn content_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Canonical JSON hash: `serde_json::Value` maps are BTreeMaps, so key order
/// is already sorted and the encoding is deterministic.
pub fn canonical_hash(value: &serde_json::Value) -> String {
    let payload = serde_json::to_string(value).unwrap_or_default();
    content_hash(payload.as_bytes())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRef {
    pub kind: String,
    pub ident: String,
    pub revision: u32,
    pub locator: Option<String>,
}

pub fn make_ref(kind: &str, ident: &str, revision: u32, locator: Option<&str>) -> String {
    assert!(REF_KINDS.contains(&kind), "unknown ref kind: {kind}");
    let base = match kind {
        "task" => format!("task://{ident}/index@{revision}"),
        "event" => format!("event://{ident}@{revision}"),
        _ => format!("{kind}://{ident}@{revision}"),
    };
    match locator {
        Some(loc) => format!("{base}#{loc}"),
        None => base,
    }
}

pub fn parse_ref(reference: &str) -> Result<ParsedRef, String> {
    let (body, locator) = match reference.split_once('#') {
        Some((b, loc)) => (b, Some(loc.to_string())),
        None => (reference, None),
    };
    let (kind, rest) = body
        .split_once("://")
        .ok_or_else(|| format!("malformed ref: {reference}"))?;
    if !REF_KINDS.contains(&kind) {
        return Err(format!("unknown ref kind in: {reference}"));
    }
    let (ident_part, rev_str) = rest
        .rsplit_once('@')
        .ok_or_else(|| format!("ref missing revision: {reference}"))?;
    let ident = if kind == "task" {
        ident_part
            .strip_suffix("/index")
            .ok_or_else(|| format!("task ref must end with /index: {reference}"))?
            .to_string()
    } else {
        ident_part.to_string()
    };
    let revision: u32 = rev_str
        .parse()
        .map_err(|_| format!("ref revision must be an integer: {reference}"))?;
    if revision < 1 {
        return Err(format!("ref revision must be >= 1: {reference}"));
    }
    Ok(ParsedRef {
        kind: kind.to_string(),
        ident,
        revision,
        locator,
    })
}

pub fn source_ref(source_id: &str, revision: u32) -> String {
    make_ref("source", source_id, revision, None)
}

pub fn artifact_ref(artifact_id: &str, revision: u32) -> String {
    make_ref("artifact", artifact_id, revision, None)
}

pub fn task_index_ref(task_id: &str, revision: u32) -> String {
    make_ref("task", task_id, revision, None)
}

/// Extract every `source://` / `artifact://` ref mentioned in free text.
/// Used for citation verification and artifact provenance.
pub fn refs_in_text(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for prefix in ["source://", "artifact://"] {
        let mut rest = text;
        while let Some(pos) = rest.find(prefix) {
            let after = &rest[pos + prefix.len()..];
            let end = after
                .char_indices()
                .find(|(_, c)| {
                    !(c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-' | '@'))
                })
                .map(|(i, _)| i)
                .unwrap_or(after.len());
            let mut candidate = format!("{prefix}{}", &after[..end]);
            while candidate.ends_with('.') {
                candidate.pop();
            }
            let valid = candidate
                .rsplit_once('@')
                .map(|(_, rev)| !rev.is_empty() && rev.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false);
            if valid && !out.contains(&candidate) {
                out.push(candidate);
            }
            rest = &rest[pos + prefix.len()..];
        }
    }
    out
}

#[cfg(test)]
mod text_ref_tests {
    use super::*;

    #[test]
    fn extracts_refs_from_markdown() {
        let text =
            "See source://src_abc@1. Also (artifact://art_x@2) and source://src_abc@1 again.";
        let refs = refs_in_text(text);
        assert_eq!(refs, vec!["source://src_abc@1", "artifact://art_x@2"]);
    }

    #[test]
    fn ignores_malformed_refs() {
        assert!(refs_in_text("source://norev and source://x@abc are bad").is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ref_roundtrip() {
        let r = source_ref("src_abc", 3);
        let parsed = parse_ref(&r).unwrap();
        assert_eq!(parsed.kind, "source");
        assert_eq!(parsed.ident, "src_abc");
        assert_eq!(parsed.revision, 3);
        assert_eq!(parsed.locator, None);
    }

    #[test]
    fn task_ref_shape() {
        let r = task_index_ref("task_1", 1);
        assert_eq!(r, "task://task_1/index@1");
        let parsed = parse_ref(&r).unwrap();
        assert_eq!(parsed.ident, "task_1");
    }

    #[test]
    fn malformed_refs_rejected() {
        assert!(parse_ref("source://x").is_err());
        assert!(parse_ref("source://x@0").is_err());
        assert!(parse_ref("nope://x@1").is_err());
        assert!(parse_ref("task://x/other@1").is_err());
    }
}
