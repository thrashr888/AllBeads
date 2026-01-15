//! Policy definition and configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The type of policy rule to apply
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PolicyType {
    /// All beads must have a description
    RequireDescription,
    /// Limit concurrent in-progress beads per assignee
    MaxInProgress {
        #[serde(default = "default_max_count")]
        max_count: usize,
    },
    /// All beads must have at least one label
    RequireLabels {
        #[serde(default = "default_min_labels")]
        min_count: usize,
    },
    /// Detect and prevent circular dependencies
    DependencyCycleCheck,
    /// Beads must have a valid priority set
    RequirePriority,
    /// Open beads should have an assignee
    RequireAssignee,
    /// Custom rule with arbitrary configuration
    Custom { rule_name: String },
}

fn default_max_count() -> usize {
    3
}

fn default_min_labels() -> usize {
    1
}

/// Policy configuration options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Extra configuration options as key-value pairs
    #[serde(flatten)]
    pub options: HashMap<String, String>,
}

impl PolicyConfig {
    /// Create a new empty configuration
    pub fn new() -> Self {
        Self {
            options: HashMap::new(),
        }
    }

    /// Add a configuration option
    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }

    /// Get a configuration option
    pub fn get(&self, key: &str) -> Option<&str> {
        self.options.get(key).map(|s| s.as_str())
    }
}

/// A policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Unique name for this policy
    pub name: String,
    /// Whether this policy is enabled
    pub enabled: bool,
    /// Human-readable description
    pub description: String,
    /// The type of rule to apply
    pub policy_type: PolicyType,
    /// Additional configuration
    #[serde(default)]
    pub config: PolicyConfig,
    /// Severity level (error, warning, info)
    #[serde(default)]
    pub severity: PolicySeverity,
}

/// Severity level for policy violations
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PolicySeverity {
    /// Error - blocks work
    Error,
    /// Warning - should be addressed
    #[default]
    Warning,
    /// Info - informational only
    Info,
}

/// Enforcement level for policies (inspired by HCP Terraform)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Enforcement {
    /// Advisory: warn only, never blocks
    Advisory,
    /// Soft Mandatory: blocks by default, can be overridden with justification
    #[default]
    SoftMandatory,
    /// Hard Mandatory: always blocks, no override possible
    HardMandatory,
}

impl Enforcement {
    /// Check if this enforcement level can be overridden
    pub fn can_override(&self) -> bool {
        matches!(self, Enforcement::Advisory | Enforcement::SoftMandatory)
    }

    /// Check if this enforcement level blocks by default
    pub fn blocks_by_default(&self) -> bool {
        matches!(
            self,
            Enforcement::SoftMandatory | Enforcement::HardMandatory
        )
    }

    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Enforcement::Advisory => "Advisory",
            Enforcement::SoftMandatory => "Soft Mandatory",
            Enforcement::HardMandatory => "Hard Mandatory",
        }
    }

    /// Get short symbol for display
    pub fn symbol(&self) -> &'static str {
        match self {
            Enforcement::Advisory => "○",      // Empty circle - informational
            Enforcement::SoftMandatory => "◐", // Half circle - can override
            Enforcement::HardMandatory => "●", // Full circle - no override
        }
    }
}

impl Policy {
    /// Create a new policy
    pub fn new(name: impl Into<String>, policy_type: PolicyType) -> Self {
        let name = name.into();
        let description = Self::default_description(&policy_type);
        Self {
            name,
            enabled: true,
            description,
            policy_type,
            config: PolicyConfig::default(),
            severity: PolicySeverity::Warning,
        }
    }

    /// Create a builder for constructing a policy
    pub fn builder(name: impl Into<String>) -> PolicyBuilder {
        PolicyBuilder::new(name)
    }

    /// Set the enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the severity
    pub fn with_severity(mut self, severity: PolicySeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Set the configuration
    pub fn with_config(mut self, config: PolicyConfig) -> Self {
        self.config = config;
        self
    }

    fn default_description(policy_type: &PolicyType) -> String {
        match policy_type {
            PolicyType::RequireDescription => "All beads must have a description".to_string(),
            PolicyType::MaxInProgress { max_count } => {
                format!(
                    "Limit concurrent in-progress beads to {} per assignee",
                    max_count
                )
            }
            PolicyType::RequireLabels { min_count } => {
                format!("All beads must have at least {} label(s)", min_count)
            }
            PolicyType::DependencyCycleCheck => {
                "Detect and prevent circular dependencies".to_string()
            }
            PolicyType::RequirePriority => "All beads must have a valid priority set".to_string(),
            PolicyType::RequireAssignee => "Open beads should have an assignee".to_string(),
            PolicyType::Custom { rule_name } => {
                format!("Custom rule: {}", rule_name)
            }
        }
    }
}

/// Builder for constructing policies
pub struct PolicyBuilder {
    name: String,
    enabled: bool,
    description: Option<String>,
    policy_type: Option<PolicyType>,
    config: PolicyConfig,
    severity: PolicySeverity,
}

impl PolicyBuilder {
    /// Create a new policy builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: true,
            description: None,
            policy_type: None,
            config: PolicyConfig::default(),
            severity: PolicySeverity::Warning,
        }
    }

    /// Set the policy type
    pub fn policy_type(mut self, policy_type: PolicyType) -> Self {
        self.policy_type = Some(policy_type);
        self
    }

    /// Set the enabled state
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the severity
    pub fn severity(mut self, severity: PolicySeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Set the configuration
    pub fn config(mut self, config: PolicyConfig) -> Self {
        self.config = config;
        self
    }

    /// Build the policy
    pub fn build(self) -> Option<Policy> {
        let policy_type = self.policy_type?;
        let description = self
            .description
            .unwrap_or_else(|| Policy::default_description(&policy_type));

        Some(Policy {
            name: self.name,
            enabled: self.enabled,
            description,
            policy_type,
            config: self.config,
            severity: self.severity,
        })
    }
}

/// A collection of default policies
pub fn default_policies() -> Vec<Policy> {
    vec![
        Policy::new("require-description", PolicyType::RequireDescription),
        Policy::new(
            "max-in-progress",
            PolicyType::MaxInProgress { max_count: 3 },
        ),
        Policy::new("require-labels", PolicyType::RequireLabels { min_count: 1 })
            .with_enabled(false),
        Policy::new("dependency-cycle-check", PolicyType::DependencyCycleCheck),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_creation() {
        let policy = Policy::new("test", PolicyType::RequireDescription);
        assert_eq!(policy.name, "test");
        assert!(policy.enabled);
        assert_eq!(policy.severity, PolicySeverity::Warning);
    }

    #[test]
    fn test_policy_builder() {
        let policy = Policy::builder("test")
            .policy_type(PolicyType::MaxInProgress { max_count: 5 })
            .severity(PolicySeverity::Error)
            .enabled(false)
            .description("Custom description")
            .build()
            .unwrap();

        assert_eq!(policy.name, "test");
        assert!(!policy.enabled);
        assert_eq!(policy.severity, PolicySeverity::Error);
        assert_eq!(policy.description, "Custom description");
    }

    #[test]
    fn test_default_policies() {
        let policies = default_policies();
        assert!(!policies.is_empty());
        assert!(policies.iter().any(|p| p.name == "require-description"));
    }

    #[test]
    fn test_policy_config() {
        let config = PolicyConfig::new()
            .with_option("key1", "value1")
            .with_option("key2", "value2");

        assert_eq!(config.get("key1"), Some("value1"));
        assert_eq!(config.get("key2"), Some("value2"));
        assert_eq!(config.get("key3"), None);
    }
}
