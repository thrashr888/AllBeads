//! Plugin system for AllBeads
//!
//! Provides plugin detection, management, and onboarding capabilities.
//! Integrates with Claude's plugin infrastructure while adding AllBeads-specific
//! onboarding protocols.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Plugin status levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginStatus {
    /// Not installed
    NotInstalled,
    /// Installed but not configured
    Installed,
    /// Init command has been run
    Initialized,
    /// Fully configured
    Configured,
}

impl Default for PluginStatus {
    fn default() -> Self {
        Self::NotInstalled
    }
}

impl PluginStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotInstalled => "not_installed",
            Self::Installed => "installed",
            Self::Initialized => "initialized",
            Self::Configured => "configured",
        }
    }
}

/// Plugin category
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCategory {
    /// Official Claude plugins
    Claude,
    /// Beads-related plugins
    Beads,
    /// Prose documentation plugins
    Prose,
    /// Development tools
    DevTools,
    /// Testing and CI
    Testing,
    /// Other/uncategorized
    Other,
}

impl Default for PluginCategory {
    fn default() -> Self {
        Self::Other
    }
}

/// Information about an installed Claude plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub name: String,
    pub version: Option<String>,
    pub enabled: bool,
    pub path: Option<PathBuf>,
    pub marketplace: Option<String>,
}

/// Plugin from a marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplacePlugin {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: Option<String>,
    pub repository: Option<String>,
    pub category: PluginCategory,
    pub has_onboarding: bool,
}

/// Parsed plugin onboarding protocol from allbeads-onboarding.yaml
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginOnboarding {
    pub schema_version: String,
    pub plugin: String,
    pub version: String,
    #[serde(default)]
    pub relevance: PluginRelevance,
    #[serde(default)]
    pub detect: DetectionConfig,
    #[serde(default)]
    pub status_levels: Vec<StatusLevel>,
    #[serde(default)]
    pub prerequisites: Vec<Prerequisite>,
    #[serde(default)]
    pub onboard: OnboardingSteps,
    #[serde(default)]
    pub uninstall: Option<UninstallSteps>,
    #[serde(default)]
    pub hooks: Option<PluginHooks>,
}

/// When should this plugin be suggested?
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginRelevance {
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub frameworks: Vec<String>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub always_suggest: bool,
    #[serde(default)]
    pub user_requested: bool,
}

/// How to detect if plugin is installed/configured
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DetectionConfig {
    #[serde(default)]
    pub files: Vec<FileDetection>,
    #[serde(default)]
    pub commands: Vec<CommandDetection>,
}

/// File-based detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDetection {
    pub path: String,
    #[serde(default)]
    pub contains: Option<String>,
}

/// Command-based detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDetection {
    pub command: String,
    #[serde(default)]
    pub expected_output: Option<String>,
    #[serde(default)]
    pub success_exit_code: Option<i32>,
}

/// Status level definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusLevel {
    pub level: String,
    pub name: String,
    pub detect: DetectionConfig,
}

/// A prerequisite that must be installed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prerequisite {
    pub name: String,
    pub description: String,
    pub check: CommandDetection,
    #[serde(default)]
    pub install: InstallMethods,
}

/// Multiple ways to install a prerequisite
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstallMethods {
    pub cargo: Option<String>,
    pub brew: Option<String>,
    pub npm: Option<String>,
    pub pip: Option<String>,
    pub manual: Option<String>,
}

/// Onboarding steps
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OnboardingSteps {
    #[serde(default)]
    pub steps: Vec<OnboardingStep>,
}

/// Individual onboarding step
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OnboardingStep {
    Command {
        id: String,
        name: String,
        description: String,
        command: String,
        #[serde(default)]
        cwd: Option<String>,
        #[serde(default)]
        skip_if: Option<DetectionConfig>,
    },
    Interactive {
        id: String,
        name: String,
        description: String,
        prompts: Vec<Prompt>,
    },
    Template {
        id: String,
        name: String,
        description: String,
        template: String,
        dest: String,
    },
    Append {
        id: String,
        name: String,
        description: String,
        dest: String,
        content: String,
    },
}

/// Interactive prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub id: String,
    pub message: String,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub options: Vec<String>,
}

/// Uninstall steps
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UninstallSteps {
    #[serde(default)]
    pub steps: Vec<OnboardingStep>,
}

/// Plugin hooks
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginHooks {
    #[serde(default)]
    pub on_sync: Option<String>,
    #[serde(default)]
    pub on_update: Option<String>,
    #[serde(default)]
    pub on_status: Option<String>,
}

/// Curated plugin entry for the built-in plugin list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratedPlugin {
    pub name: String,
    pub description: String,
    pub category: PluginCategory,
    pub marketplace: Option<String>,
    pub repository: Option<String>,
    pub has_onboarding: bool,
    #[serde(default)]
    pub relevance: PluginRelevance,
}

/// Plugin registry containing curated plugins
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginRegistry {
    pub plugins: Vec<CuratedPlugin>,
}

impl PluginRegistry {
    /// Get the built-in curated plugin list
    pub fn builtin() -> Self {
        Self {
            plugins: vec![
                CuratedPlugin {
                    name: "beads".to_string(),
                    description: "Git-backed issue tracker with dependencies".to_string(),
                    category: PluginCategory::Beads,
                    marketplace: Some("steveyegge/beads".to_string()),
                    repository: Some("https://github.com/steveyegge/beads".to_string()),
                    has_onboarding: true,
                    relevance: PluginRelevance {
                        always_suggest: true,
                        ..Default::default()
                    },
                },
                CuratedPlugin {
                    name: "prose".to_string(),
                    description: "AI-assisted documentation system".to_string(),
                    category: PluginCategory::Prose,
                    marketplace: Some("openprose/prose".to_string()),
                    repository: Some("https://github.com/openprose/prose".to_string()),
                    has_onboarding: true,
                    relevance: PluginRelevance::default(),
                },
                CuratedPlugin {
                    name: "mcp-github".to_string(),
                    description: "GitHub integration via MCP".to_string(),
                    category: PluginCategory::DevTools,
                    marketplace: Some("anthropics/mcp-github".to_string()),
                    repository: None,
                    has_onboarding: false,
                    relevance: PluginRelevance {
                        files: vec![".github".to_string()],
                        ..Default::default()
                    },
                },
                CuratedPlugin {
                    name: "mcp-filesystem".to_string(),
                    description: "Filesystem access via MCP".to_string(),
                    category: PluginCategory::DevTools,
                    marketplace: Some("anthropics/mcp-filesystem".to_string()),
                    repository: None,
                    has_onboarding: false,
                    relevance: PluginRelevance {
                        always_suggest: true,
                        ..Default::default()
                    },
                },
                CuratedPlugin {
                    name: "prettier".to_string(),
                    description: "Code formatting with Prettier".to_string(),
                    category: PluginCategory::DevTools,
                    marketplace: None,
                    repository: None,
                    has_onboarding: false,
                    relevance: PluginRelevance {
                        languages: vec!["javascript".to_string(), "typescript".to_string()],
                        files: vec![".prettierrc".to_string(), "prettier.config.js".to_string()],
                        ..Default::default()
                    },
                },
                CuratedPlugin {
                    name: "eslint".to_string(),
                    description: "JavaScript/TypeScript linting".to_string(),
                    category: PluginCategory::DevTools,
                    marketplace: None,
                    repository: None,
                    has_onboarding: false,
                    relevance: PluginRelevance {
                        languages: vec!["javascript".to_string(), "typescript".to_string()],
                        files: vec![".eslintrc".to_string(), ".eslintrc.json".to_string(), "eslint.config.js".to_string()],
                        ..Default::default()
                    },
                },
                CuratedPlugin {
                    name: "jest".to_string(),
                    description: "JavaScript testing framework".to_string(),
                    category: PluginCategory::Testing,
                    marketplace: None,
                    repository: None,
                    has_onboarding: false,
                    relevance: PluginRelevance {
                        languages: vec!["javascript".to_string(), "typescript".to_string()],
                        files: vec!["jest.config.js".to_string(), "jest.config.ts".to_string()],
                        ..Default::default()
                    },
                },
                CuratedPlugin {
                    name: "pytest".to_string(),
                    description: "Python testing framework".to_string(),
                    category: PluginCategory::Testing,
                    marketplace: None,
                    repository: None,
                    has_onboarding: false,
                    relevance: PluginRelevance {
                        languages: vec!["python".to_string()],
                        files: vec!["pytest.ini".to_string(), "pyproject.toml".to_string()],
                        ..Default::default()
                    },
                },
            ],
        }
    }

    /// Find a plugin by name
    pub fn find(&self, name: &str) -> Option<&CuratedPlugin> {
        self.plugins.iter().find(|p| p.name == name)
    }

    /// Get plugins relevant to given languages and files
    pub fn recommend(&self, languages: &[String], files: &[String]) -> Vec<&CuratedPlugin> {
        self.plugins
            .iter()
            .filter(|p| {
                // Always suggest if marked
                if p.relevance.always_suggest {
                    return true;
                }
                // Check language match
                for lang in &p.relevance.languages {
                    if languages.iter().any(|l| l.to_lowercase() == lang.to_lowercase()) {
                        return true;
                    }
                }
                // Check file match
                for file_pattern in &p.relevance.files {
                    if files.iter().any(|f| f.contains(file_pattern)) {
                        return true;
                    }
                }
                false
            })
            .collect()
    }
}

/// Claude plugin state from ~/.claude/plugins/
#[derive(Debug, Clone, Default)]
pub struct ClaudePluginState {
    pub installed_plugins: Vec<InstalledPlugin>,
    pub known_marketplaces: Vec<String>,
    pub enabled_plugins: Vec<String>,
}

impl ClaudePluginState {
    /// Load Claude plugin state from the filesystem
    pub fn load() -> Self {
        let mut state = Self::default();

        // Try to load installed_plugins.json
        if let Some(home) = dirs::home_dir() {
            let plugins_dir = home.join(".claude").join("plugins");

            // Load installed plugins
            let installed_path = plugins_dir.join("installed_plugins.json");
            if installed_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&installed_path) {
                    if let Ok(plugins) = serde_json::from_str::<Vec<InstalledPlugin>>(&content) {
                        state.installed_plugins = plugins;
                    }
                }
            }

            // Load known marketplaces
            let marketplaces_path = plugins_dir.join("known_marketplaces.json");
            if marketplaces_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&marketplaces_path) {
                    if let Ok(marketplaces) = serde_json::from_str::<Vec<String>>(&content) {
                        state.known_marketplaces = marketplaces;
                    }
                }
            }

            // Load enabled plugins from settings
            let settings_path = home.join(".claude").join("settings.json");
            if settings_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&settings_path) {
                    if let Ok(settings) = serde_json::from_str::<HashMap<String, serde_json::Value>>(&content) {
                        if let Some(enabled) = settings.get("enabledPlugins") {
                            if let Some(arr) = enabled.as_array() {
                                state.enabled_plugins = arr
                                    .iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect();
                            }
                        }
                    }
                }
            }
        }

        state
    }

    /// Check if a plugin is installed
    pub fn is_installed(&self, name: &str) -> bool {
        self.installed_plugins.iter().any(|p| p.name == name)
    }

    /// Check if a plugin is enabled
    pub fn is_enabled(&self, name: &str) -> bool {
        self.enabled_plugins.iter().any(|p| p == name)
    }
}

/// Load plugin onboarding protocol from a path
pub fn load_onboarding(path: &PathBuf) -> Option<PluginOnboarding> {
    let onboarding_path = path.join(".claude-plugin").join("allbeads-onboarding.yaml");
    if onboarding_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&onboarding_path) {
            if let Ok(onboarding) = serde_yaml::from_str::<PluginOnboarding>(&content) {
                return Some(onboarding);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_registry() {
        let registry = PluginRegistry::builtin();
        assert!(!registry.plugins.is_empty());
        assert!(registry.find("beads").is_some());
    }

    #[test]
    fn test_recommend_plugins() {
        let registry = PluginRegistry::builtin();
        let languages = vec!["typescript".to_string()];
        let files = vec!["package.json".to_string()];
        let recommended = registry.recommend(&languages, &files);
        assert!(!recommended.is_empty());
    }
}
