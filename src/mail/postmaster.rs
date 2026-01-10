//! Postmaster - Agent Mail message router and broker
//!
//! The Postmaster is the central message routing service for Agent Mail.
//! It handles message delivery, persistence, and inbox management.
//!
//! # Responsibilities
//!
//! - Route messages to appropriate recipients
//! - Store messages in SQLite for persistence
//! - Manage per-agent inboxes
//! - Handle LOCK/UNLOCK requests via LockManager
//! - Broadcast messages to all agents in a project
//!
//! # Example
//!
//! ```no_run
//! use allbeads::mail::{Postmaster, Message, MessageType, NotifyPayload, Address};
//! use std::path::PathBuf;
//!
//! let mut postmaster = Postmaster::new(PathBuf::from("mail.db")).unwrap();
//!
//! // Send a message
//! let msg = Message::from_strings(
//!     "worker@project",
//!     "human@localhost",
//!     MessageType::Notify(NotifyPayload::new("Task completed")),
//! );
//! postmaster.send(msg).unwrap();
//!
//! // Check inbox
//! let human = Address::human();
//! let messages = postmaster.inbox(&human).unwrap();
//! ```

use super::{
    Address, ConflictStrategy, LockManager, LockRequest, LockResult, Message, MessageId,
    MessageType, ResponsePayload, ResponseStatus, RoutingTarget, UnlockRequest,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::PathBuf;
use thiserror::Error;

/// Postmaster errors
#[derive(Debug, Error)]
pub enum PostmasterError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("invalid address: {0}")]
    InvalidAddress(#[from] super::AddressError),

    #[error("message not found: {0}")]
    MessageNotFound(String),
}

/// Result type for Postmaster operations
pub type Result<T> = std::result::Result<T, PostmasterError>;

/// Delivery status for a message
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryStatus {
    /// Message is queued for delivery
    Pending,
    /// Message has been delivered to recipient's inbox
    Delivered,
    /// Message has been read by recipient
    Read,
    /// Message delivery failed
    Failed,
}

impl DeliveryStatus {
    fn as_str(&self) -> &'static str {
        match self {
            DeliveryStatus::Pending => "pending",
            DeliveryStatus::Delivered => "delivered",
            DeliveryStatus::Read => "read",
            DeliveryStatus::Failed => "failed",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "pending" => DeliveryStatus::Pending,
            "delivered" => DeliveryStatus::Delivered,
            "read" => DeliveryStatus::Read,
            "failed" => DeliveryStatus::Failed,
            _ => DeliveryStatus::Pending,
        }
    }
}

/// A stored message with metadata
#[derive(Debug, Clone)]
pub struct StoredMessage {
    /// The message itself
    pub message: Message,
    /// Delivery status
    pub status: DeliveryStatus,
    /// When the message was stored
    pub stored_at: DateTime<Utc>,
    /// When the message was delivered (if applicable)
    pub delivered_at: Option<DateTime<Utc>>,
    /// When the message was read (if applicable)
    pub read_at: Option<DateTime<Utc>>,
}

/// The Postmaster message broker
pub struct Postmaster {
    /// SQLite connection for message persistence
    conn: Connection,
    /// Lock manager for file locking
    lock_manager: LockManager,
    /// Project ID for this postmaster instance
    project_id: String,
}

impl Postmaster {
    /// Create a new Postmaster with SQLite storage
    pub fn new(db_path: PathBuf) -> Result<Self> {
        Self::with_project_id(db_path, "default")
    }

    /// Create a new Postmaster with a specific project ID
    pub fn with_project_id(db_path: PathBuf, project_id: impl Into<String>) -> Result<Self> {
        let conn = Connection::open(&db_path)?;

        let postmaster = Self {
            conn,
            lock_manager: LockManager::new(),
            project_id: project_id.into(),
        };

        postmaster.init_schema()?;
        Ok(postmaster)
    }

    /// Initialize the database schema
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                from_addr TEXT NOT NULL,
                to_addr TEXT NOT NULL,
                message_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                correlation_id TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                stored_at TEXT NOT NULL,
                delivered_at TEXT,
                read_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_messages_to ON messages(to_addr);
            CREATE INDEX IF NOT EXISTS idx_messages_from ON messages(from_addr);
            CREATE INDEX IF NOT EXISTS idx_messages_status ON messages(status);
            CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);

            CREATE TABLE IF NOT EXISTS agents (
                address TEXT PRIMARY KEY,
                last_seen TEXT,
                status TEXT
            );
            "#,
        )?;
        Ok(())
    }

    /// Send a message
    ///
    /// Routes the message to the appropriate recipient(s) and stores it.
    pub fn send(&mut self, message: Message) -> Result<SendResult> {
        // Handle special message types
        match &message.message_type {
            MessageType::Lock(lock_req) => {
                return self.handle_lock_request(&message, lock_req);
            }
            MessageType::Unlock(unlock_req) => {
                return self.handle_unlock_request(&message, unlock_req);
            }
            _ => {}
        }

        // Route based on recipient
        let target = RoutingTarget::from_address(&message.to);

        match target {
            RoutingTarget::Broadcast { project_id } => {
                self.handle_broadcast(&message, &project_id)
            }
            _ => {
                // Store message for recipient
                self.store_message(&message, DeliveryStatus::Delivered)?;
                Ok(SendResult::Delivered {
                    message_id: message.id.clone(),
                })
            }
        }
    }

    /// Handle a LOCK request
    fn handle_lock_request(
        &mut self,
        message: &Message,
        lock_req: &LockRequest,
    ) -> Result<SendResult> {
        let result = self.lock_manager.acquire_with_reason(
            &lock_req.path,
            message.from.clone(),
            lock_req.ttl,
            ConflictStrategy::Abort,
            lock_req.reason.clone(),
        );

        // Create response message
        let response = match &result {
            LockResult::Acquired { expires_at } => Message::new(
                Address::postmaster(&self.project_id),
                message.from.clone(),
                MessageType::Response(ResponsePayload {
                    in_reply_to: message.id.clone(),
                    status: ResponseStatus::Success,
                    message: Some(format!("Lock acquired until {}", expires_at)),
                    data: None,
                }),
            ),
            LockResult::Denied {
                holder,
                expires_at,
                reason,
            } => Message::new(
                Address::postmaster(&self.project_id),
                message.from.clone(),
                MessageType::Response(ResponsePayload {
                    in_reply_to: message.id.clone(),
                    status: ResponseStatus::Denied,
                    message: Some(format!(
                        "Lock held by {} until {}{}",
                        holder,
                        expires_at,
                        reason
                            .as_ref()
                            .map(|r| format!(" ({})", r))
                            .unwrap_or_default()
                    )),
                    data: None,
                }),
            ),
            _ => Message::new(
                Address::postmaster(&self.project_id),
                message.from.clone(),
                MessageType::Response(ResponsePayload {
                    in_reply_to: message.id.clone(),
                    status: ResponseStatus::Error,
                    message: Some("Unexpected lock result".to_string()),
                    data: None,
                }),
            ),
        };

        // Store original message
        self.store_message(message, DeliveryStatus::Delivered)?;
        // Store response
        self.store_message(&response, DeliveryStatus::Delivered)?;

        Ok(SendResult::LockResult {
            message_id: message.id.clone(),
            result,
        })
    }

    /// Handle an UNLOCK request
    fn handle_unlock_request(
        &mut self,
        message: &Message,
        unlock_req: &UnlockRequest,
    ) -> Result<SendResult> {
        let result = self.lock_manager.release(&unlock_req.path, &message.from);

        // Create response message
        let response = match &result {
            LockResult::Released => Message::new(
                Address::postmaster(&self.project_id),
                message.from.clone(),
                MessageType::Response(ResponsePayload {
                    in_reply_to: message.id.clone(),
                    status: ResponseStatus::Success,
                    message: Some("Lock released".to_string()),
                    data: None,
                }),
            ),
            LockResult::NotLocked => Message::new(
                Address::postmaster(&self.project_id),
                message.from.clone(),
                MessageType::Response(ResponsePayload {
                    in_reply_to: message.id.clone(),
                    status: ResponseStatus::Error,
                    message: Some("File was not locked".to_string()),
                    data: None,
                }),
            ),
            LockResult::Denied { holder, .. } => Message::new(
                Address::postmaster(&self.project_id),
                message.from.clone(),
                MessageType::Response(ResponsePayload {
                    in_reply_to: message.id.clone(),
                    status: ResponseStatus::Denied,
                    message: Some(format!("Lock held by {}", holder)),
                    data: None,
                }),
            ),
            _ => Message::new(
                Address::postmaster(&self.project_id),
                message.from.clone(),
                MessageType::Response(ResponsePayload {
                    in_reply_to: message.id.clone(),
                    status: ResponseStatus::Error,
                    message: Some("Unexpected unlock result".to_string()),
                    data: None,
                }),
            ),
        };

        // Store messages
        self.store_message(message, DeliveryStatus::Delivered)?;
        self.store_message(&response, DeliveryStatus::Delivered)?;

        Ok(SendResult::LockResult {
            message_id: message.id.clone(),
            result,
        })
    }

    /// Handle a broadcast message
    fn handle_broadcast(&mut self, message: &Message, _project_id: &str) -> Result<SendResult> {
        // Store the broadcast message
        self.store_message(message, DeliveryStatus::Delivered)?;

        // In a full implementation, we would:
        // 1. Get list of all agents in the project
        // 2. Create a copy for each agent's inbox
        // For now, just store the original

        Ok(SendResult::Broadcast {
            message_id: message.id.clone(),
            recipient_count: 1, // Placeholder
        })
    }

    /// Store a message in the database
    fn store_message(&self, message: &Message, status: DeliveryStatus) -> Result<()> {
        let now = Utc::now().to_rfc3339();

        // Serialize message type to get type name and payload
        let (msg_type, payload) = match &message.message_type {
            MessageType::Lock(p) => ("Lock", serde_json::to_string(p)?),
            MessageType::Unlock(p) => ("Unlock", serde_json::to_string(p)?),
            MessageType::Notify(p) => ("Notify", serde_json::to_string(p)?),
            MessageType::Request(p) => ("Request", serde_json::to_string(p)?),
            MessageType::Broadcast(p) => ("Broadcast", serde_json::to_string(p)?),
            MessageType::Heartbeat(p) => ("Heartbeat", serde_json::to_string(p)?),
            MessageType::Response(p) => ("Response", serde_json::to_string(p)?),
        };

        let delivered_at = if status == DeliveryStatus::Delivered {
            Some(now.clone())
        } else {
            None
        };

        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO messages
            (id, from_addr, to_addr, message_type, payload, timestamp, correlation_id, status, stored_at, delivered_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                message.id.as_str(),
                message.from.to_string(),
                message.to.to_string(),
                msg_type,
                payload,
                message.timestamp.to_rfc3339(),
                message.correlation_id.as_ref().map(|id| id.as_str()),
                status.as_str(),
                now,
                delivered_at,
            ],
        )?;

        Ok(())
    }

    /// Get messages in an agent's inbox
    pub fn inbox(&self, address: &Address) -> Result<Vec<StoredMessage>> {
        self.inbox_with_filter(address, None)
    }

    /// Get unread messages in an agent's inbox
    pub fn unread(&self, address: &Address) -> Result<Vec<StoredMessage>> {
        self.inbox_with_filter(address, Some(DeliveryStatus::Delivered))
    }

    /// Get messages with optional status filter
    fn inbox_with_filter(
        &self,
        address: &Address,
        status_filter: Option<DeliveryStatus>,
    ) -> Result<Vec<StoredMessage>> {
        let addr_str = address.to_string();
        let mut messages = Vec::new();

        if let Some(status) = status_filter {
            let mut stmt = self.conn.prepare(
                r#"
                SELECT id, from_addr, to_addr, message_type, payload, timestamp,
                       correlation_id, status, stored_at, delivered_at, read_at
                FROM messages
                WHERE to_addr = ?1 AND status = ?2
                ORDER BY timestamp DESC
                "#,
            )?;
            let rows = stmt.query_map(params![addr_str, status.as_str()], |row| {
                self.row_to_stored_message(row)
            })?;
            for row in rows {
                messages.push(row?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                r#"
                SELECT id, from_addr, to_addr, message_type, payload, timestamp,
                       correlation_id, status, stored_at, delivered_at, read_at
                FROM messages
                WHERE to_addr = ?1
                ORDER BY timestamp DESC
                "#,
            )?;
            let rows =
                stmt.query_map(params![addr_str], |row| self.row_to_stored_message(row))?;
            for row in rows {
                messages.push(row?);
            }
        }

        Ok(messages)
    }

    /// Convert a database row to StoredMessage
    fn row_to_stored_message(&self, row: &rusqlite::Row) -> SqliteResult<StoredMessage> {
        let id: String = row.get(0)?;
        let from_str: String = row.get(1)?;
        let to_str: String = row.get(2)?;
        let msg_type: String = row.get(3)?;
        let payload: String = row.get(4)?;
        let timestamp_str: String = row.get(5)?;
        let correlation_id: Option<String> = row.get(6)?;
        let status_str: String = row.get(7)?;
        let stored_at_str: String = row.get(8)?;
        let delivered_at_str: Option<String> = row.get(9)?;
        let read_at_str: Option<String> = row.get(10)?;

        // Parse addresses
        let from: Address = from_str.parse().unwrap_or_else(|_| Address::human());
        let to: Address = to_str.parse().unwrap_or_else(|_| Address::human());

        // Parse message type
        let message_type = self
            .parse_message_type(&msg_type, &payload)
            .unwrap_or_else(|_| {
                MessageType::Notify(super::NotifyPayload::new("Parse error"))
            });

        // Parse timestamps
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let stored_at = DateTime::parse_from_rfc3339(&stored_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let delivered_at = delivered_at_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        });
        let read_at = read_at_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        });

        let message = Message {
            id: MessageId::from_string(id),
            from,
            to,
            message_type,
            timestamp,
            correlation_id: correlation_id.map(MessageId::from_string),
        };

        Ok(StoredMessage {
            message,
            status: DeliveryStatus::from_str(&status_str),
            stored_at,
            delivered_at,
            read_at,
        })
    }

    /// Parse message type from stored strings
    fn parse_message_type(
        &self,
        type_name: &str,
        payload: &str,
    ) -> std::result::Result<MessageType, serde_json::Error> {
        match type_name {
            "Lock" => Ok(MessageType::Lock(serde_json::from_str(payload)?)),
            "Unlock" => Ok(MessageType::Unlock(serde_json::from_str(payload)?)),
            "Notify" => Ok(MessageType::Notify(serde_json::from_str(payload)?)),
            "Request" => Ok(MessageType::Request(serde_json::from_str(payload)?)),
            "Broadcast" => Ok(MessageType::Broadcast(serde_json::from_str(payload)?)),
            "Heartbeat" => Ok(MessageType::Heartbeat(serde_json::from_str(payload)?)),
            "Response" => Ok(MessageType::Response(serde_json::from_str(payload)?)),
            _ => Ok(MessageType::Notify(super::NotifyPayload::new(
                "Unknown message type",
            ))),
        }
    }

    /// Mark a message as read
    pub fn mark_read(&self, message_id: &MessageId) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE messages SET status = 'read', read_at = ?1 WHERE id = ?2",
            params![now, message_id.as_str()],
        )?;
        Ok(())
    }

    /// Get a specific message by ID
    pub fn get_message(&self, message_id: &MessageId) -> Result<Option<StoredMessage>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, from_addr, to_addr, message_type, payload, timestamp,
                   correlation_id, status, stored_at, delivered_at, read_at
            FROM messages
            WHERE id = ?1
            "#,
        )?;

        let mut rows = stmt.query_map(params![message_id.as_str()], |row| {
            self.row_to_stored_message(row)
        })?;

        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    /// Get messages sent by an agent
    pub fn outbox(&self, address: &Address) -> Result<Vec<StoredMessage>> {
        let addr_str = address.to_string();

        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, from_addr, to_addr, message_type, payload, timestamp,
                   correlation_id, status, stored_at, delivered_at, read_at
            FROM messages
            WHERE from_addr = ?1
            ORDER BY timestamp DESC
            "#,
        )?;

        let rows = stmt.query_map(params![addr_str], |row| self.row_to_stored_message(row))?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        Ok(messages)
    }

    /// Get message count for an inbox
    pub fn inbox_count(&self, address: &Address) -> Result<usize> {
        let addr_str = address.to_string();
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE to_addr = ?1",
            params![addr_str],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Get unread message count
    pub fn unread_count(&self, address: &Address) -> Result<usize> {
        let addr_str = address.to_string();
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE to_addr = ?1 AND status = 'delivered'",
            params![addr_str],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Get the lock manager
    pub fn lock_manager(&self) -> &LockManager {
        &self.lock_manager
    }

    /// Get mutable access to lock manager
    pub fn lock_manager_mut(&mut self) -> &mut LockManager {
        &mut self.lock_manager
    }

    /// Clean up expired locks
    pub fn cleanup_expired_locks(&mut self) -> usize {
        self.lock_manager.cleanup_expired()
    }
}

/// Result of sending a message
#[derive(Debug)]
pub enum SendResult {
    /// Message was delivered to recipient
    Delivered { message_id: MessageId },

    /// Message was broadcast to multiple recipients
    Broadcast {
        message_id: MessageId,
        recipient_count: usize,
    },

    /// Lock operation result
    LockResult {
        message_id: MessageId,
        result: LockResult,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_postmaster() -> (Postmaster, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("mail.db");
        let postmaster = Postmaster::with_project_id(db_path, "test-project").unwrap();
        (postmaster, temp_dir)
    }

    #[test]
    fn test_send_and_receive() {
        let (mut postmaster, _dir) = create_test_postmaster();

        let msg = Message::from_strings(
            "worker@test-project",
            "human@localhost",
            MessageType::Notify(super::super::NotifyPayload::new("Hello!")),
        );

        let result = postmaster.send(msg).unwrap();
        assert!(matches!(result, SendResult::Delivered { .. }));

        let human = Address::human();
        let inbox = postmaster.inbox(&human).unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].message.from.to_string(), "worker@test-project");
    }

    #[test]
    fn test_lock_request() {
        let (mut postmaster, _dir) = create_test_postmaster();

        let msg = Message::from_strings(
            "worker@test-project",
            "postmaster@test-project",
            MessageType::Lock(LockRequest::new(
                "src/main.rs",
                std::time::Duration::from_secs(3600),
            )),
        );

        let result = postmaster.send(msg).unwrap();
        assert!(matches!(
            result,
            SendResult::LockResult {
                result: LockResult::Acquired { .. },
                ..
            }
        ));

        // Check that the lock exists
        let lock = postmaster.lock_manager().status("src/main.rs");
        assert!(lock.is_some());
    }

    #[test]
    fn test_unlock_request() {
        let (mut postmaster, _dir) = create_test_postmaster();

        // First acquire the lock
        let lock_msg = Message::from_strings(
            "worker@test-project",
            "postmaster@test-project",
            MessageType::Lock(LockRequest::new(
                "src/main.rs",
                std::time::Duration::from_secs(3600),
            )),
        );
        postmaster.send(lock_msg).unwrap();

        // Then release it
        let unlock_msg = Message::from_strings(
            "worker@test-project",
            "postmaster@test-project",
            MessageType::Unlock(UnlockRequest::new("src/main.rs")),
        );
        let result = postmaster.send(unlock_msg).unwrap();

        assert!(matches!(
            result,
            SendResult::LockResult {
                result: LockResult::Released,
                ..
            }
        ));
    }

    #[test]
    fn test_unread_count() {
        let (mut postmaster, _dir) = create_test_postmaster();

        let human = Address::human();
        assert_eq!(postmaster.unread_count(&human).unwrap(), 0);

        // Send some messages
        for i in 0..3 {
            let msg = Message::from_strings(
                "worker@test-project",
                "human@localhost",
                MessageType::Notify(super::super::NotifyPayload::new(format!("Message {}", i))),
            );
            postmaster.send(msg).unwrap();
        }

        assert_eq!(postmaster.unread_count(&human).unwrap(), 3);
    }

    #[test]
    fn test_mark_read() {
        let (mut postmaster, _dir) = create_test_postmaster();

        let msg = Message::from_strings(
            "worker@test-project",
            "human@localhost",
            MessageType::Notify(super::super::NotifyPayload::new("Hello!")),
        );

        postmaster.send(msg).unwrap();

        let human = Address::human();
        let inbox = postmaster.inbox(&human).unwrap();
        let message_id = &inbox[0].message.id;

        postmaster.mark_read(message_id).unwrap();

        assert_eq!(postmaster.unread_count(&human).unwrap(), 0);
    }
}
