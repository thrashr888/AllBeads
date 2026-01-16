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
pub mod repo_policy;
pub mod rules;
pub mod scanner;
pub mod storage;
pub mod usage;

pub use agents::{
    detect_agents, print_agent_scan, AgentDetection, AgentScanResult, AgentType,
    DetectionConfidence,
};
pub use checker::PolicyChecker;
pub use config::{load_policies_for_context, PoliciesConfig};
pub use policy::{Enforcement, Policy, PolicyConfig, PolicySeverity, PolicyType};
pub use repo_policy::{
    check_all_policies, check_policy, default_policies_path, PolicyCheckResult, PolicyExemption,
    RepoPolicy, RepoPolicyCheck, RepoPolicyConfig,
};
pub use rules::PolicyRule;
pub use scanner::{
    format_scan_result_csv, format_scan_result_junit, print_scan_result, GitHubScanner,
    OnboardingPriority, ScanFilter, ScanResult, ScanSource, ScanSummary, ScannedRepo,
};
pub use storage::PolicyStorage;
pub use usage::{print_usage_stats, UsageRecord, UsageStats, UsageStorage, UsageTrend};
