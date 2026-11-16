//! Model adapter interface (plan §2 / §3.3).
//!
//! The main model is the policy for choosing which action to request; the
//! runtime decides whether it executes. Provider-native calls normalize into
//! the single action-intent envelope.

use crate::contracts::{ContextEnvelope, ModelResponse};

#[derive(Debug)]
pub enum ModelError {
    SimulatedCrash(String),
    Provider(String),
}

impl ModelError {
    pub fn message(&self) -> &str {
        match self {
            ModelError::SimulatedCrash(m) => m,
            ModelError::Provider(m) => m,
        }
    }
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

pub trait ModelAdapter {
    fn model_id(&self) -> String;

    /// Return exactly one normalized action intent for this step.
    fn decide(&mut self, envelope: &ContextEnvelope) -> Result<ModelResponse, ModelError>;
}
