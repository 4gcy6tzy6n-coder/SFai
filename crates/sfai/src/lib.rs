//! System for future ai (sfai) — model-native agent platform, Rust edition.

pub mod benchmark;
pub mod contracts;
pub mod error;
pub mod eval;
pub mod ids;

pub mod storage {
    pub mod db;
    pub mod event_ledger;
    pub mod objects;
}

pub mod authority {
    pub mod approvals;
    pub mod tasks;
}

pub mod capabilities {
    pub mod registry;
    pub mod skills;
}

pub mod context {
    pub mod index;
    pub mod retrieval;
    pub mod wiki;
    pub mod working_set;
}

pub mod runtime {
    pub mod broker;
    pub mod policy;
}

pub mod core {
    pub mod action_intents;
    pub mod context_envelope;
    pub mod decision_kernel;
    pub mod model_adapter;
    pub mod termination;
}

pub mod verify {
    pub mod diagnostics;
    pub mod engine;
}

pub mod telemetry {
    pub mod cost;
    pub mod replay;
    pub mod tracing;
}

pub mod integrations {
    pub mod fake_tools;
    pub mod filesystem;
    pub mod providers {
        pub mod fake;
        pub mod forced_pipeline;
        pub mod openai_compat;
        pub mod profiles;
    }
}

pub mod app;
pub mod fixtures;
pub mod server;
pub mod ui;

pub use error::{Result, SfaiError};
