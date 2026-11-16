//! M5 — Skill packages with progressive disclosure (plan §6 / M5).
//!
//! Discovery loads only `name` + `description` (+ version); the instructions
//! body is read *only* on activation and then lands in the working set for the
//! rest of that run. Activation is metadata-only: it confers no permission.
//! Third-party Skill text is untrusted content — it can never change policy.

use crate::error::{Result, SfaiError};
use crate::ids::make_ref;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const MAX_INSTRUCTIONS_CHARS: usize = 4000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillTrust {
    Builtin,
    Installed,
}

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: String,
    pub instructions: String,
    pub trust: SkillTrust,
    pub path: PathBuf,
}

/// What discovery is allowed to return: metadata only, no instructions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillManifest {
    pub name: String,
    pub description: String,
    pub version: String,
}

/// What activation returns: this run's working-set payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillActivation {
    pub name: String,
    pub version: String,
    pub description: String,
    pub instructions: String,
    pub ref_id: String,
    pub trust: SkillTrust,
}

#[derive(Default)]
pub struct SkillRegistry {
    skills: BTreeMap<String, Skill>,
    pub activations: Mutex<Vec<String>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load every `<dir>/<skill>/SKILL.md` package.
    pub fn load_from_dir(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let mut registry = SkillRegistry::new();
        if !dir.exists() {
            return Ok(registry);
        }
        let entries = std::fs::read_dir(dir)?;
        for entry in entries {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }
            let skill_file = path.join("SKILL.md");
            if !skill_file.is_file() {
                continue;
            }
            let raw = std::fs::read_to_string(&skill_file)?;
            let skill = parse_skill(&raw, &path)?;
            registry.skills.insert(skill.name.clone(), skill);
        }
        Ok(registry)
    }

    pub fn len(&self) -> usize {
        self.skills.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Discovery: name/description/version only.
    pub fn discover(&self, query: &str, limit: usize) -> Vec<SkillManifest> {
        let terms: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|t| t.to_string())
            .collect();
        let mut out = Vec::new();
        for skill in self.skills.values() {
            let haystack = format!("{} {}", skill.name, skill.description).to_lowercase();
            if !terms.is_empty() && !terms.iter().all(|t| haystack.contains(t)) {
                continue;
            }
            out.push(SkillManifest {
                name: skill.name.clone(),
                description: skill.description.clone(),
                version: skill.version.clone(),
            });
        }
        out.truncate(limit);
        out
    }

    /// Activation: loads instructions into the caller's working set.
    pub fn activate(&self, name: &str) -> Result<SkillActivation> {
        let skill = self
            .skills
            .get(name)
            .ok_or_else(|| SfaiError::NotFound(format!("unknown skill: {name}")))?;
        self.activations
            .lock()
            .expect("skills mutex poisoned")
            .push(name.to_string());
        let instructions: String = skill
            .instructions
            .chars()
            .take(MAX_INSTRUCTIONS_CHARS)
            .collect();
        Ok(SkillActivation {
            name: skill.name.clone(),
            version: skill.version.clone(),
            description: skill.description.clone(),
            instructions,
            ref_id: make_ref("skill", &skill.name, 1, None),
            trust: skill.trust,
        })
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }
}

fn parse_skill(raw: &str, dir: &Path) -> Result<Skill> {
    let mut name = String::new();
    let mut description = String::new();
    let mut version = "0.1.0".to_string();
    let mut body = raw;
    if let Some(rest) = raw.strip_prefix("---") {
        if let Some((front, after)) = rest.split_once("\n---") {
            for line in front.lines() {
                if let Some((key, value)) = line.split_once(':') {
                    let value = value.trim().trim_matches('"').to_string();
                    match key.trim() {
                        "name" => name = value,
                        "description" => description = value,
                        "version" => version = value,
                        _ => {}
                    }
                }
            }
            body = after.trim_start_matches('\n');
        }
    }
    if name.is_empty() {
        name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed")
            .to_string();
    }
    if description.is_empty() {
        return Err(SfaiError::Validation(format!(
            "skill {name} is missing a description in SKILL.md frontmatter"
        )));
    }
    Ok(Skill {
        name,
        description,
        version,
        instructions: body.trim().to_string(),
        trust: SkillTrust::Installed,
        path: dir.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_skill(root: &Path, name: &str, version: &str, instructions: &str) {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            format!(
                "---\nname: {name}\ndescription: Skill for {name}\nversion: {version}\n---\n{instructions}"
            ),
        )
        .unwrap();
    }

    #[test]
    fn discovery_exposes_metadata_only() {
        let dir = tempfile::tempdir().unwrap();
        write_skill(dir.path(), "research", "0.2.0", "SECRET INSTRUCTIONS");
        let registry = SkillRegistry::load_from_dir(dir.path()).unwrap();
        let manifests = registry.discover("research", 10);
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].version, "0.2.0");
        // The manifest type has no instruction field at all; serializing it
        // must not leak the body.
        let json = serde_json::to_string(&manifests[0]).unwrap();
        assert!(!json.contains("SECRET INSTRUCTIONS"));
    }

    #[test]
    fn activation_returns_instructions() {
        let dir = tempfile::tempdir().unwrap();
        write_skill(dir.path(), "writing", "0.1.0", "Use short sentences.");
        let registry = SkillRegistry::load_from_dir(dir.path()).unwrap();
        let act = registry.activate("writing").unwrap();
        assert!(act.instructions.contains("Use short sentences."));
        assert_eq!(act.version, "0.1.0");
        assert_eq!(act.ref_id, "skill://writing@1");
    }

    #[test]
    fn unknown_skill_errors() {
        let registry = SkillRegistry::new();
        assert!(registry.activate("nope").is_err());
    }
}
