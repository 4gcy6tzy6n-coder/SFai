//! Bounded context envelope (plan §3.5) — the only thing the model sees.
//!
//! Pinned goal/constraints/status always survive; everything else arrives as
//! addresses (root index + selected refs) plus bounded previews of recent
//! tool results, with a retrieval receipt.

use crate::capabilities::skills::{SkillActivation, MAX_INSTRUCTIONS_CHARS};
use crate::context::index::ContextIndex;
use crate::context::memory::{MemoryStore, MEMORY_ENVELOPE_LIMIT};
use crate::context::retrieval::ContextRetriever;
use crate::context::working_set::compile_working_set;
use crate::contracts::{ActiveSkill, ContextEnvelope, Task, TaskState};
use crate::error::Result;
use crate::storage::event_ledger::EventLedger;
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub const ACTIVE_CONTEXT_TOKENS: usize = 10_000;
pub const RECENT_RESULTS_WINDOW: usize = 5;

pub struct EnvelopeBuilder {
    pub index: ContextIndex,
    pub retriever: ContextRetriever,
    pub ledger: Option<EventLedger>,
    pub memory: Option<MemoryStore>,
    pub active_context_tokens: usize,
    /// How many retrieval candidates the envelope considers (full-context
    /// ablation raises this).
    pub candidate_limit: usize,
    /// Ablation switch: when false, constraints are not pinned into the envelope.
    pub pin_constraints: bool,
}

impl EnvelopeBuilder {
    pub fn new(
        index: ContextIndex,
        retriever: ContextRetriever,
        ledger: Option<EventLedger>,
    ) -> Self {
        Self {
            index,
            retriever,
            ledger,
            memory: None,
            active_context_tokens: ACTIVE_CONTEXT_TOKENS,
            candidate_limit: 12,
            pin_constraints: true,
        }
    }

    pub fn build(
        &self,
        task: &Task,
        state: &TaskState,
        skills: &[SkillActivation],
    ) -> Result<ContextEnvelope> {
        // OR-joined goal terms: FTS5 barewords AND by default, which would
        // zero out recall for long natural-language goals.
        let terms: Vec<&str> = task
            .goal
            .split_whitespace()
            .filter(|t| t.chars().count() > 2)
            .take(12)
            .collect();
        let query = if terms.is_empty() {
            task.goal.clone()
        } else {
            // Quote every term: barewords with '-' or ':' would otherwise be
            // parsed as FTS5 operators (e.g. "sparse-context" becomes a
            // subtraction and raises "no such column").
            terms
                .iter()
                .map(|t| format!("\"{}\"", t.replace('"', "")))
                .collect::<Vec<_>>()
                .join(" OR ")
        };
        let candidates =
            self.retriever
                .search(&query, Some(&task.task_id), None, self.candidate_limit)?;
        let active_refs: BTreeSet<String> = state.completed_outputs.iter().cloned().collect();
        let pinned_texts: Vec<String> = if self.pin_constraints {
            task.constraints.clone()
        } else {
            Vec::new()
        };
        let (selected, receipt) = compile_working_set(
            &candidates,
            &task.goal,
            self.active_context_tokens,
            &pinned_texts,
            Some(&active_refs),
        );
        let selected_refs: Vec<String> = selected.iter().map(|c| c.ref_id.clone()).collect();
        Ok(ContextEnvelope {
            goal: task.goal.clone(),
            pinned_constraints: pinned_texts,
            task_state: Self::state_summary(state),
            root_index: self.index.root_index(&task.task_id),
            active_refs: selected_refs,
            recent_results: self.recent_results(&task.task_id)?,
            memory: self.memory_notes(task)?,
            active_skills: skills
                .iter()
                .map(|s| ActiveSkill {
                    name: s.name.clone(),
                    version: s.version.clone(),
                    description: s.description.clone(),
                    instructions: s
                        .instructions
                        .chars()
                        .take(MAX_INSTRUCTIONS_CHARS)
                        .collect(),
                })
                .collect(),
            budget: json!({
                "active_context_tokens": self.active_context_tokens,
                "tool_calls_remaining": task
                    .budget
                    .max_tool_calls
                    .map(|max| max.saturating_sub(state.tool_calls_made)),
            }),
            retrieval_receipt: Some(receipt),
        })
    }

    pub fn envelope_hash(envelope: &ContextEnvelope) -> String {
        let value = serde_json::to_value(envelope).unwrap_or(Value::Null);
        crate::ids::canonical_hash(&value)
    }

    fn state_summary(state: &TaskState) -> Value {
        let last_material = state
            .checkpoint
            .get("note")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
            .or_else(|| state.completed_outputs.last().cloned());
        json!({
            "current_status": state.status,
            "last_material_result": last_material,
            "open_failure": state.unresolved_errors.last().cloned(),
            "open_issues": state.open_issues,
            "tool_calls_made": state.tool_calls_made,
            "model_calls_made": state.model_calls_made,
        })
    }

    /// Bounded memory: Way lessons first (mistakes are what the agent must
    /// not repeat), then other relevant notes (progressive disclosure).
    fn memory_notes(&self, task: &Task) -> Result<Vec<Value>> {
        let Some(memory) = &self.memory else {
            return Ok(Vec::new());
        };
        let way = memory.way_relevant(&task.goal, MEMORY_ENVELOPE_LIMIT)?;
        let mut out: Vec<Value> = way.iter().map(|n| n.to_value()).collect();
        let others = memory.relevant(&task.goal, Some(&task.task_id), MEMORY_ENVELOPE_LIMIT * 2)?;
        for note in others {
            if out.len() >= MEMORY_ENVELOPE_LIMIT {
                break;
            }
            if note.kind != "way" {
                out.push(note.to_value());
            }
        }
        Ok(out)
    }

    /// Bounded previews of recent tool/meta results (short previews + refs,
    /// never giant payloads — plan §4 cost-control rules).
    pub fn recent_results(&self, task_id: &str) -> Result<Vec<Value>> {
        let Some(ledger) = &self.ledger else {
            return Ok(Vec::new());
        };
        let mut recent: Vec<Value> = Vec::new();
        for event in ledger.events_for(task_id, 0)? {
            if event.event_type != "tool.committed" {
                continue;
            }
            recent.push(json!({
                "operation_id": event.payload.get("operation_id").cloned().unwrap_or(Value::Null),
                "status": event.payload.get("status").cloned().unwrap_or(Value::Null),
                "result_preview": event.payload.get("result_preview").cloned().unwrap_or(Value::Null),
                "result_refs": event.payload.get("result_refs").cloned().unwrap_or_else(|| json!([])),
                "side_effects": event.payload.get("side_effects").cloned().unwrap_or_else(|| json!([])),
            }));
        }
        if recent.len() > RECENT_RESULTS_WINDOW {
            recent = recent.split_off(recent.len() - RECENT_RESULTS_WINDOW);
        }
        Ok(recent)
    }
}
