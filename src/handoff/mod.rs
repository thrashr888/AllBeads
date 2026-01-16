//! Agent handoff module
//!
//! Provides functionality for handing off beads to AI agents.
//! This is fire-and-forget delegation - we launch agents with context and move on.

mod config;
mod types;

pub use config::{get_preferred_agent, is_worktree_enabled, save_preferred_agent};
pub use types::{detect_installed_agents, get_installed_agents, AgentHandoff, AgentType};
