//! Sheriff daemon
//!
//! Background synchronization daemon with event loop.
//! Handles syncing beads between Rig repositories and the Boss repo.

mod sync;

pub use sync::{sync_rig_to_shadows, ShadowSync, SyncResult};
