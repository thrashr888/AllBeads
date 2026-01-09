//! Storage layer
//!
//! Provides integration with the beads CLI for reading and writing issues.
//! Handles type conversions and high-level operations.

mod beads_repo;
mod conversions;

pub use beads_repo::BeadsRepo;
pub use conversions::{issue_to_bead, issues_to_beads, parse_issue_type, parse_status};
