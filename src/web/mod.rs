//! Web API client for AllBeads web platform
//!
//! Provides authenticated access to allbeads.co API endpoints:
//! - /api/beads/stats - Aggregated statistics
//! - /api/beads/[id]/comments - Bead comments
//! - /api/beads/import - Bulk bead import

mod client;

pub use client::*;
