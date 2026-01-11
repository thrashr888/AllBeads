//! Folder configuration and tracking
//!
//! Defines TrackedFolder for individual folders and FolderConfig for their settings.

use super::status::FolderStatus;
use super::tracked::DetectedInfo;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Beads installation mode for a folder
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "mode")]
pub enum BeadsMode {
    /// SQLite + JSONL (default)
    #[default]
    Standard,
    /// JSONL only, no SQLite
    JsonlOnly,
    /// Dedicated sync branch
    SyncBranch {
        /// Branch name for beads sync
        branch: String,
    },
    /// Background daemon sync
    Daemon {
        /// Sync interval in seconds
        #[serde(with = "serde_duration")]
        interval: Duration,
    },
}

/// Configuration for a tracked folder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderConfig {
    /// Issue prefix for this folder
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,

    /// Agent persona for this folder
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,

    /// Beads installation mode
    #[serde(default)]
    pub beads_mode: BeadsMode,

    /// Whether sync is enabled
    #[serde(default)]
    pub sync_enabled: bool,

    /// Sync interval
    #[serde(default = "default_sync_interval", with = "serde_duration")]
    pub sync_interval: Duration,

    /// Labels to apply to all beads from this folder
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,

    /// Custom CLAUDE.md path (if not default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_md_path: Option<PathBuf>,
}

fn default_sync_interval() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

impl Default for FolderConfig {
    fn default() -> Self {
        Self {
            prefix: None,
            persona: None,
            beads_mode: BeadsMode::Standard,
            sync_enabled: false,
            sync_interval: default_sync_interval(),
            labels: Vec::new(),
            claude_md_path: None,
        }
    }
}

impl FolderConfig {
    /// Create a new folder config with prefix
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            prefix: Some(prefix.into()),
            ..Default::default()
        }
    }

    /// Enable sync with default interval
    pub fn with_sync(mut self) -> Self {
        self.sync_enabled = true;
        self
    }

    /// Set persona
    pub fn with_persona(mut self, persona: impl Into<String>) -> Self {
        self.persona = Some(persona.into());
        self
    }

    /// Set beads mode
    pub fn with_beads_mode(mut self, mode: BeadsMode) -> Self {
        self.beads_mode = mode;
        self
    }

    /// Add a label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }
}

/// A tracked folder with status and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedFolder {
    /// Absolute path to the folder
    pub path: PathBuf,

    /// Current status (dry to wet)
    pub status: FolderStatus,

    /// Folder configuration (if configured)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<FolderConfig>,

    /// Detected project information
    #[serde(default)]
    pub detected: DetectedInfo,

    /// When this folder was added
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_at: Option<String>,

    /// Last time status was checked
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checked: Option<String>,

    /// Number of beads in this folder (cached)
    #[serde(default)]
    pub bead_count: usize,

    /// Whether there are pending sync operations
    #[serde(default)]
    pub pending_sync: bool,
}

impl TrackedFolder {
    /// Create a new tracked folder at the given path
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            status: FolderStatus::Dry,
            config: None,
            detected: DetectedInfo::default(),
            added_at: Some(chrono::Utc::now().to_rfc3339()),
            last_checked: None,
            bead_count: 0,
            pending_sync: false,
        }
    }

    /// Set the status
    pub fn with_status(mut self, status: FolderStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the configuration
    pub fn with_config(mut self, config: FolderConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set detected info
    pub fn with_detected(mut self, detected: DetectedInfo) -> Self {
        self.detected = detected;
        self
    }

    /// Get the folder name (last path component)
    pub fn name(&self) -> &str {
        self.path.file_name().and_then(|n| n.to_str()).unwrap_or("")
    }

    /// Get the display path (with ~ substitution for home)
    pub fn display_path(&self) -> String {
        if let Some(home) = dirs::home_dir() {
            if self.path.starts_with(&home) {
                return format!("~{}", self.path.strip_prefix(&home).unwrap().display());
            }
        }
        self.path.display().to_string()
    }

    /// Check if this folder can be promoted to the next status
    pub fn can_promote(&self) -> bool {
        self.status.next().is_some()
    }

    /// Check if this folder is fully configured
    pub fn is_wet(&self) -> bool {
        self.status == FolderStatus::Wet
    }

    /// Check if this folder has beads
    pub fn has_beads(&self) -> bool {
        self.status.meets(FolderStatus::Beads)
    }

    /// Get the prefix, if configured
    pub fn prefix(&self) -> Option<&str> {
        self.config.as_ref().and_then(|c| c.prefix.as_deref())
    }
}

/// Serde helper for Duration
mod serde_duration {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_folder_creation() {
        let folder = TrackedFolder::new("/home/user/project");
        assert_eq!(folder.status, FolderStatus::Dry);
        assert!(folder.config.is_none());
        assert!(folder.added_at.is_some());
    }

    #[test]
    fn test_folder_builder() {
        let folder = TrackedFolder::new("/home/user/project")
            .with_status(FolderStatus::Beads)
            .with_config(FolderConfig::with_prefix("proj").with_sync());

        assert_eq!(folder.status, FolderStatus::Beads);
        assert!(folder.config.is_some());
        assert!(folder.config.as_ref().unwrap().sync_enabled);
    }

    #[test]
    fn test_folder_name() {
        let folder = TrackedFolder::new("/home/user/my-project");
        assert_eq!(folder.name(), "my-project");
    }

    #[test]
    fn test_beads_mode_serialization() {
        let mode = BeadsMode::SyncBranch {
            branch: "beads-sync".to_string(),
        };
        let json = serde_json::to_string(&mode).unwrap();
        assert!(json.contains("sync_branch"));
        assert!(json.contains("beads-sync"));
    }
}
