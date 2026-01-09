//! Cache layer for aggregated graph data
//!
//! Provides SQLite-based caching of the FederatedGraph with expiration
//! and refresh capabilities.

mod sqlite;

pub use sqlite::{Cache, CacheConfig};
