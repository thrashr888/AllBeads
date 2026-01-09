//! AllBeads - Distributed Protocol for Agentic Orchestration and Communication
//!
//! AllBeads is an open-source CLI and TUI product designed to be the "Inter-Process
//! Communication" (IPC) layer for AI agent workflows. It aggregates multiple independent
//! "Boss" repositories representing distinct contexts (work/personal) into a unified
//! dashboard without merging their data.
//!
//! # Architecture
//!
//! - **graph**: Core data structures (Bead, ShadowBead, Rig, FederatedGraph)
//! - **config**: Multi-context configuration and authentication
//! - **storage**: Data persistence (SQLite, JSONL)
//! - **sheriff**: Background daemon for sync and coordination
//! - **boss_board**: Terminal UI (ratatui-based)
//! - **manifest**: XML manifest parsing
//! - **integrations**: External system adapters (JIRA, GitHub)
//! - **mail**: Agent Mail protocol (Postmaster, file locking)

// Core modules
pub mod config;
pub mod error;
pub mod graph;
pub mod storage;

// Components (will be implemented in phases)
pub mod boss_board;
pub mod integrations;
pub mod mail;
pub mod manifest;
pub mod sheriff;

// Re-exports
pub use error::{AllBeadsError, Result};
