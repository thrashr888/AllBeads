//! Agent Mail protocol
//!
//! Message routing, file locking, and Postmaster implementation.
//!
//! # Overview
//!
//! Agent Mail provides the signaling layer for agent coordination:
//! - **File locking** to prevent concurrent modifications
//! - **Message routing** between agents and human operators
//! - **Heartbeats** for agent liveness monitoring
//!
//! # Message Types
//!
//! - `LOCK` - Request exclusive file access
//! - `UNLOCK` - Release file lock
//! - `NOTIFY` - Inform about state changes
//! - `REQUEST` - Ask for human input
//! - `BROADCAST` - Announce to all agents
//! - `HEARTBEAT` - Agent liveness signal
//!
//! # Addressing
//!
//! Messages use email-like addressing:
//! - `agent_name@project_id` - Specific agent
//! - `human@localhost` - Human operator inbox
//! - `all@project_id` - Broadcast to all agents
//! - `postmaster@project_id` - The postmaster service
//!
//! # Transports
//!
//! Agent Mail supports pluggable storage backends:
//! - **SQLite** (default) - Fast local storage via Postmaster
//! - **Filesystem** - Git-trackable JSON files in `.beads/mail/`
//!
//! # Examples
//!
//! ```
//! use allbeads::mail::{Address, Message, MessageType, NotifyPayload};
//!
//! // Create addresses
//! let from: Address = "worker@my-project".parse().unwrap();
//! let to = Address::human();
//!
//! // Create a notification message
//! let msg = Message::new(
//!     from,
//!     to,
//!     MessageType::Notify(NotifyPayload::new("Task completed")),
//! );
//! ```

mod address;
mod filesystem;
mod locks;
mod message;
mod postmaster;
mod server;
mod transport;

pub use address::{Address, AddressError, RoutingTarget};
pub use filesystem::FilesystemTransport;
pub use locks::{ConflictStrategy, LockInfo, LockManager, LockResult};
pub use message::{
    AgentStatus, BroadcastCategory, BroadcastPayload, HeartbeatPayload, LockRequest, Message,
    MessageId, MessageType, NotifyPayload, RequestPayload, ResponsePayload, ResponseStatus,
    Severity, UnlockRequest,
};
pub use postmaster::{DeliveryStatus, Postmaster, PostmasterError, SendResult, StoredMessage};
pub use server::{MailServer, ServerError};
pub use transport::{MailTransport, TransportError};
