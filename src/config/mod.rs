//! Configuration system
//!
//! Multi-context configuration supporting work/personal Boss repositories.
//!
//! Loads ~/.config/allbeads/config.yaml with support for:
//! - Multiple Boss contexts (work, personal, etc.)
//! - Different authentication strategies per context
//! - JIRA and GitHub integrations
//! - Agent Mail settings
//! - Visualization preferences

mod allbeads_config;
mod boss_context;
pub mod validation;

pub use allbeads_config::{
    AgentMailConfig, AllBeadsConfig, OnboardingConfig, VisualizationConfig, WebAuthConfig,
};
pub use boss_context::{
    AuthStrategy, BossContext, GitHubIntegration, Integrations, JiraIntegration,
};
pub use validation::{validate_config, validate_config_result, ValidationError};
