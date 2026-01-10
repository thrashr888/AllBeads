//! Agent Mail message types
//!
//! Defines the message format for agent-to-agent and agent-to-human communication.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Unique message identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(String);

impl MessageId {
    /// Create a new message ID
    pub fn new() -> Self {
        Self(format!(
            "msg-{}-{}",
            Utc::now().format("%Y%m%d%H%M%S"),
            uuid_v4()
        ))
    }

    /// Create from an existing string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the underlying string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a simple UUID v4-like string
fn uuid_v4() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{:x}{:04x}", nanos, counter)
}

/// Message type categories from the PRD
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum MessageType {
    /// Request exclusive file access
    /// Example: `LOCK src/db.rs TTL=1800`
    Lock(LockRequest),

    /// Release file lock
    /// Example: `UNLOCK src/db.rs`
    Unlock(UnlockRequest),

    /// Inform about state changes
    /// Example: `NOTIFY "PR #402 ready for review"`
    Notify(NotifyPayload),

    /// Ask for human input
    /// Example: `REQUEST "Approve scope change for Epic ab-15k?"`
    Request(RequestPayload),

    /// Announce to all agents in a project
    /// Example: `BROADCAST "JIRA API rate limit exhausted, pausing"`
    Broadcast(BroadcastPayload),

    /// Agent liveness signal
    /// Example: `HEARTBEAT agent=refactor_bot status=working`
    Heartbeat(HeartbeatPayload),

    /// Response to a previous message
    Response(ResponsePayload),
}

/// Lock request payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockRequest {
    /// File path to lock (relative to repo root)
    pub path: String,

    /// Time-to-live for the lock
    #[serde(with = "duration_seconds")]
    pub ttl: Duration,

    /// Optional reason for the lock
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl LockRequest {
    /// Create a new lock request
    pub fn new(path: impl Into<String>, ttl: Duration) -> Self {
        Self {
            path: path.into(),
            ttl,
            reason: None,
        }
    }

    /// Add a reason for the lock
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Unlock request payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnlockRequest {
    /// File path to unlock
    pub path: String,
}

impl UnlockRequest {
    /// Create a new unlock request
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

/// Notification payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotifyPayload {
    /// Notification message
    pub message: String,

    /// Severity level
    #[serde(default)]
    pub severity: Severity,

    /// Optional related bead ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bead_id: Option<String>,
}

impl NotifyPayload {
    /// Create a new notification
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            severity: Severity::Info,
            bead_id: None,
        }
    }

    /// Set severity level
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Link to a bead
    pub fn with_bead(mut self, bead_id: impl Into<String>) -> Self {
        self.bead_id = Some(bead_id.into());
        self
    }
}

/// Request payload (for human approval)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestPayload {
    /// Question or request message
    pub message: String,

    /// Available options for the human to choose
    #[serde(default)]
    pub options: Vec<String>,

    /// Is this request blocking?
    #[serde(default)]
    pub blocking: bool,

    /// Optional timeout for the request
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "option_duration_seconds")]
    pub timeout: Option<Duration>,
}

impl RequestPayload {
    /// Create a new request
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            options: vec!["Approve".to_string(), "Deny".to_string()],
            blocking: true,
            timeout: None,
        }
    }

    /// Set custom options
    pub fn with_options(mut self, options: Vec<String>) -> Self {
        self.options = options;
        self
    }

    /// Set non-blocking
    pub fn non_blocking(mut self) -> Self {
        self.blocking = false;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// Broadcast payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BroadcastPayload {
    /// Broadcast message
    pub message: String,

    /// Broadcast category
    #[serde(default)]
    pub category: BroadcastCategory,
}

impl BroadcastPayload {
    /// Create a new broadcast
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            category: BroadcastCategory::Info,
        }
    }

    /// Set category
    pub fn with_category(mut self, category: BroadcastCategory) -> Self {
        self.category = category;
        self
    }
}

/// Heartbeat payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    /// Current agent status
    pub status: AgentStatus,

    /// Optional current task description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,

    /// Optional progress percentage (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<u8>,
}

impl HeartbeatPayload {
    /// Create a new heartbeat
    pub fn new(status: AgentStatus) -> Self {
        Self {
            status,
            task: None,
            progress: None,
        }
    }

    /// Set current task
    pub fn with_task(mut self, task: impl Into<String>) -> Self {
        self.task = Some(task.into());
        self
    }

    /// Set progress
    pub fn with_progress(mut self, progress: u8) -> Self {
        self.progress = Some(progress.min(100));
        self
    }
}

/// Response payload
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponsePayload {
    /// ID of the message being responded to
    pub in_reply_to: MessageId,

    /// Response status
    pub status: ResponseStatus,

    /// Optional response message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Optional data payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl ResponsePayload {
    /// Create a successful response
    pub fn success(in_reply_to: MessageId) -> Self {
        Self {
            in_reply_to,
            status: ResponseStatus::Success,
            message: None,
            data: None,
        }
    }

    /// Create an error response
    pub fn error(in_reply_to: MessageId, message: impl Into<String>) -> Self {
        Self {
            in_reply_to,
            status: ResponseStatus::Error,
            message: Some(message.into()),
            data: None,
        }
    }

    /// Create a denied response (for locks)
    pub fn denied(in_reply_to: MessageId, message: impl Into<String>) -> Self {
        Self {
            in_reply_to,
            status: ResponseStatus::Denied,
            message: Some(message.into()),
            data: None,
        }
    }
}

/// Severity levels for notifications
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    #[default]
    Info,
    Warning,
    Error,
    Success,
}

/// Broadcast categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BroadcastCategory {
    #[default]
    Info,
    Alert,
    RateLimit,
    Maintenance,
    Shutdown,
}

/// Agent status for heartbeats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Idle,
    Working,
    Blocked,
    Paused,
    Error,
}

/// Response status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Success,
    Error,
    Denied,
    Pending,
    Timeout,
}

/// Complete message envelope
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// Unique message ID
    pub id: MessageId,

    /// Sender address (e.g., "refactor_bot@legacy-repo")
    pub from: super::Address,

    /// Recipient address (e.g., "human@localhost", "all@project")
    pub to: super::Address,

    /// Message type and payload
    #[serde(flatten)]
    pub message_type: MessageType,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Optional correlation ID for request/response matching
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<MessageId>,
}

impl Message {
    /// Create a new message with Address types
    pub fn new(from: super::Address, to: super::Address, message_type: MessageType) -> Self {
        Self {
            id: MessageId::new(),
            from,
            to,
            message_type,
            timestamp: Utc::now(),
            correlation_id: None,
        }
    }

    /// Create a new message from string addresses (convenience method)
    ///
    /// # Panics
    /// Panics if the addresses are invalid. Use `try_new` for fallible creation.
    pub fn from_strings(
        from: impl AsRef<str>,
        to: impl AsRef<str>,
        message_type: MessageType,
    ) -> Self {
        Self::new(
            from.as_ref().parse().expect("invalid from address"),
            to.as_ref().parse().expect("invalid to address"),
            message_type,
        )
    }

    /// Try to create a new message from string addresses
    pub fn try_new(
        from: impl AsRef<str>,
        to: impl AsRef<str>,
        message_type: MessageType,
    ) -> Result<Self, super::AddressError> {
        Ok(Self::new(
            from.as_ref().parse()?,
            to.as_ref().parse()?,
            message_type,
        ))
    }

    /// Set correlation ID
    pub fn with_correlation(mut self, correlation_id: MessageId) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Check if this is a lock request
    pub fn is_lock(&self) -> bool {
        matches!(self.message_type, MessageType::Lock(_))
    }

    /// Check if this is addressed to the human inbox
    pub fn is_for_human(&self) -> bool {
        self.to.is_human()
    }

    /// Check if this is a broadcast
    pub fn is_broadcast(&self) -> bool {
        self.to.is_broadcast()
    }

    /// Get the routing target for this message
    pub fn routing_target(&self) -> super::RoutingTarget {
        super::RoutingTarget::from_address(&self.to)
    }

    /// Get the sender's address
    pub fn sender(&self) -> &super::Address {
        &self.from
    }

    /// Get the recipient's address
    pub fn recipient(&self) -> &super::Address {
        &self.to
    }
}

// Custom serialization for Duration as seconds
mod duration_seconds {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

mod option_duration_seconds {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match duration {
            Some(d) => serializer.serialize_some(&d.as_secs()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<u64> = Option::deserialize(deserializer)?;
        Ok(opt.map(Duration::from_secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_request_serialization() {
        let lock = LockRequest::new("src/auth.rs", Duration::from_secs(3600))
            .with_reason("Refactoring authentication");

        let json = serde_json::to_string(&lock).unwrap();
        assert!(json.contains("src/auth.rs"));
        assert!(json.contains("3600"));
        assert!(json.contains("Refactoring"));

        let parsed: LockRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.path, "src/auth.rs");
        assert_eq!(parsed.ttl, Duration::from_secs(3600));
    }

    #[test]
    fn test_message_type_serialization() {
        let msg_type = MessageType::Lock(LockRequest::new("test.rs", Duration::from_secs(1800)));
        let json = serde_json::to_string(&msg_type).unwrap();
        assert!(json.contains(r#""type":"Lock"#));
    }

    #[test]
    fn test_full_message_serialization() {
        let msg = Message::from_strings(
            "refactor_bot@auth-service",
            "human@localhost",
            MessageType::Request(RequestPayload::new("Approve scope change?")),
        );

        let json = serde_json::to_string_pretty(&msg).unwrap();
        assert!(json.contains("refactor_bot@auth-service"));
        assert!(json.contains("human@localhost"));
        assert!(json.contains("Request"));

        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.from.to_string(), "refactor_bot@auth-service");
        assert!(parsed.is_for_human());
    }

    #[test]
    fn test_message_id_generation() {
        let id1 = MessageId::new();
        let id2 = MessageId::new();
        // IDs should be different
        assert_ne!(id1.as_str(), id2.as_str());
        // IDs should start with "msg-"
        assert!(id1.as_str().starts_with("msg-"));
    }

    #[test]
    fn test_heartbeat_payload() {
        let heartbeat = HeartbeatPayload::new(AgentStatus::Working)
            .with_task("Refactoring auth module")
            .with_progress(45);

        assert_eq!(heartbeat.status, AgentStatus::Working);
        assert_eq!(heartbeat.task, Some("Refactoring auth module".to_string()));
        assert_eq!(heartbeat.progress, Some(45));
    }

    #[test]
    fn test_broadcast_detection() {
        let msg = Message::from_strings(
            "agent@project",
            "all@project",
            MessageType::Broadcast(BroadcastPayload::new("System maintenance")),
        );
        assert!(msg.is_broadcast());
        assert!(!msg.is_for_human());
    }

    #[test]
    fn test_response_payload() {
        let original_id = MessageId::new();
        let response = ResponsePayload::success(original_id.clone());
        assert_eq!(response.status, ResponseStatus::Success);
        assert_eq!(response.in_reply_to, original_id);
    }
}
