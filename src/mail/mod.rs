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

mod message;

pub use message::{
    AgentStatus, BroadcastCategory, BroadcastPayload, HeartbeatPayload, LockRequest, Message,
    MessageId, MessageType, NotifyPayload, RequestPayload, ResponsePayload, ResponseStatus,
    Severity, UnlockRequest,
};
