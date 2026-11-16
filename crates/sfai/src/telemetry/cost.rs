//! Cost accounting (plan §0 principle 7: measure all costs).
//!
//! Counts model tokens (gross and cached), tool calls, verification runs and
//! wall-clock time. The price table is a configurable engineering constant; a
//! fake model still reports token estimates so the accounting path is
//! exercised offline.

use crate::contracts::ModelUsage;
use serde::Serialize;
use serde_json::json;
use std::time::Instant;

pub const PRICE_PER_MILLION_INPUT: f64 = 1.00;
pub const PRICE_PER_MILLION_OUTPUT: f64 = 3.00;

#[derive(Debug, Clone, Default, Serialize)]
pub struct CostReport {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_input_tokens: u64,
    pub cost_usd: f64,
    pub model_calls: u32,
    pub tool_calls: u32,
    pub verification_runs: u32,
    pub wall_clock_s: f64,
}

impl CostReport {
    pub fn as_value(&self) -> serde_json::Value {
        json!({
            "input_tokens": self.input_tokens,
            "output_tokens": self.output_tokens,
            "cached_input_tokens": self.cached_input_tokens,
            "cost_usd": (self.cost_usd * 1_000_000.0).round() / 1_000_000.0,
            "model_calls": self.model_calls,
            "tool_calls": self.tool_calls,
            "verification_runs": self.verification_runs,
            "wall_clock_s": (self.wall_clock_s * 10_000.0).round() / 10_000.0,
        })
    }
}

pub struct CostLedger {
    report: CostReport,
    started: Instant,
}

impl CostLedger {
    pub fn new() -> Self {
        Self {
            report: CostReport::default(),
            started: Instant::now(),
        }
    }

    pub fn add_model_usage(&mut self, usage: &ModelUsage) {
        self.report.input_tokens += usage.input_tokens;
        self.report.output_tokens += usage.output_tokens;
        self.report.cached_input_tokens += usage.cached_input_tokens;
        self.report.cost_usd += usage.cost_usd;
        self.report.model_calls += 1;
    }

    pub fn add_tool_call(&mut self) {
        self.report.tool_calls += 1;
    }

    pub fn add_verification(&mut self) {
        self.report.verification_runs += 1;
    }

    pub fn report_snapshot(&self) -> CostReport {
        let mut snapshot = self.report.clone();
        snapshot.wall_clock_s = self.started.elapsed().as_secs_f64();
        snapshot
    }

    pub fn report(&mut self) -> CostReport {
        self.report.wall_clock_s = self.started.elapsed().as_secs_f64();
        self.report.clone()
    }
}

impl Default for CostLedger {
    fn default() -> Self {
        Self::new()
    }
}

/// Deterministic token estimate for offline/fake models.
pub fn estimate_usage(input_text: &str, output_text: &str) -> ModelUsage {
    let input_tokens = (input_text.chars().count() / 4).max(1) as u64;
    let output_tokens = (output_text.chars().count() / 4).max(1) as u64;
    let cost = (input_tokens as f64 * PRICE_PER_MILLION_INPUT
        + output_tokens as f64 * PRICE_PER_MILLION_OUTPUT)
        / 1_000_000.0;
    ModelUsage {
        input_tokens,
        output_tokens,
        cached_input_tokens: 0,
        cost_usd: cost,
    }
}
