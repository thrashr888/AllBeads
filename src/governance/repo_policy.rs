//! Repository-level policy definitions and checking
//!
//! These policies check repository state (files, structure, configuration)
//! rather than bead-level checks handled by the existing policy module.

use crate::config::AllBeadsConfig;
use crate::governance::agents::{detect_agents, AgentType};
use crate::governance::policy::Enforcement;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Repository policy check types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RepoPolicyCheck {
    /// Check that one or more files/directories exist
    FileExists {
        paths: Vec<String>,
        #[serde(default)]
        all: bool, // true = all must exist, false = any one must exist
    },

    /// Check that one or more files/directories exist (any)
    FileExistsAny { paths: Vec<String> },

    /// Check minimum onboarding score
    OnboardingScore { minimum: u8 },

    /// Check for allowed/denied agents
    AgentAllowlist {
        #[serde(default)]
        allowed: Vec<String>,
        #[serde(default)]
        denied: Vec<String>,
    },

    /// Check that a pattern is absent from files
    PatternAbsent {
        patterns: Vec<String>,
        files: Vec<String>,
        #[serde(default)]
        exclude: Vec<String>,
    },
}

/// A repository-level policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoPolicy {
    /// Policy name/ID
    pub name: String,
    /// Whether this policy is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Enforcement level
    #[serde(default)]
    pub enforcement: Enforcement,
    /// Human-readable description
    pub description: String,
    /// The check to perform
    pub check: RepoPolicyCheck,
}

fn default_true() -> bool {
    true
}

/// Result of checking a single policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCheckResult {
    /// Policy that was checked
    pub policy_name: String,
    /// Whether the check passed
    pub passed: bool,
    /// Enforcement level
    pub enforcement: Enforcement,
    /// Message describing the result
    pub message: String,
    /// Suggested remediation
    pub remediation: Option<String>,
}

impl PolicyCheckResult {
    pub fn pass(policy_name: &str, enforcement: Enforcement) -> Self {
        Self {
            policy_name: policy_name.to_string(),
            passed: true,
            enforcement,
            message: "Policy check passed".to_string(),
            remediation: None,
        }
    }

    pub fn fail(
        policy_name: &str,
        enforcement: Enforcement,
        message: impl Into<String>,
        remediation: Option<String>,
    ) -> Self {
        Self {
            policy_name: policy_name.to_string(),
            passed: false,
            enforcement,
            message: message.into(),
            remediation,
        }
    }
}

/// Repository policy configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoPolicyConfig {
    /// Version of the config schema
    #[serde(default = "default_version")]
    pub version: u32,
    /// Global settings
    #[serde(default)]
    pub settings: RepoPolicySettings,
    /// Policy definitions
    #[serde(default)]
    pub policies: HashMap<String, RepoPolicy>,
    /// Per-repo exemptions
    #[serde(default)]
    pub exemptions: Vec<PolicyExemption>,
}

fn default_version() -> u32 {
    1
}

/// Global policy settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoPolicySettings {
    /// Default enforcement level for new policies
    #[serde(default)]
    pub default_enforcement: Enforcement,
}

impl Default for RepoPolicySettings {
    fn default() -> Self {
        Self {
            default_enforcement: Enforcement::SoftMandatory,
        }
    }
}

/// Policy exemption for a specific repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyExemption {
    /// Repository name (or pattern)
    pub repo: String,
    /// Policy to exempt (or "*" for all)
    pub policy: String,
    /// Reason for exemption
    pub reason: String,
    /// Expiration date (YYYY-MM-DD)
    #[serde(default)]
    pub expires: Option<String>,
    /// Who approved this exemption
    #[serde(default)]
    pub approved_by: Option<String>,
}

impl RepoPolicyConfig {
    /// Load from a YAML file
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default_policies());
        }
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Save to a YAML file
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get default policies
    pub fn default_policies() -> Self {
        let mut policies = HashMap::new();

        // require-beads
        policies.insert(
            "require-beads".to_string(),
            RepoPolicy {
                name: "require-beads".to_string(),
                enabled: true,
                enforcement: Enforcement::SoftMandatory,
                description: "Repository must have beads initialized".to_string(),
                check: RepoPolicyCheck::FileExists {
                    paths: vec![".beads/".to_string(), ".beads/issues.jsonl".to_string()],
                    all: false,
                },
            },
        );

        // require-agent-config
        policies.insert(
            "require-agent-config".to_string(),
            RepoPolicy {
                name: "require-agent-config".to_string(),
                enabled: true,
                enforcement: Enforcement::Advisory,
                description: "Repository should have agent configuration".to_string(),
                check: RepoPolicyCheck::FileExistsAny {
                    paths: vec![
                        "CLAUDE.md".to_string(),
                        ".claude/".to_string(),
                        ".github/copilot-instructions.md".to_string(),
                        ".cursorrules".to_string(),
                    ],
                },
            },
        );

        // require-readme
        policies.insert(
            "require-readme".to_string(),
            RepoPolicy {
                name: "require-readme".to_string(),
                enabled: true,
                enforcement: Enforcement::Advisory,
                description: "Repository should have a README".to_string(),
                check: RepoPolicyCheck::FileExistsAny {
                    paths: vec![
                        "README.md".to_string(),
                        "README.rst".to_string(),
                        "README.txt".to_string(),
                        "README".to_string(),
                    ],
                },
            },
        );

        Self {
            version: 1,
            settings: RepoPolicySettings::default(),
            policies,
            exemptions: Vec::new(),
        }
    }

    /// Check if a repo has an exemption for a policy
    pub fn has_exemption(&self, repo_name: &str, policy_name: &str) -> Option<&PolicyExemption> {
        self.exemptions.iter().find(|e| {
            (e.repo == repo_name || e.repo == "*")
                && (e.policy == policy_name || e.policy == "*")
                && !Self::is_expired(&e.expires)
        })
    }

    fn is_expired(expires: &Option<String>) -> bool {
        if let Some(exp) = expires {
            if let Ok(exp_date) = chrono::NaiveDate::parse_from_str(exp, "%Y-%m-%d") {
                return exp_date < chrono::Utc::now().date_naive();
            }
        }
        false
    }
}

/// Check a repository against a policy
pub fn check_policy(repo_path: &Path, policy: &RepoPolicy) -> PolicyCheckResult {
    match &policy.check {
        RepoPolicyCheck::FileExists { paths, all } => {
            let exists: Vec<bool> = paths.iter().map(|p| repo_path.join(p).exists()).collect();

            let passed = if *all {
                exists.iter().all(|&e| e)
            } else {
                exists.iter().any(|&e| e)
            };

            if passed {
                PolicyCheckResult::pass(&policy.name, policy.enforcement)
            } else {
                let missing: Vec<_> = paths
                    .iter()
                    .zip(exists.iter())
                    .filter(|(_, &e)| !e)
                    .map(|(p, _)| p.as_str())
                    .collect();
                PolicyCheckResult::fail(
                    &policy.name,
                    policy.enforcement,
                    format!("Missing: {}", missing.join(", ")),
                    Some(format!(
                        "Create the required file(s): {}",
                        missing.join(", ")
                    )),
                )
            }
        }

        RepoPolicyCheck::FileExistsAny { paths } => {
            let exists = paths.iter().any(|p| repo_path.join(p).exists());

            if exists {
                PolicyCheckResult::pass(&policy.name, policy.enforcement)
            } else {
                PolicyCheckResult::fail(
                    &policy.name,
                    policy.enforcement,
                    format!("None of these exist: {}", paths.join(", ")),
                    Some(format!("Create one of: {}", paths.join(", "))),
                )
            }
        }

        RepoPolicyCheck::OnboardingScore { minimum } => {
            // TODO: Integrate with onboarding module
            // For now, return pass with a note
            PolicyCheckResult {
                policy_name: policy.name.clone(),
                passed: true,
                enforcement: policy.enforcement,
                message: format!(
                    "Onboarding score check (minimum: {}%) - not yet implemented",
                    minimum
                ),
                remediation: None,
            }
        }

        RepoPolicyCheck::AgentAllowlist { allowed, denied } => {
            let scan = detect_agents(repo_path);
            let detected_agents: Vec<AgentType> = scan.agent_types();

            // Check for denied agents
            for agent in &detected_agents {
                let agent_id = agent.id();
                if denied.iter().any(|d| d.to_lowercase() == agent_id) {
                    return PolicyCheckResult::fail(
                        &policy.name,
                        policy.enforcement,
                        format!("Denied agent detected: {}", agent.name()),
                        Some(format!("Remove {} configuration", agent.name())),
                    );
                }
            }

            // If allowed list is specified, check all detected agents are in it
            if !allowed.is_empty() {
                for agent in &detected_agents {
                    let agent_id = agent.id();
                    if !allowed.iter().any(|a| a.to_lowercase() == agent_id) {
                        return PolicyCheckResult::fail(
                            &policy.name,
                            policy.enforcement,
                            format!("Unapproved agent detected: {}", agent.name()),
                            Some(format!("Only allowed agents: {}", allowed.join(", "))),
                        );
                    }
                }
            }

            PolicyCheckResult::pass(&policy.name, policy.enforcement)
        }

        RepoPolicyCheck::PatternAbsent {
            patterns,
            files: _,
            exclude: _,
        } => {
            // TODO: Implement pattern matching
            PolicyCheckResult {
                policy_name: policy.name.clone(),
                passed: true,
                enforcement: policy.enforcement,
                message: format!(
                    "Pattern check ({} patterns) - not yet implemented",
                    patterns.len()
                ),
                remediation: None,
            }
        }
    }
}

/// Check all policies against a repository
pub fn check_all_policies(
    repo_path: &Path,
    repo_name: &str,
    config: &RepoPolicyConfig,
) -> Vec<PolicyCheckResult> {
    let mut results = Vec::new();

    for (name, policy) in &config.policies {
        if !policy.enabled {
            continue;
        }

        // Check for exemption
        if let Some(exemption) = config.has_exemption(repo_name, name) {
            results.push(PolicyCheckResult {
                policy_name: name.clone(),
                passed: true,
                enforcement: policy.enforcement,
                message: format!("Exempt: {}", exemption.reason),
                remediation: None,
            });
            continue;
        }

        results.push(check_policy(repo_path, policy));
    }

    results
}

/// Get the default policies config path
pub fn default_policies_path() -> std::path::PathBuf {
    AllBeadsConfig::default_path()
        .parent()
        .map(|p| p.join("governance/policies.yaml"))
        .unwrap_or_else(|| std::path::PathBuf::from("policies.yaml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_exists_check() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("README.md"), "# Test").unwrap();

        let policy = RepoPolicy {
            name: "test".to_string(),
            enabled: true,
            enforcement: Enforcement::SoftMandatory,
            description: "Test".to_string(),
            check: RepoPolicyCheck::FileExistsAny {
                paths: vec!["README.md".to_string()],
            },
        };

        let result = check_policy(dir.path(), &policy);
        assert!(result.passed);
    }

    #[test]
    fn test_file_missing_check() {
        let dir = TempDir::new().unwrap();

        let policy = RepoPolicy {
            name: "test".to_string(),
            enabled: true,
            enforcement: Enforcement::SoftMandatory,
            description: "Test".to_string(),
            check: RepoPolicyCheck::FileExistsAny {
                paths: vec!["README.md".to_string()],
            },
        };

        let result = check_policy(dir.path(), &policy);
        assert!(!result.passed);
    }

    #[test]
    fn test_default_policies() {
        let config = RepoPolicyConfig::default_policies();
        assert!(config.policies.contains_key("require-beads"));
        assert!(config.policies.contains_key("require-agent-config"));
        assert!(config.policies.contains_key("require-readme"));
    }

    #[test]
    fn test_exemption() {
        let mut config = RepoPolicyConfig::default_policies();
        config.exemptions.push(PolicyExemption {
            repo: "test-repo".to_string(),
            policy: "require-beads".to_string(),
            reason: "Test exemption".to_string(),
            expires: None,
            approved_by: Some("test".to_string()),
        });

        assert!(config.has_exemption("test-repo", "require-beads").is_some());
        assert!(config
            .has_exemption("other-repo", "require-beads")
            .is_none());
    }
}
