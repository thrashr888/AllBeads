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

    /// Swarm/Agent management errors
    #[error("Swarm error: {0}")]
    Swarm(String),

    /// Other errors
    #[error("{0}")]
    Other(String),

    /// Anyhow errors (for more context)
    #[error("{0}")]
    Anyhow(#[from] anyhow::Error),

    /// Rate limited (with optional retry-after duration in seconds)
    #[error("Rate limited, retry after {0} seconds")]
    RateLimited(u64),
}

impl crate::integrations::retry::RetryableError for AllBeadsError {
    fn retry_decision(&self) -> crate::integrations::retry::RetryDecision {
        use crate::integrations::retry::RetryDecision;
        use std::time::Duration;

        match self {
            // Retryable errors
            AllBeadsError::Network(_) => RetryDecision::Retry,
            AllBeadsError::Http(e) => {
                // Check if it's a connection or timeout error
                if e.is_connect() || e.is_timeout() {
                    RetryDecision::Retry
                } else if e.is_status() {
                    // Check status code
                    if let Some(status) = e.status() {
                        match status.as_u16() {
                            429 => RetryDecision::RetryAfter(Duration::from_secs(60)),
                            500..=599 => RetryDecision::Retry,
                            _ => RetryDecision::NoRetry,
                        }
                    } else {
                        RetryDecision::NoRetry
                    }
                } else {
                    RetryDecision::Retry // Default to retry for other HTTP errors
                }
            }
            AllBeadsError::RateLimited(secs) => {
                RetryDecision::RetryAfter(Duration::from_secs(*secs))
            }
            AllBeadsError::Integration(msg) => {
                // Check for rate limit messages
                if msg.contains("Rate limited") || msg.contains("rate limit") {
                    // Try to extract retry-after from message
                    if let Some(secs) = extract_retry_after(msg) {
                        RetryDecision::RetryAfter(Duration::from_secs(secs))
                    } else {
                        RetryDecision::RetryAfter(Duration::from_secs(60))
                    }
                } else if msg.contains("timeout") || msg.contains("connection") {
                    RetryDecision::Retry
                } else {
                    RetryDecision::NoRetry
                }
            }
            // Non-retryable errors
            AllBeadsError::Config(_) => RetryDecision::NoRetry,
            AllBeadsError::Git(_) => RetryDecision::NoRetry,
            AllBeadsError::Storage(_) => RetryDecision::NoRetry,
            AllBeadsError::Parse(_) => RetryDecision::NoRetry,
            AllBeadsError::Io(_) => RetryDecision::NoRetry,
            AllBeadsError::IssueNotFound(_) => RetryDecision::NoRetry,
            AllBeadsError::LockConflict { .. } => RetryDecision::NoRetry,
            AllBeadsError::Auth(_) => RetryDecision::NoRetry,
            AllBeadsError::Json(_) => RetryDecision::NoRetry,
            AllBeadsError::Yaml(_) => RetryDecision::NoRetry,
            AllBeadsError::Git2(_) => RetryDecision::NoRetry,
            AllBeadsError::Database(_) => RetryDecision::NoRetry,
            AllBeadsError::Mail(_) => RetryDecision::NoRetry,
            AllBeadsError::Address(_) => RetryDecision::NoRetry,
            AllBeadsError::Swarm(_) => RetryDecision::NoRetry,
            AllBeadsError::Other(_) => RetryDecision::NoRetry,
            AllBeadsError::Anyhow(_) => RetryDecision::NoRetry,
        }
    }
}

/// Extract retry-after seconds from an error message
fn extract_retry_after(msg: &str) -> Option<u64> {
    // Look for patterns like "retry after 60 seconds" or "retry after 60"
    let msg_lower = msg.to_lowercase();
    if let Some(pos) = msg_lower.find("retry after") {
        let after_text = &msg[pos + 11..];
        // Find the first number
        let num_str: String = after_text
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        num_str.parse().ok()
    } else {
        None
    }
}
