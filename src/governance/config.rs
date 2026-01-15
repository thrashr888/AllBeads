//! Policy configuration loading from YAML files

use super::policy::{Policy, PolicyConfig, PolicySeverity, PolicyType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Policy configuration file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoliciesConfig {
    /// List of policy definitions
    pub policies: Vec<PolicyDef>,
}

/// A policy definition in the config file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDef {
    /// Policy name
    pub name: String,

    /// Whether enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Severity level
    #[serde(default)]
    pub severity: SeverityDef,

    /// Policy type (snake_case)
    #[serde(rename = "type")]
    pub policy_type: String,

    /// Optional description
    #[serde(default)]
    pub description: Option<String>,

    /// Type-specific configuration
    #[serde(default)]
    pub config: HashMap<String, serde_yaml::Value>,
}

fn default_true() -> bool {
    true
}

/// Severity in config file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeverityDef {
    Error,
    #[default]
    Warning,
    Info,
}

impl From<SeverityDef> for PolicySeverity {
    fn from(s: SeverityDef) -> Self {
        match s {
            SeverityDef::Error => PolicySeverity::Error,
            SeverityDef::Warning => PolicySeverity::Warning,
            SeverityDef::Info => PolicySeverity::Info,
        }
    }
}

impl PoliciesConfig {
    /// Load policies from a YAML file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| format!("Failed to read policies file: {}", e))?;

        serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse policies YAML: {}", e))
    }

    /// Load policies from the default location (.beads/policies.yaml)
    pub fn from_beads_dir(beads_dir: impl AsRef<Path>) -> Result<Self, String> {
        let path = beads_dir.as_ref().join("policies.yaml");
        if path.exists() {
            Self::from_file(path)
        } else {
            // Return empty config if file doesn't exist
            Ok(Self {
                policies: Vec::new(),
            })
        }
    }

    /// Convert to Policy objects
    pub fn to_policies(&self) -> Vec<Policy> {
        self.policies
            .iter()
            .filter_map(|def| def.to_policy())
            .collect()
    }
}

impl PolicyDef {
    /// Convert to a Policy object
    pub fn to_policy(&self) -> Option<Policy> {
        let policy_type = self.parse_policy_type()?;

        let description = self
            .description
            .clone()
            .unwrap_or_else(|| Policy::default_description_for(&policy_type));

        Some(Policy {
            name: self.name.clone(),
            enabled: self.enabled,
            description,
            policy_type,
            config: self.to_policy_config(),
            severity: self.severity.clone().into(),
        })
    }

    fn parse_policy_type(&self) -> Option<PolicyType> {
        match self.policy_type.as_str() {
            "require_description" => Some(PolicyType::RequireDescription),
            "max_in_progress" => {
                let max_count = self
                    .config
                    .get("max_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(3) as usize;
                Some(PolicyType::MaxInProgress { max_count })
            }
            "require_labels" => {
                let min_count = self
                    .config
                    .get("min_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as usize;
                Some(PolicyType::RequireLabels { min_count })
            }
            "dependency_cycle_check" => Some(PolicyType::DependencyCycleCheck),
            "require_priority" => Some(PolicyType::RequirePriority),
            "require_assignee" => Some(PolicyType::RequireAssignee),
            other => Some(PolicyType::Custom {
                rule_name: other.to_string(),
            }),
        }
    }

    fn to_policy_config(&self) -> PolicyConfig {
        let mut config = PolicyConfig::new();
        for (key, value) in &self.config {
            if let Some(s) = value.as_str() {
                config = config.with_option(key, s);
            } else if let Some(n) = value.as_u64() {
                config = config.with_option(key, n.to_string());
            } else if let Some(b) = value.as_bool() {
                config = config.with_option(key, b.to_string());
            }
        }
        config
    }
}

impl Policy {
    /// Get default description for a policy type
    pub fn default_description_for(policy_type: &PolicyType) -> String {
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
            PolicyType::Custom { rule_name } => format!("Custom rule: {}", rule_name),
        }
    }
}

/// Load policies from a context's .beads directory
pub fn load_policies_for_context(context_path: impl AsRef<Path>) -> Vec<Policy> {
    let beads_dir = context_path.as_ref().join(".beads");
    match PoliciesConfig::from_beads_dir(&beads_dir) {
        Ok(config) => config.to_policies(),
        Err(e) => {
            eprintln!(
                "Warning: Failed to load policies from {:?}: {}",
                beads_dir, e
            );
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_policies_yaml() {
        let yaml = r#"
policies:
  - name: require-description
    enabled: true
    severity: warning
    type: require_description

  - name: max-in-progress
    enabled: true
    type: max_in_progress
    config:
      max_count: 3

  - name: dependency-cycle-check
    enabled: false
    severity: error
    type: dependency_cycle_check
"#;

        let config: PoliciesConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.policies.len(), 3);

        let policies = config.to_policies();
        assert_eq!(policies.len(), 3);

        assert_eq!(policies[0].name, "require-description");
        assert!(policies[0].enabled);

        assert_eq!(policies[1].name, "max-in-progress");
        assert!(matches!(
            policies[1].policy_type,
            PolicyType::MaxInProgress { max_count: 3 }
        ));

        assert_eq!(policies[2].name, "dependency-cycle-check");
        assert!(!policies[2].enabled);
        assert_eq!(policies[2].severity, PolicySeverity::Error);
    }

    #[test]
    fn test_empty_config() {
        let yaml = "policies: []";
        let config: PoliciesConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.policies.is_empty());
    }

    #[test]
    fn test_load_allbeads_policies() {
        // Test loading the actual AllBeads policies.yaml file
        let policies = load_policies_for_context(".");
        assert!(
            !policies.is_empty(),
            "Should load policies from .beads/policies.yaml"
        );

        // Verify expected policies exist
        let policy_names: Vec<&str> = policies.iter().map(|p| p.name.as_str()).collect();
        assert!(
            policy_names.contains(&"require-description"),
            "Should have require-description policy"
        );
        assert!(
            policy_names.contains(&"max-in-progress"),
            "Should have max-in-progress policy"
        );
        assert!(
            policy_names.contains(&"dependency-cycle-check"),
            "Should have dependency-cycle-check policy"
        );
    }

    #[test]
    fn test_custom_policy_type() {
        let yaml = r#"
policies:
  - name: custom-rule
    enabled: true
    type: my_custom_rule
    description: "A custom policy rule"
"#;
        let config: PoliciesConfig = serde_yaml::from_str(yaml).unwrap();
        let policies = config.to_policies();
        assert_eq!(policies.len(), 1);
        assert!(matches!(
            &policies[0].policy_type,
            PolicyType::Custom { rule_name } if rule_name == "my_custom_rule"
        ));
    }
}
