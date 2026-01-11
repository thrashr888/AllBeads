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

// ============================================================================
// Onboarding Executor
// ============================================================================

use std::collections::HashMap as StdHashMap;
use std::io::{self, Write};

/// Results from executing onboarding
#[derive(Debug, Clone, Default)]
pub struct OnboardingResult {
    pub success: bool,
    pub steps_completed: usize,
    pub steps_skipped: usize,
    pub errors: Vec<String>,
    pub prompt_responses: StdHashMap<String, String>,
}

/// Execute onboarding steps for a plugin
pub struct OnboardingExecutor {
    project_path: PathBuf,
    dry_run: bool,
    auto_yes: bool,
    prompt_responses: StdHashMap<String, String>,
}

impl OnboardingExecutor {
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            project_path,
            dry_run: false,
            auto_yes: false,
            prompt_responses: StdHashMap::new(),
        }
    }

    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn auto_yes(mut self, auto_yes: bool) -> Self {
        self.auto_yes = auto_yes;
        self
    }

    /// Execute all onboarding steps
    pub fn execute(&mut self, onboarding: &PluginOnboarding) -> OnboardingResult {
        let mut result = OnboardingResult::default();

        for step in &onboarding.onboard.steps {
            match self.execute_step(step) {
                Ok(skipped) => {
                    if skipped {
                        result.steps_skipped += 1;
                    } else {
                        result.steps_completed += 1;
                    }
                }
                Err(e) => {
                    result.errors.push(e);
                    // Continue with other steps unless critical
                }
            }
        }

        result.success = result.errors.is_empty();
        result.prompt_responses = self.prompt_responses.clone();
        result
    }

    /// Execute a single step, returns Ok(true) if skipped
    fn execute_step(&mut self, step: &OnboardingStep) -> Result<bool, String> {
        match step {
            OnboardingStep::Command {
                id,
                name,
                description,
                command,
                cwd,
                skip_if,
            } => {
                println!("  Step: {}", name);
                println!("    {}", description);

                // Check skip condition
                if let Some(skip_config) = skip_if {
                    if self.check_detection(skip_config) {
                        println!("    → Skipped (already done)");
                        return Ok(true);
                    }
                }

                if self.dry_run {
                    println!("    → Would run: {}", command);
                    return Ok(false);
                }

                // Execute command
                let work_dir = if let Some(dir) = cwd {
                    self.project_path.join(dir)
                } else {
                    self.project_path.clone()
                };

                let output = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .current_dir(&work_dir)
                    .output()
                    .map_err(|e| format!("Failed to run command '{}': {}", id, e))?;

                if output.status.success() {
                    println!("    ✓ Completed");
                    Ok(false)
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(format!("Command '{}' failed: {}", id, stderr))
                }
            }

            OnboardingStep::Interactive {
                id: _,
                name,
                description,
                prompts,
            } => {
                println!("  Step: {}", name);
                println!("    {}", description);

                if self.dry_run {
                    println!("    → Would prompt for {} values", prompts.len());
                    return Ok(false);
                }

                for prompt in prompts {
                    let response = self.get_prompt_response(prompt)?;
                    self.prompt_responses.insert(prompt.id.clone(), response);
                }

                println!("    ✓ Collected responses");
                Ok(false)
            }

            OnboardingStep::Template {
                id,
                name,
                description,
                template,
                dest,
            } => {
                println!("  Step: {}", name);
                println!("    {}", description);

                let dest_path = self.project_path.join(dest);

                if self.dry_run {
                    println!("    → Would create: {}", dest_path.display());
                    return Ok(false);
                }

                // Simple template substitution
                let rendered = self.render_template(template);

                // Ensure parent directory exists
                if let Some(parent) = dest_path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create directory: {}", e))?;
                }

                std::fs::write(&dest_path, rendered)
                    .map_err(|e| format!("Failed to write '{}': {}", id, e))?;

                println!("    ✓ Created {}", dest);
                Ok(false)
            }

            OnboardingStep::Append {
                id,
                name,
                description,
                dest,
                content,
            } => {
                println!("  Step: {}", name);
                println!("    {}", description);

                let dest_path = self.project_path.join(dest);

                if self.dry_run {
                    println!("    → Would append to: {}", dest_path.display());
                    return Ok(false);
                }

                // Read existing content
                let existing = std::fs::read_to_string(&dest_path).unwrap_or_default();

                // Check if content already exists
                let rendered = self.render_template(content);
                if existing.contains(rendered.trim()) {
                    println!("    → Skipped (content already exists)");
                    return Ok(true);
                }

                // Append content
                let mut file = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&dest_path)
                    .map_err(|e| format!("Failed to open '{}': {}", id, e))?;

                writeln!(file, "{}", rendered)
                    .map_err(|e| format!("Failed to append to '{}': {}", id, e))?;

                println!("    ✓ Appended to {}", dest);
                Ok(false)
            }
        }
    }

    /// Check if detection config matches (for skip_if)
    fn check_detection(&self, config: &DetectionConfig) -> bool {
        // Check files
        for file_check in &config.files {
            let path = self.project_path.join(&file_check.path);
            if !path.exists() {
                return false;
            }
            if let Some(ref contains) = file_check.contains {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if !content.contains(contains) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }

        // Check commands
        for cmd_check in &config.commands {
            let output = std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd_check.command)
                .current_dir(&self.project_path)
                .output();

            match output {
                Ok(out) => {
                    let expected_code = cmd_check.success_exit_code.unwrap_or(0);
                    if out.status.code() != Some(expected_code) {
                        return false;
                    }
                    if let Some(ref expected) = cmd_check.expected_output {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        if !stdout.contains(expected) {
                            return false;
                        }
                    }
                }
                Err(_) => return false,
            }
        }

        true
    }

    /// Get response for an interactive prompt
    fn get_prompt_response(&self, prompt: &Prompt) -> Result<String, String> {
        if self.auto_yes {
            // Use default if available
            return Ok(prompt.default.clone().unwrap_or_default());
        }

        // Print prompt
        print!("    ? {}", prompt.message);
        if let Some(ref default) = prompt.default {
            print!(" [{}]", default);
        }
        print!(": ");
        io::stdout().flush().ok();

        // Read response
        let mut response = String::new();
        io::stdin()
            .read_line(&mut response)
            .map_err(|e| format!("Failed to read input: {}", e))?;

        let response = response.trim().to_string();

        // Use default if empty
        if response.is_empty() {
            Ok(prompt.default.clone().unwrap_or_default())
        } else {
            Ok(response)
        }
    }

    /// Simple template rendering with {{ variable }} substitution
    fn render_template(&self, template: &str) -> String {
        let mut result = template.to_string();

        // Replace {{ prompts.key }} with collected values
        for (key, value) in &self.prompt_responses {
            let pattern = format!("{{{{ prompts.{} }}}}", key);
            result = result.replace(&pattern, value);

            // Also try without spaces
            let pattern_no_space = format!("{{{{prompts.{}}}}}", key);
            result = result.replace(&pattern_no_space, value);
        }

        result
    }

    /// Execute uninstall steps
    pub fn execute_uninstall(&mut self, onboarding: &PluginOnboarding) -> OnboardingResult {
        let mut result = OnboardingResult::default();

        if let Some(ref uninstall) = onboarding.uninstall {
            for step in &uninstall.steps {
                match self.execute_step(step) {
                    Ok(skipped) => {
                        if skipped {
                            result.steps_skipped += 1;
                        } else {
                            result.steps_completed += 1;
                        }
                    }
                    Err(e) => {
                        result.errors.push(e);
                    }
                }
            }
        }

        result.success = result.errors.is_empty();
        result
    }
}

/// Check prerequisites for a plugin
pub fn check_prerequisites(
    onboarding: &PluginOnboarding,
    project_path: &PathBuf,
) -> Vec<(String, bool, Option<String>)> {
    let mut results = Vec::new();

    for prereq in &onboarding.prerequisites {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(&prereq.check.command)
            .current_dir(project_path)
            .output();

        let (satisfied, install_hint) = match output {
            Ok(out) => {
                let expected_code = prereq.check.success_exit_code.unwrap_or(0);
                let success = out.status.code() == Some(expected_code);

                let hint = if !success {
                    // Build install hint
                    let methods: Vec<String> = [
                        prereq.install.cargo.as_ref().map(|c| format!("cargo install {}", c)),
                        prereq.install.brew.as_ref().map(|b| format!("brew install {}", b)),
                        prereq.install.npm.as_ref().map(|n| format!("npm install -g {}", n)),
                        prereq.install.pip.as_ref().map(|p| format!("pip install {}", p)),
                        prereq.install.manual.clone(),
                    ]
                    .into_iter()
                    .flatten()
                    .collect();

                    if methods.is_empty() {
                        None
                    } else {
                        Some(methods.join(" or "))
                    }
                } else {
                    None
                };

                (success, hint)
            }
            Err(_) => {
                let hint = prereq.install.manual.clone();
                (false, hint)
            }
        };

        results.push((prereq.name.clone(), satisfied, install_hint));
    }

    results
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
