//! Error types for AllBeads
//!
//! Defines a comprehensive error enum covering all failure modes across the system.
//! Uses thiserror for ergonomic error handling.

use crate::mail::{AddressError, PostmasterError};
use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for AllBeads operations
pub type Result<T> = std::result::Result<T, AllBeadsError>;

/// Comprehensive error type for AllBeads operations
#[derive(Error, Debug)]
pub enum AllBeadsError {
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Git operation errors
    #[error("Git error: {0}")]
    Git(String),

    /// Storage/database errors
    #[error("Storage error: {0}")]
    Storage(String),

    /// Network/HTTP errors
    #[error("Network error: {0}")]
    Network(String),

    /// Parsing errors (JSONL, YAML, XML)
    #[error("Parse error: {0}")]
    Parse(String),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Issue not found
    #[error("Issue not found: {0}")]
    IssueNotFound(String),

    /// File lock errors (for Agent Mail)
    #[error("File lock conflict: {path} locked by {holder} until {expires_at}")]
    LockConflict {
        path: PathBuf,
        holder: String,
        expires_at: String,
    },

    /// Authentication errors
    #[error("Authentication error: {0}")]
    Auth(String),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// YAML parsing errors
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// Git2 library errors
    #[error("Git library error: {0}")]
    Git2(#[from] git2::Error),

    /// SQLite database errors
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// HTTP request errors
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Postmaster (Agent Mail) errors
    #[error("Mail error: {0}")]
    Mail(#[from] PostmasterError),

    /// Address parsing errors
    #[error("Address error: {0}")]
    Address(#[from] AddressError),

    /// Integration errors (JIRA, GitHub, plugins)
    #[error("Integration error: {0}")]
    Integration(String),

    /// Other errors
    #[error("{0}")]
    Other(String),
}
