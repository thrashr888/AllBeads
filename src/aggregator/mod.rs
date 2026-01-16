//! Multi-Boss repository aggregator
//!
//! Aggregates .beads/ directories from multiple Boss repositories into a
//! unified FederatedGraph, respecting context boundaries.

mod boss_aggregator;

pub use boss_aggregator::{Aggregator, AggregatorConfig, RefreshProgress, RefreshResult, SyncMode};
