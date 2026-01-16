//! Filesystem-based mail transport
//!
//! Stores messages as JSON files in `.beads/mail/`.
//!
//! Directory structure:
//! ```text
//! .beads/mail/
//!   inbox/
//!     human@localhost/
//!       <message-id>.json
//!     worker@project/
//!       <message-id>.json
//!   outbox/
//!     worker@project/
//!       <message-id>.json
//!   index.jsonl  # Optional index for fast lookups
//! ```
//!
//! Each message file contains the full StoredMessage as JSON.

use super::transport::{MailTransport, Result, TransportError};
use super::{Address, DeliveryStatus, Message, MessageId, MessageType, StoredMessage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Filesystem mail transport
pub struct FilesystemTransport {
    /// Base directory for mail storage (typically `.beads/mail`)
    base_path: PathBuf,
}

/// JSON-serializable message record
#[derive(Debug, Serialize, Deserialize)]
struct MessageRecord {
    /// The message
    #[serde(flatten)]
    message: MessageJson,
    /// Delivery status
    status: String,
    /// When the message was stored
    stored_at: String,
    /// When the message was delivered
    delivered_at: Option<String>,
    /// When the message was read
    read_at: Option<String>,
}

/// JSON-serializable message format
#[derive(Debug, Serialize, Deserialize)]
struct MessageJson {
    id: String,
    from: String,
    to: String,
    message_type: String,
    payload: serde_json::Value,
    timestamp: String,
    correlation_id: Option<String>,
}

impl FilesystemTransport {
    /// Create a new filesystem transport
    pub fn new(base_path: impl Into<PathBuf>) -> Result<Self> {
        let base_path = base_path.into();

        // Create directory structure
        fs::create_dir_all(base_path.join("inbox"))?;
        fs::create_dir_all(base_path.join("outbox"))?;

        Ok(Self { base_path })
    }

    /// Get the inbox directory for an address
    fn inbox_dir(&self, address: &Address) -> PathBuf {
        self.base_path
            .join("inbox")
            .join(sanitize_address(address))
    }

    /// Get the outbox directory for an address
    fn outbox_dir(&self, address: &Address) -> PathBuf {
        self.base_path
            .join("outbox")
            .join(sanitize_address(address))
    }

    /// Get the path for a specific message
    fn message_path(&self, address: &Address, message_id: &MessageId, is_inbox: bool) -> PathBuf {
        let dir = if is_inbox {
            self.inbox_dir(address)
        } else {
            self.outbox_dir(address)
        };
        dir.join(format!("{}.json", message_id.as_str()))
    }

    /// Write a message to disk
    fn write_message(&self, path: &Path, record: &MessageRecord) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(record)?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    /// Read a message from disk
    fn read_message(&self, path: &Path) -> Result<StoredMessage> {
        let content = fs::read_to_string(path)?;
        let record: MessageRecord = serde_json::from_str(&content)?;
        record_to_stored_message(record)
    }

    /// Update a message on disk
    fn update_message(&self, path: &Path, update_fn: impl FnOnce(&mut MessageRecord)) -> Result<()> {
        let content = fs::read_to_string(path)?;
        let mut record: MessageRecord = serde_json::from_str(&content)?;
        update_fn(&mut record);
        let json = serde_json::to_string_pretty(&record)?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    /// List all message files in a directory
    fn list_messages(&self, dir: &Path) -> Result<Vec<StoredMessage>> {
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut messages = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                match self.read_message(&path) {
                    Ok(msg) => messages.push(msg),
                    Err(e) => {
                        // Log but don't fail on individual message errors
                        tracing::warn!("Failed to read message {:?}: {}", path, e);
                    }
                }
            }
        }

        // Sort by timestamp descending
        messages.sort_by(|a, b| b.message.timestamp.cmp(&a.message.timestamp));
        Ok(messages)
    }

    /// Find a message by ID across all directories
    fn find_message(&self, message_id: &MessageId) -> Result<Option<(PathBuf, StoredMessage)>> {
        // Search inbox directories
        if let Ok(entries) = fs::read_dir(self.base_path.join("inbox")) {
            for entry in entries.flatten() {
                let path = entry.path().join(format!("{}.json", message_id.as_str()));
                if path.exists() {
                    return Ok(Some((path.clone(), self.read_message(&path)?)));
                }
            }
        }

        // Search outbox directories
        if let Ok(entries) = fs::read_dir(self.base_path.join("outbox")) {
            for entry in entries.flatten() {
                let path = entry.path().join(format!("{}.json", message_id.as_str()));
                if path.exists() {
                    return Ok(Some((path.clone(), self.read_message(&path)?)));
                }
            }
        }

        Ok(None)
    }
}

impl MailTransport for FilesystemTransport {
    fn store(&self, message: &Message, status: DeliveryStatus) -> Result<()> {
        let now = Utc::now();
        let delivered_at = if status == DeliveryStatus::Delivered {
            Some(now.to_rfc3339())
        } else {
            None
        };

        let record = MessageRecord {
            message: message_to_json(message),
            status: status_to_string(status),
            stored_at: now.to_rfc3339(),
            delivered_at,
            read_at: None,
        };

        // Store in recipient's inbox
        let inbox_path = self.message_path(&message.to, &message.id, true);
        self.write_message(&inbox_path, &record)?;

        // Store in sender's outbox
        let outbox_path = self.message_path(&message.from, &message.id, false);
        self.write_message(&outbox_path, &record)?;

        // Append to index for fast lookups
        self.append_to_index(message, status)?;

        Ok(())
    }

    fn inbox(&self, address: &Address) -> Result<Vec<StoredMessage>> {
        self.list_messages(&self.inbox_dir(address))
    }

    fn inbox_with_status(
        &self,
        address: &Address,
        status: DeliveryStatus,
    ) -> Result<Vec<StoredMessage>> {
        let messages = self.inbox(address)?;
        Ok(messages.into_iter().filter(|m| m.status == status).collect())
    }

    fn outbox(&self, address: &Address) -> Result<Vec<StoredMessage>> {
        self.list_messages(&self.outbox_dir(address))
    }

    fn get(&self, message_id: &MessageId) -> Result<Option<StoredMessage>> {
        Ok(self.find_message(message_id)?.map(|(_, msg)| msg))
    }

    fn mark_read(&self, message_id: &MessageId) -> Result<()> {
        if let Some((path, _)) = self.find_message(message_id)? {
            self.update_message(&path, |record| {
                record.status = "read".to_string();
                record.read_at = Some(Utc::now().to_rfc3339());
            })?;
        }
        Ok(())
    }

    fn inbox_count(&self, address: &Address) -> Result<usize> {
        let dir = self.inbox_dir(address);
        if !dir.exists() {
            return Ok(0);
        }
        Ok(fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
            .count())
    }

    fn unread_count(&self, address: &Address) -> Result<usize> {
        let messages = self.inbox_with_status(address, DeliveryStatus::Delivered)?;
        Ok(messages.len())
    }
}

impl FilesystemTransport {
    /// Append message to index for fast lookups
    fn append_to_index(&self, message: &Message, status: DeliveryStatus) -> Result<()> {
        let index_path = self.base_path.join("index.jsonl");
        let entry = serde_json::json!({
            "id": message.id.as_str(),
            "from": message.from.to_string(),
            "to": message.to.to_string(),
            "timestamp": message.timestamp.to_rfc3339(),
            "status": status_to_string(status),
        });

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(index_path)?;
        writeln!(file, "{}", entry)?;
        Ok(())
    }
}

/// Sanitize an address for use as a directory name
fn sanitize_address(address: &Address) -> String {
    address.to_string().replace(['/', '\\', ':'], "_")
}

/// Convert message to JSON format
fn message_to_json(message: &Message) -> MessageJson {
    let (msg_type, payload) = match &message.message_type {
        MessageType::Lock(p) => ("Lock", serde_json::to_value(p).unwrap_or_default()),
        MessageType::Unlock(p) => ("Unlock", serde_json::to_value(p).unwrap_or_default()),
        MessageType::Notify(p) => ("Notify", serde_json::to_value(p).unwrap_or_default()),
        MessageType::Request(p) => ("Request", serde_json::to_value(p).unwrap_or_default()),
        MessageType::Broadcast(p) => ("Broadcast", serde_json::to_value(p).unwrap_or_default()),
        MessageType::Heartbeat(p) => ("Heartbeat", serde_json::to_value(p).unwrap_or_default()),
        MessageType::Response(p) => ("Response", serde_json::to_value(p).unwrap_or_default()),
        MessageType::AikiEvent(p) => ("AikiEvent", serde_json::to_value(p).unwrap_or_default()),
    };

    MessageJson {
        id: message.id.as_str().to_string(),
        from: message.from.to_string(),
        to: message.to.to_string(),
        message_type: msg_type.to_string(),
        payload,
        timestamp: message.timestamp.to_rfc3339(),
        correlation_id: message.correlation_id.as_ref().map(|id| id.as_str().to_string()),
    }
}

/// Convert status to string
fn status_to_string(status: DeliveryStatus) -> String {
    match status {
        DeliveryStatus::Pending => "pending",
        DeliveryStatus::Delivered => "delivered",
        DeliveryStatus::Read => "read",
        DeliveryStatus::Failed => "failed",
    }
    .to_string()
}

/// Parse status from string
fn status_from_string(s: &str) -> DeliveryStatus {
    match s {
        "pending" => DeliveryStatus::Pending,
        "delivered" => DeliveryStatus::Delivered,
        "read" => DeliveryStatus::Read,
        "failed" => DeliveryStatus::Failed,
        _ => DeliveryStatus::Pending,
    }
}

/// Convert a record to StoredMessage
fn record_to_stored_message(record: MessageRecord) -> Result<StoredMessage> {
    let from: Address = record
        .message
        .from
        .parse()
        .map_err(|e| TransportError::Storage(format!("Invalid from address: {}", e)))?;
    let to: Address = record
        .message
        .to
        .parse()
        .map_err(|e| TransportError::Storage(format!("Invalid to address: {}", e)))?;

    let message_type = parse_message_type(&record.message.message_type, &record.message.payload)?;

    let timestamp = DateTime::parse_from_rfc3339(&record.message.timestamp)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let stored_at = DateTime::parse_from_rfc3339(&record.stored_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let delivered_at = record.delivered_at.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Utc))
            .ok()
    });
    let read_at = record.read_at.and_then(|s| {
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Utc))
            .ok()
    });

    let message = Message {
        id: MessageId::from_string(record.message.id),
        from,
        to,
        message_type,
        timestamp,
        correlation_id: record.message.correlation_id.map(MessageId::from_string),
    };

    Ok(StoredMessage {
        message,
        status: status_from_string(&record.status),
        stored_at,
        delivered_at,
        read_at,
    })
}

/// Parse message type from stored data
fn parse_message_type(type_name: &str, payload: &serde_json::Value) -> Result<MessageType> {
    use super::NotifyPayload;

    match type_name {
        "Lock" => Ok(MessageType::Lock(serde_json::from_value(payload.clone())?)),
        "Unlock" => Ok(MessageType::Unlock(serde_json::from_value(payload.clone())?)),
        "Notify" => Ok(MessageType::Notify(serde_json::from_value(payload.clone())?)),
        "Request" => Ok(MessageType::Request(serde_json::from_value(payload.clone())?)),
        "Broadcast" => Ok(MessageType::Broadcast(serde_json::from_value(
            payload.clone(),
        )?)),
        "Heartbeat" => Ok(MessageType::Heartbeat(serde_json::from_value(
            payload.clone(),
        )?)),
        "Response" => Ok(MessageType::Response(serde_json::from_value(
            payload.clone(),
        )?)),
        "AikiEvent" => Ok(MessageType::AikiEvent(serde_json::from_value(
            payload.clone(),
        )?)),
        _ => Ok(MessageType::Notify(NotifyPayload::new(
            "Unknown message type",
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_transport() -> (FilesystemTransport, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let transport = FilesystemTransport::new(temp_dir.path().join("mail")).unwrap();
        (transport, temp_dir)
    }

    #[test]
    fn test_store_and_retrieve() {
        let (transport, _dir) = create_test_transport();

        let msg = Message::from_strings(
            "worker@test-project",
            "human@localhost",
            MessageType::Notify(super::super::NotifyPayload::new("Hello!")),
        );

        transport.store(&msg, DeliveryStatus::Delivered).unwrap();

        let human = Address::human();
        let inbox = transport.inbox(&human).unwrap();
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].message.id, msg.id);
    }

    #[test]
    fn test_inbox_count() {
        let (transport, _dir) = create_test_transport();

        let human = Address::human();
        assert_eq!(transport.inbox_count(&human).unwrap(), 0);

        for i in 0..3 {
            let msg = Message::from_strings(
                "worker@test-project",
                "human@localhost",
                MessageType::Notify(super::super::NotifyPayload::new(format!("Message {}", i))),
            );
            transport.store(&msg, DeliveryStatus::Delivered).unwrap();
        }

        assert_eq!(transport.inbox_count(&human).unwrap(), 3);
    }

    #[test]
    fn test_mark_read() {
        let (transport, _dir) = create_test_transport();

        let msg = Message::from_strings(
            "worker@test-project",
            "human@localhost",
            MessageType::Notify(super::super::NotifyPayload::new("Hello!")),
        );

        transport.store(&msg, DeliveryStatus::Delivered).unwrap();

        let human = Address::human();
        assert_eq!(transport.unread_count(&human).unwrap(), 1);

        transport.mark_read(&msg.id).unwrap();

        assert_eq!(transport.unread_count(&human).unwrap(), 0);
    }

    #[test]
    fn test_outbox() {
        let (transport, _dir) = create_test_transport();

        let msg = Message::from_strings(
            "worker@test-project",
            "human@localhost",
            MessageType::Notify(super::super::NotifyPayload::new("Hello!")),
        );

        transport.store(&msg, DeliveryStatus::Delivered).unwrap();

        let worker: Address = "worker@test-project".parse().unwrap();
        let outbox = transport.outbox(&worker).unwrap();
        assert_eq!(outbox.len(), 1);
        assert_eq!(outbox[0].message.id, msg.id);
    }
}
