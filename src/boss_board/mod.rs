//! Boss Board TUI
//!
//! Terminal-based user interface with ratatui.
//!
//! The Boss Board is implemented in the [`tui`](crate::tui) module,
//! providing Kanban and Mail views for managing beads across contexts.
//!
//! See [`crate::tui::run`] to launch the TUI.

// Re-export from tui module for convenience
pub use crate::tui::{run, run_with_mail, App, Tab};
