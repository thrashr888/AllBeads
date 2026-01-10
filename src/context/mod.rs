//! Context and folder tracking system
//!
//! Manages tracked folders with progressive status from "Dry" to "Wet".
//! Supports batch onboarding, status detection, and distributed configuration.

mod folder;
mod status;
mod tracked;

pub use folder::{FolderConfig, TrackedFolder};
pub use status::FolderStatus;
pub use tracked::{Context, ContextDefaults, DetectedInfo};
