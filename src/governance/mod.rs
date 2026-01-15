//! Governance module for policy enforcement and compliance checking
//!
//! This module provides:
//! - Policy definitions and configuration
//! - A policy checker that validates beads against defined rules
//! - Built-in policy rules for common governance requirements
//! - Storage for check results
//! - Agent detection across repositories
//!
//! # Architecture
//!
//! The Policy Engine integrates with the Sheriff daemon to run checks
//! during each poll cycle. Results are stored in SQLite for the TUI
//! to display and for audit purposes.
//!
//! # Example
//!
//! ```ignore
//! use allbeads::governance::{PolicyChecker, Policy, PolicyType};
//!
//! let mut checker = PolicyChecker::new();
//! checker.add_policy(Policy::new("require-description", PolicyType::RequireDescription));
//!
//! let results = checker.check_graph(&graph);
//! ```

pub mod agents;
pub mod checker;
pub mod config;
pub mod policy;
pub mod rules;
pub mod storage;

pub use agents::{
    detect_agents, print_agent_scan, AgentDetection, AgentScanResult, AgentType,
    DetectionConfidence,
};
pub use checker::PolicyChecker;
pub use config::{load_policies_for_context, PoliciesConfig};
pub use policy::{Enforcement, Policy, PolicyConfig, PolicySeverity, PolicyType};
pub use rules::PolicyRule;
pub use storage::PolicyStorage;
