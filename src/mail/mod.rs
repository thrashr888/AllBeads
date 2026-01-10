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
mod message;

pub use address::{Address, AddressError, RoutingTarget};
pub use message::{
    AgentStatus, BroadcastCategory, BroadcastPayload, HeartbeatPayload, LockRequest, Message,
    MessageId, MessageType, NotifyPayload, RequestPayload, ResponsePayload, ResponseStatus,
    Severity, UnlockRequest,
};
