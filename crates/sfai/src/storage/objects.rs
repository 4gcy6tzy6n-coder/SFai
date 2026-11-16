//! Immutable source store and artifact store (plan §5.1 stratum 2).
//!
//! Blobs are content-addressed on the local filesystem; SQLite keeps metadata.
//! Sources and artifacts are never mutated in place.

use crate::contracts::{Artifact, Source, TrustLevel};
use crate::error::{Result, SfaiError};
use crate::ids::{artifact_ref, content_hash, new_id, parse_ref, source_ref};
use crate::storage::db::{json_param, row_i64, row_str, Database, RowMap};
use crate::storage::event_ledger::now;
use rusqlite::params;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct ObjectStore {
    pub db: Database,
    pub blob_root: PathBuf,
}

impl ObjectStore {
    pub fn new(db: Database, blob_root: impl AsRef<Path>) -> Result<Self> {
        let blob_root = blob_root.as_ref().to_path_buf();
        std::fs::create_dir_all(&blob_root)?;
        Ok(Self { db, blob_root })
    }

    // ---- blobs ----

    pub fn put_blob(&self, data: &[u8]) -> Result<String> {
        let digest = content_hash(data);
        let path = self.blob_root.join(&digest);
        if !path.exists() {
            std::fs::write(&path, data)?;
        }
        Ok(digest)
    }

    pub fn get_blob(&self, digest: &str) -> Result<Vec<u8>> {
        Ok(std::fs::read(self.blob_root.join(digest))?)
    }

    // ---- sources ----

    pub fn put_source(
        &self,
        task_id: &str,
        origin: &str,
        data: &[u8],
        mime_type: &str,
        owner: &str,
        trust_level: TrustLevel,
    ) -> Result<Source> {
        let digest = self.put_blob(data)?;
        let source = Source {
            source_id: new_id("src"),
            task_id: task_id.to_string(),
            origin: origin.to_string(),
            content_hash: digest.clone(),
            mime_type: mime_type.to_string(),
            owner: owner.to_string(),
            trust_level,
            storage_uri: format!("blob://{digest}"),
            revision: 1,
            created_at: now(),
        };
        self.db.execute(
            "INSERT INTO sources(source_id, task_id, origin, content_hash, mime_type,
             owner, trust_level, storage_uri, revision, created_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                source.source_id,
                source.task_id,
                source.origin,
                source.content_hash,
                source.mime_type,
                source.owner,
                serde_json::to_string(&source.trust_level)?,
                source.storage_uri,
                source.revision,
                source.created_at
            ],
        )?;
        Ok(source)
    }

    pub fn get_source(&self, source_id: &str, revision: u32) -> Result<Option<Source>> {
        let row = self.db.query_one(
            "SELECT * FROM sources WHERE source_id = ?1 AND revision = ?2",
            params![source_id, revision],
        )?;
        row.as_ref().map(row_to_source).transpose()
    }

    pub fn read_source_text(&self, source_id: &str, revision: u32) -> Result<String> {
        let source = self.get_source(source_id, revision)?.ok_or_else(|| {
            SfaiError::NotFound(format!("unknown source: {source_id}@{revision}"))
        })?;
        Ok(String::from_utf8_lossy(&self.get_blob(&source.content_hash)?).into_owned())
    }

    pub fn source_refs_for_task(&self, task_id: &str) -> Result<Vec<String>> {
        let rows = self.db.query(
            "SELECT source_id, revision FROM sources WHERE task_id = ?1 ORDER BY created_at",
            [task_id],
        )?;
        Ok(rows
            .iter()
            .map(|r| source_ref(&row_str(r, "source_id"), row_i64(r, "revision") as u32))
            .collect())
    }

    // ---- artifacts ----

    pub fn put_artifact(
        &self,
        task_id: &str,
        path: &str,
        data: &[u8],
        mime_type: &str,
        provenance: &[String],
    ) -> Result<Artifact> {
        let digest = self.put_blob(data)?;
        let artifact = Artifact {
            artifact_id: new_id("art"),
            task_id: task_id.to_string(),
            path: path.to_string(),
            checksum: digest,
            mime_type: mime_type.to_string(),
            version: 1,
            provenance: provenance.to_vec(),
            created_at: now(),
        };
        self.db.execute(
            "INSERT INTO artifacts(artifact_id, task_id, path, checksum, mime_type,
             version, provenance_json, created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                artifact.artifact_id,
                artifact.task_id,
                artifact.path,
                artifact.checksum,
                artifact.mime_type,
                artifact.version,
                json_param(&artifact.provenance),
                artifact.created_at
            ],
        )?;
        Ok(artifact)
    }

    pub fn get_artifact(&self, artifact_id: &str) -> Result<Option<Artifact>> {
        let row = self.db.query_one(
            "SELECT * FROM artifacts WHERE artifact_id = ?1",
            [artifact_id],
        )?;
        row.as_ref().map(row_to_artifact).transpose()
    }

    pub fn find_artifact_by_path(&self, task_id: &str, path: &str) -> Result<Option<Artifact>> {
        let row = self.db.query_one(
            "SELECT * FROM artifacts WHERE task_id = ?1 AND path = ?2
             ORDER BY version DESC LIMIT 1",
            params![task_id, path],
        )?;
        row.as_ref().map(row_to_artifact).transpose()
    }

    pub fn read_artifact_text(&self, artifact_id: &str) -> Result<String> {
        let artifact = self
            .get_artifact(artifact_id)?
            .ok_or_else(|| SfaiError::NotFound(format!("unknown artifact: {artifact_id}")))?;
        Ok(String::from_utf8_lossy(&self.get_blob(&artifact.checksum)?).into_owned())
    }

    pub fn artifact_refs_for_task(&self, task_id: &str) -> Result<Vec<String>> {
        let rows = self.db.query(
            "SELECT artifact_id, version FROM artifacts WHERE task_id = ?1 ORDER BY created_at",
            [task_id],
        )?;
        Ok(rows
            .iter()
            .map(|r| artifact_ref(&row_str(r, "artifact_id"), row_i64(r, "version") as u32))
            .collect())
    }

    pub fn task_artifacts(&self, task_id: &str) -> Result<Vec<Artifact>> {
        let rows = self.db.query(
            "SELECT * FROM artifacts WHERE task_id = ?1 ORDER BY created_at",
            [task_id],
        )?;
        rows.iter().map(row_to_artifact).collect()
    }

    /// Resolve a `source://` or `artifact://` ref to (text, provenance).
    pub fn resolve_ref_text(&self, reference: &str) -> Result<Option<(String, Value)>> {
        let parsed = match parse_ref(reference) {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };
        if parsed.kind == "source" {
            let Some(source) = self.get_source(&parsed.ident, parsed.revision)? else {
                return Ok(None);
            };
            let text = String::from_utf8_lossy(&self.get_blob(&source.content_hash)?).into_owned();
            return Ok(Some((
                text.clone(),
                json!({
                    "ref_id": reference,
                    "origin": source.origin,
                    "content_hash": source.content_hash,
                    "owner": source.owner,
                    "trust_level": source.trust_level,
                    "revision": source.revision,
                    "mime_type": source.mime_type,
                }),
            )));
        }
        if parsed.kind == "artifact" {
            let Some(artifact) = self.get_artifact(&parsed.ident)? else {
                return Ok(None);
            };
            let text = String::from_utf8_lossy(&self.get_blob(&artifact.checksum)?).into_owned();
            return Ok(Some((
                text.clone(),
                json!({
                    "ref_id": reference,
                    "origin": artifact.path,
                    "content_hash": artifact.checksum,
                    "owner": "agent",
                    "trust_level": "trusted",
                    "revision": artifact.version,
                    "mime_type": artifact.mime_type,
                }),
            )));
        }
        Ok(None)
    }
}

fn row_to_source(row: &RowMap) -> Result<Source> {
    Ok(Source {
        source_id: row_str(row, "source_id"),
        task_id: row_str(row, "task_id"),
        origin: row_str(row, "origin"),
        content_hash: row_str(row, "content_hash"),
        mime_type: row_str(row, "mime_type"),
        owner: row_str(row, "owner"),
        trust_level: serde_json::from_str(&format!("\"{}\"", row_str(row, "trust_level")))
            .unwrap_or(TrustLevel::Untrusted),
        storage_uri: row_str(row, "storage_uri"),
        revision: row_i64(row, "revision") as u32,
        created_at: row_str(row, "created_at"),
    })
}

fn row_to_artifact(row: &RowMap) -> Result<Artifact> {
    let provenance: Vec<String> =
        serde_json::from_str(&row_str(row, "provenance_json")).unwrap_or_default();
    Ok(Artifact {
        artifact_id: row_str(row, "artifact_id"),
        task_id: row_str(row, "task_id"),
        path: row_str(row, "path"),
        checksum: row_str(row, "checksum"),
        mime_type: row_str(row, "mime_type"),
        version: row_i64(row, "version") as u32,
        provenance,
        created_at: row_str(row, "created_at"),
    })
}
