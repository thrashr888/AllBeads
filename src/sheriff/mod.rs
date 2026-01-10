//! Sheriff daemon
//!
//! Background synchronization daemon with event loop.
//! Handles syncing beads between Rig repositories and the Boss repo.
//!
//! # Architecture
//!
//! The Sheriff is a long-running background process that enforces consistency
//! across the federated graph. It is the "glue" of the AllBeads architecture.
//!
//! ## Event Loop Phases
//!
//! 1. **Poll Phase**: Fetch beads updates from all Rigs
//! 2. **Diff Phase**: Compare Rig state with cached Boss state
//! 3. **Sync Phase**: Create/update Shadow Beads, push Boss directives to Rigs
//! 4. **External Sync**: Bi-directional sync with JIRA/GitHub
//! 5. **Mail Delivery**: Process pending agent mail
//!
//! ## Communication
//!
//! The TUI runs as a client connected to the Sheriff daemon via:
//! - **Event Stream**: Sheriff pushes updates (new beads, agent status changes, mail)
//! - **Command Channel**: TUI can trigger syncs or control the daemon
//! - **Shared SQLite**: WAL mode for concurrent read access to state
//!
//! # Example
//!
//! ```ignore
//! use allbeads::sheriff::{Sheriff, SheriffConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = SheriffConfig::new(".")
//!         .with_manifest("manifests/default.xml")
//!         .with_poll_interval(std::time::Duration::from_secs(10));
//!
//!     let mut sheriff = Sheriff::new(config)?;
//!     sheriff.init()?;
//!
//!     // Subscribe to events
//!     let mut events = sheriff.subscribe();
//!     tokio::spawn(async move {
//!         while let Ok(event) = events.recv().await {
//!             println!("Event: {:?}", event);
//!         }
//!     });
//!
//!     // Run the daemon
//!     sheriff.run().await?;
//!     Ok(())
//! }
//! ```

mod daemon;
mod external_sync;
mod sync;

pub use daemon::{
    Sheriff, SheriffBuilder, SheriffCommand, SheriffConfig, SheriffEvent, SheriffStats,
    DEFAULT_POLL_INTERVAL,
};
pub use external_sync::{ExternalSyncConfig, ExternalSyncEvent, ExternalSyncResult, ExternalSyncer};
pub use sync::{sync_rig_to_shadows, ShadowSync, SyncResult};
