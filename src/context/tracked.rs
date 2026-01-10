//! Context and detection data structures
//!
//! Defines Context for managing tracked folders and DetectedInfo for project detection.

use super::folder::TrackedFolder;
use super::status::FolderStatus;
use crate::config::Integrations;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Programming language detection
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    Ruby,
    Cpp,
    CSharp,
    Swift,
    Kotlin,
    Php,
    Shell,
    Other(String),
}

impl Language {
    /// Parse language from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Self::Rust,
            "typescript" | "ts" => Self::TypeScript,
            "javascript" | "js" => Self::JavaScript,
            "python" | "py" => Self::Python,
            "go" | "golang" => Self::Go,
            "java" => Self::Java,
            "ruby" | "rb" => Self::Ruby,
            "c++" | "cpp" | "cxx" => Self::Cpp,
            "c#" | "csharp" | "cs" => Self::CSharp,
            "swift" => Self::Swift,
            "kotlin" | "kt" => Self::Kotlin,
            "php" => Self::Php,
            "shell" | "bash" | "sh" => Self::Shell,
            other => Self::Other(other.to_string()),
        }
    }

    /// Get file extensions for this language
    pub fn extensions(&self) -> &[&str] {
        match self {
            Self::Rust => &["rs"],
            Self::TypeScript => &["ts", "tsx"],
            Self::JavaScript => &["js", "jsx", "mjs", "cjs"],
            Self::Python => &["py", "pyi"],
            Self::Go => &["go"],
            Self::Java => &["java"],
            Self::Ruby => &["rb"],
            Self::Cpp => &["cpp", "cxx", "cc", "c++", "h", "hpp"],
            Self::CSharp => &["cs"],
            Self::Swift => &["swift"],
            Self::Kotlin => &["kt", "kts"],
            Self::Php => &["php"],
            Self::Shell => &["sh", "bash"],
            Self::Other(_) => &[],
        }
    }
}

/// Framework detection
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Framework {
    // Frontend
    React,
    Vue,
    Angular,
    Svelte,
    Next,
    Nuxt,

    // Backend
    Express,
    FastApi,
    Django,
    Rails,
    Spring,
    Actix,
    Axum,

    // Mobile
    ReactNative,
    Flutter,

    // Other
    Electron,
    Tauri,
    Other(String),
}

impl Framework {
    /// Get indicator files for detecting this framework
    pub fn indicator_files(&self) -> &[&str] {
        match self {
            Self::React | Self::ReactNative => &["package.json"], // Need to check for react dep
            Self::Vue | Self::Nuxt => &["vue.config.js", "nuxt.config.js", "nuxt.config.ts"],
            Self::Angular => &["angular.json"],
            Self::Svelte => &["svelte.config.js"],
            Self::Next => &["next.config.js", "next.config.ts", "next.config.mjs"],
            Self::Express => &["package.json"], // Need to check for express dep
            Self::FastApi => &["pyproject.toml"], // Need to check for fastapi dep
            Self::Django => &["manage.py", "settings.py"],
            Self::Rails => &["Gemfile", "config/application.rb"],
            Self::Spring => &["pom.xml", "build.gradle"], // Need to check for spring
            Self::Actix => &["Cargo.toml"], // Need to check for actix
            Self::Axum => &["Cargo.toml"],  // Need to check for axum
            Self::Flutter => &["pubspec.yaml"],
            Self::Electron => &["electron.js", "main.js"], // Approximate
            Self::Tauri => &["tauri.conf.json"],
            Self::Other(_) => &[],
        }
    }
}

/// Detected project information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DetectedInfo {
    /// Primary programming languages
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<Language>,

    /// Detected frameworks
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frameworks: Vec<Framework>,

    /// Whether this is a monorepo
    #[serde(default)]
    pub is_monorepo: bool,

    /// Whether this is a git worktree
    #[serde(default)]
    pub is_worktree: bool,

    /// Path to main worktree (if this is a worktree)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_worktree: Option<PathBuf>,

    /// Detected package managers
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub package_managers: Vec<String>,

    /// Whether Claude Code is configured
    #[serde(default)]
    pub has_claude: bool,

    /// Whether Cursor is configured
    #[serde(default)]
    pub has_cursor: bool,

    /// Whether Copilot is configured
    #[serde(default)]
    pub has_copilot: bool,

    /// Whether Aider is configured
    #[serde(default)]
    pub has_aider: bool,

    /// Remote git URL (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_remote: Option<String>,

    /// Default branch name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_branch: Option<String>,
}

impl DetectedInfo {
    /// Check if any language is detected
    pub fn has_languages(&self) -> bool {
        !self.languages.is_empty()
    }

    /// Check if any AI agent is configured
    pub fn has_any_agent(&self) -> bool {
        self.has_claude || self.has_cursor || self.has_copilot || self.has_aider
    }

    /// Get primary language (first in list)
    pub fn primary_language(&self) -> Option<&Language> {
        self.languages.first()
    }
}

/// Default settings for new folders in a context
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextDefaults {
    /// Default persona for new folders
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,

    /// Default sync enabled
    #[serde(default)]
    pub sync_enabled: bool,

    /// Default sync interval
    #[serde(default = "default_sync_interval")]
    pub sync_interval: u64,

    /// Default labels to apply
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
}

fn default_sync_interval() -> u64 {
    300 // 5 minutes in seconds
}

/// A context containing multiple tracked folders
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    /// Context name (e.g., "work", "personal")
    pub name: String,

    /// Tracked folders in this context
    #[serde(default)]
    pub folders: Vec<TrackedFolder>,

    /// Default settings for new folders
    #[serde(default)]
    pub defaults: ContextDefaults,

    /// External integrations
    #[serde(default)]
    pub integrations: Integrations,

    /// Last sync timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync: Option<String>,

    /// Context description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Context {
    /// Create a new context
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            folders: Vec::new(),
            defaults: ContextDefaults::default(),
            integrations: Integrations::default(),
            last_sync: None,
            description: None,
        }
    }

    /// Add a folder to this context
    pub fn add_folder(&mut self, folder: TrackedFolder) {
        self.folders.push(folder);
    }

    /// Remove a folder by path
    pub fn remove_folder(&mut self, path: &PathBuf) -> Option<TrackedFolder> {
        if let Some(pos) = self.folders.iter().position(|f| &f.path == path) {
            Some(self.folders.remove(pos))
        } else {
            None
        }
    }

    /// Get a folder by path
    pub fn get_folder(&self, path: &PathBuf) -> Option<&TrackedFolder> {
        self.folders.iter().find(|f| &f.path == path)
    }

    /// Get a mutable folder by path
    pub fn get_folder_mut(&mut self, path: &PathBuf) -> Option<&mut TrackedFolder> {
        self.folders.iter_mut().find(|f| &f.path == path)
    }

    /// Get folders by status
    pub fn folders_by_status(&self, status: FolderStatus) -> Vec<&TrackedFolder> {
        self.folders.iter().filter(|f| f.status == status).collect()
    }

    /// Get all wet (fully integrated) folders
    pub fn wet_folders(&self) -> Vec<&TrackedFolder> {
        self.folders_by_status(FolderStatus::Wet)
    }

    /// Get all folders that need attention (not wet)
    pub fn pending_folders(&self) -> Vec<&TrackedFolder> {
        self.folders.iter().filter(|f| !f.is_wet()).collect()
    }

    /// Count folders by status
    pub fn status_counts(&self) -> std::collections::HashMap<FolderStatus, usize> {
        let mut counts = std::collections::HashMap::new();
        for folder in &self.folders {
            *counts.entry(folder.status).or_insert(0) += 1;
        }
        counts
    }

    /// Total number of folders
    pub fn folder_count(&self) -> usize {
        self.folders.len()
    }

    /// Total beads across all folders
    pub fn total_beads(&self) -> usize {
        self.folders.iter().map(|f| f.bead_count).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let context = Context::new("work");
        assert_eq!(context.name, "work");
        assert!(context.folders.is_empty());
    }

    #[test]
    fn test_add_remove_folder() {
        let mut context = Context::new("work");

        let folder = TrackedFolder::new("/home/user/project");
        context.add_folder(folder);
        assert_eq!(context.folder_count(), 1);

        let path = PathBuf::from("/home/user/project");
        let removed = context.remove_folder(&path);
        assert!(removed.is_some());
        assert_eq!(context.folder_count(), 0);
    }

    #[test]
    fn test_status_counts() {
        let mut context = Context::new("work");

        context.add_folder(TrackedFolder::new("/a").with_status(FolderStatus::Dry));
        context.add_folder(TrackedFolder::new("/b").with_status(FolderStatus::Dry));
        context.add_folder(TrackedFolder::new("/c").with_status(FolderStatus::Git));
        context.add_folder(TrackedFolder::new("/d").with_status(FolderStatus::Wet));

        let counts = context.status_counts();
        assert_eq!(counts.get(&FolderStatus::Dry), Some(&2));
        assert_eq!(counts.get(&FolderStatus::Git), Some(&1));
        assert_eq!(counts.get(&FolderStatus::Wet), Some(&1));
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(Language::from_str("rust"), Language::Rust);
        assert_eq!(Language::from_str("ts"), Language::TypeScript);
        assert_eq!(Language::from_str("python"), Language::Python);
    }

    #[test]
    fn test_detected_info() {
        let mut info = DetectedInfo::default();
        assert!(!info.has_languages());
        assert!(!info.has_any_agent());

        info.languages.push(Language::Rust);
        info.has_claude = true;

        assert!(info.has_languages());
        assert!(info.has_any_agent());
        assert_eq!(info.primary_language(), Some(&Language::Rust));
    }
}
