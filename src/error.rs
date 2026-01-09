//! Error types for AllBeads
//!
//! Defines a comprehensive error enum covering all failure modes across the system.

use std::path::PathBuf;

/// Result type alias for AllBeads operations
pub type Result<T> = std::result::Result<T, AllBeadsError>;

/// Comprehensive error type for AllBeads operations
#[derive(Debug)]
pub enum AllBeadsError {
    /// Configuration errors
    Config(String),

    /// Git operation errors
    Git(String),

    /// Storage/database errors
    Storage(String),

    /// Network/HTTP errors
    Network(String),

    /// Parsing errors (JSONL, YAML, XML)
    Parse(String),

    /// I/O errors
    Io(std::io::Error),

    /// Issue not found
    IssueNotFound(String),

    /// File lock errors
    LockConflict {
        path: PathBuf,
        holder: String,
        expires_at: String,
    },

    /// Authentication errors
    Auth(String),

    /// Other errors
    Other(String),
}

impl std::fmt::Display for AllBeadsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(msg) => write!(f, "Configuration error: {}", msg),
            Self::Git(msg) => write!(f, "Git error: {}", msg),
            Self::Storage(msg) => write!(f, "Storage error: {}", msg),
            Self::Network(msg) => write!(f, "Network error: {}", msg),
            Self::Parse(msg) => write!(f, "Parse error: {}", msg),
            Self::Io(err) => write!(f, "I/O error: {}", err),
            Self::IssueNotFound(id) => write!(f, "Issue not found: {}", id),
            Self::LockConflict { path, holder, expires_at } => {
                write!(
                    f,
                    "File lock conflict: {} locked by {} until {}",
                    path.display(),
                    holder,
                    expires_at
                )
            }
            Self::Auth(msg) => write!(f, "Authentication error: {}", msg),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for AllBeadsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for AllBeadsError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

// Placeholder for future conversions from library errors
// impl From<serde_json::Error> for AllBeadsError { ... }
// impl From<serde_yaml::Error> for AllBeadsError { ... }
// impl From<git2::Error> for AllBeadsError { ... }
