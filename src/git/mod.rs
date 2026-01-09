//! Git operations for Boss repositories
//!
//! Handles cloning, fetching, and reading .beads/ directories from remote
//! Boss repositories with authentication support.

mod operations;

pub use operations::{BossRepo, GitCredentials, RepoStatus};
