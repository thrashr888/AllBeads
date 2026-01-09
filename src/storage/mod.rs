//! Storage layer
//!
//! Provides integration with the beads CLI for reading and writing issues.
//! Handles type conversions, JSONL parsing, and high-level operations.

mod beads_repo;
mod conversions;
mod jsonl;

pub use beads_repo::BeadsRepo;
pub use conversions::{issue_to_bead, issues_to_beads, parse_issue_type, parse_status};
pub use jsonl::{read_beads, write_beads, JsonlReader, JsonlWriter};
