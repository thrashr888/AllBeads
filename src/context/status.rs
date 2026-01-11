//! Folder status progression
//!
//! Represents the "Dry" to "Wet" progression of a tracked folder.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Status levels for tracked folders
///
/// Progression from Dry (uninitialized) to Wet (fully integrated):
/// - Dry: Folder exists, no git or beads
/// - Git: Git repository initialized
/// - Beads: Beads initialized (.beads/ exists)
/// - Configured: AllBeads config applied (prefix, persona, etc.)
/// - Wet: Fully integrated (syncing, hooks active)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FolderStatus {
    /// No git repository
    Dry,
    /// Git initialized but no beads
    Git,
    /// Beads initialized (.beads/ exists)
    Beads,
    /// AllBeads config applied
    Configured,
    /// Fully integrated with sync and hooks
    Wet,
}

impl FolderStatus {
    /// Get numeric level (0-4) for comparison
    pub fn level(&self) -> u8 {
        match self {
            Self::Dry => 0,
            Self::Git => 1,
            Self::Beads => 2,
            Self::Configured => 3,
            Self::Wet => 4,
        }
    }

    /// Get status from numeric level
    pub fn from_level(level: u8) -> Option<Self> {
        match level {
            0 => Some(Self::Dry),
            1 => Some(Self::Git),
            2 => Some(Self::Beads),
            3 => Some(Self::Configured),
            4 => Some(Self::Wet),
            _ => None,
        }
    }

    /// Check if this status meets or exceeds a required level
    pub fn meets(&self, required: Self) -> bool {
        self.level() >= required.level()
    }

    /// Get the next status level in progression
    pub fn next(&self) -> Option<Self> {
        Self::from_level(self.level() + 1)
    }

    /// Get the previous status level
    pub fn prev(&self) -> Option<Self> {
        if self.level() == 0 {
            None
        } else {
            Self::from_level(self.level() - 1)
        }
    }

    /// Get icon for status (minimal emoji use)
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Dry => "○",        // Empty circle
            Self::Git => "◔",        // Quarter filled
            Self::Beads => "◑",      // Half filled
            Self::Configured => "◕", // Three-quarter filled
            Self::Wet => "●",        // Full circle
        }
    }

    /// Get short display name
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Dry => "dry",
            Self::Git => "git",
            Self::Beads => "beads",
            Self::Configured => "config",
            Self::Wet => "wet",
        }
    }

    /// Get full display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Dry => "Dry (no git)",
            Self::Git => "Git initialized",
            Self::Beads => "Beads initialized",
            Self::Configured => "Configured",
            Self::Wet => "Fully integrated",
        }
    }

    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dry" => Some(Self::Dry),
            "git" => Some(Self::Git),
            "beads" => Some(Self::Beads),
            "configured" | "config" => Some(Self::Configured),
            "wet" => Some(Self::Wet),
            _ => None,
        }
    }
}

impl Default for FolderStatus {
    fn default() -> Self {
        Self::Dry
    }
}

impl fmt::Display for FolderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.icon(), self.short_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_progression() {
        assert_eq!(FolderStatus::Dry.level(), 0);
        assert_eq!(FolderStatus::Wet.level(), 4);

        assert_eq!(FolderStatus::Dry.next(), Some(FolderStatus::Git));
        assert_eq!(FolderStatus::Wet.next(), None);

        assert_eq!(FolderStatus::Wet.prev(), Some(FolderStatus::Configured));
        assert_eq!(FolderStatus::Dry.prev(), None);
    }

    #[test]
    fn test_status_meets() {
        assert!(FolderStatus::Wet.meets(FolderStatus::Dry));
        assert!(FolderStatus::Git.meets(FolderStatus::Git));
        assert!(!FolderStatus::Dry.meets(FolderStatus::Git));
    }

    #[test]
    fn test_status_ordering() {
        assert!(FolderStatus::Dry < FolderStatus::Git);
        assert!(FolderStatus::Git < FolderStatus::Beads);
        assert!(FolderStatus::Beads < FolderStatus::Configured);
        assert!(FolderStatus::Configured < FolderStatus::Wet);
    }

    #[test]
    fn test_from_str() {
        assert_eq!(FolderStatus::from_str("dry"), Some(FolderStatus::Dry));
        assert_eq!(FolderStatus::from_str("WET"), Some(FolderStatus::Wet));
        assert_eq!(
            FolderStatus::from_str("config"),
            Some(FolderStatus::Configured)
        );
        assert_eq!(
            FolderStatus::from_str("configured"),
            Some(FolderStatus::Configured)
        );
        assert_eq!(FolderStatus::from_str("unknown"), None);
    }

    #[test]
    fn test_serialization() {
        let status = FolderStatus::Configured;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"configured\"");

        let parsed: FolderStatus = serde_json::from_str("\"beads\"").unwrap();
        assert_eq!(parsed, FolderStatus::Beads);
    }
}
