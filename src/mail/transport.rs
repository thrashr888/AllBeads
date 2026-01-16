//! Mail transport abstraction
//!
//! Defines the trait for pluggable message storage backends.

use super::{Address, DeliveryStatus, Message, MessageId, StoredMessage};
use thiserror::Error;

/// Transport errors
#[derive(Debug, Error)]
pub enum TransportError {
    #[error("storage error: {0}")]
    Storage(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("message not found: {0}")]
    NotFound(String),
}

/// Result type for transport operations
pub type Result<T> = std::result::Result<T, TransportError>;

/// Trait for mail storage backends
pub trait MailTransport: Send + Sync {
    /// Store a message
    fn store(&self, message: &Message, status: DeliveryStatus) -> Result<()>;

    /// Get messages for a recipient (inbox)
    fn inbox(&self, address: &Address) -> Result<Vec<StoredMessage>>;

    /// Get messages with a specific status
    fn inbox_with_status(&self, address: &Address, status: DeliveryStatus)
        -> Result<Vec<StoredMessage>>;

    /// Get messages sent by an address (outbox)
    fn outbox(&self, address: &Address) -> Result<Vec<StoredMessage>>;

    /// Get a specific message by ID
    fn get(&self, message_id: &MessageId) -> Result<Option<StoredMessage>>;

    /// Mark a message as read
    fn mark_read(&self, message_id: &MessageId) -> Result<()>;

    /// Count messages in inbox
    fn inbox_count(&self, address: &Address) -> Result<usize>;

    /// Count unread messages
    fn unread_count(&self, address: &Address) -> Result<usize>;
}
