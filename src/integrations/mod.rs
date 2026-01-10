//! External Integrations
//!
//! Adapters for JIRA, GitHub Issues, and plugin architecture for extensibility.
//!
//! # Overview
//!
//! This module provides bi-directional synchronization between AllBeads and
//! external issue tracking systems. The Sheriff daemon uses these adapters
//! to keep beads in sync with enterprise tools.
//!
//! # Built-in Integrations
//!
//! - **JIRA**: REST API adapter for Atlassian JIRA
//! - **GitHub**: GraphQL API adapter for GitHub Issues
//!
//! # Sync Flow
//!
//! The Sheriff daemon performs sync in these phases:
//!
//! 1. **Ingress** (External → Boss): Pull issues matching filters (e.g., `label:ai-agent`)
//! 2. **Diff**: Compare external state with local Shadow Beads
//! 3. **Egress** (Boss → External): Push status changes back to external systems

pub mod github;
pub mod jira;
pub mod plugin;

// JIRA exports
pub use jira::{
    JiraAdapter, JiraComment, JiraError, JiraFields, JiraIssue, JiraIssueType, JiraPriority,
    JiraStatus, JiraSyncAction, JiraSyncResult, JiraSyncStats, JiraTransition, JiraUser,
};

// GitHub exports
pub use github::{
    CreateIssueRequest, GitHubAdapter, GitHubComment, GitHubError, GitHubIssue, GitHubLabel,
    GitHubSyncAction, GitHubSyncResult, GitHubSyncStats, GitHubUser, IssueNode, UpdateIssueRequest,
};

// Plugin exports
pub use plugin::{ExternalIssue, PluginConfig, PluginSyncResult, StatusMapping, SyncDirection};
