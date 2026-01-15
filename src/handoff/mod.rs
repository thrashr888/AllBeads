//! Agent handoff module
//!
//! Provides functionality for handing off beads to AI agents.
//! This is fire-and-forget delegation - we launch agents with context and move on.

mod types;

pub use types::{AgentHandoff, AgentType};
