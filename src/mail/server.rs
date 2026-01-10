//! HTTP server for Agent Mail
//!
//! Provides a REST API for message sending, inbox management, and lock operations.
//!
//! # Overview
//!
//! The server exposes the Postmaster functionality via HTTP:
//! - Send messages
//! - Query inboxes
//! - Manage file locks
//!
//! # Routes
//!
//! - `POST /send` - Send a message
//! - `GET /inbox/{address}` - Get inbox for an address
//! - `GET /unread/{address}` - Get unread messages
//! - `POST /read/{message_id}` - Mark message as read
//! - `GET /locks` - List active locks
//! - `POST /lock/status` - Check lock status (body: `{"path": "..."}`)
//! - `POST /lock/release` - Force release a lock (body: `{"path": "..."}`)
//!
//! # Example
//!
//! ```no_run
//! use allbeads::mail::MailServer;
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = MailServer::new(PathBuf::from("mail.db"), "my-project")
//!         .expect("Failed to create server");
//!
//!     server.run("127.0.0.1:8080").await.expect("Server failed");
//! }
//! ```

use super::{
    Address, LockInfo, LockResult, Message, MessageId, MessageType, Postmaster, PostmasterError,
    SendResult, StoredMessage,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

/// Server error types
#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Postmaster error: {0}")]
    Postmaster(#[from] PostmasterError),

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Bind error: {0}")]
    Bind(String),
}

/// Shared server state
struct AppState {
    postmaster: Mutex<Postmaster>,
}

/// HTTP server for Agent Mail
pub struct MailServer {
    state: Arc<AppState>,
}

impl MailServer {
    /// Create a new mail server
    pub fn new(db_path: PathBuf, project_id: impl Into<String>) -> Result<Self, ServerError> {
        let postmaster = Postmaster::with_project_id(db_path, project_id)?;
        Ok(Self {
            state: Arc::new(AppState {
                postmaster: Mutex::new(postmaster),
            }),
        })
    }

    /// Build the router
    fn router(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/health", get(health))
            .route("/send", post(send_message))
            .route("/inbox/{address}", get(get_inbox))
            .route("/unread/{address}", get(get_unread))
            .route("/read/{message_id}", post(mark_read))
            .route("/locks", get(list_locks))
            .route("/lock/status", post(get_lock_status))
            .route("/lock/release", post(force_release_lock))
            .with_state(state)
    }

    /// Run the server on the given address
    pub async fn run(self, addr: &str) -> Result<(), ServerError> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| ServerError::Bind(e.to_string()))?;

        tracing::info!("Mail server listening on {}", addr);

        axum::serve(listener, Self::router(self.state))
            .await
            .map_err(ServerError::Io)
    }

    /// Get a reference to the postmaster (for testing)
    pub fn postmaster(&self) -> &Mutex<Postmaster> {
        &self.state.postmaster
    }
}

// ============================================================================
// Request/Response types
// ============================================================================

/// Request to send a message
#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub from: String,
    pub to: String,
    pub message_type: MessageType,
    pub correlation_id: Option<String>,
}

/// Response from sending a message
#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    pub success: bool,
    pub message_id: Option<String>,
    pub result: SendResultDto,
}

/// DTO for send result
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum SendResultDto {
    Delivered {
        message_id: String,
    },
    Broadcast {
        message_id: String,
        recipient_count: usize,
    },
    LockResult {
        message_id: String,
        lock_result: LockResultDto,
    },
}

/// DTO for lock result
#[derive(Debug, Serialize)]
#[serde(tag = "status")]
pub enum LockResultDto {
    Acquired { expires_at: String },
    Denied { holder: String, expires_at: String, reason: Option<String> },
    Released,
    NotLocked,
    Stolen { previous_holder: String, expires_at: String },
}

impl From<LockResult> for LockResultDto {
    fn from(result: LockResult) -> Self {
        match result {
            LockResult::Acquired { expires_at } => LockResultDto::Acquired {
                expires_at: expires_at.to_rfc3339(),
            },
            LockResult::Denied { holder, expires_at, reason } => LockResultDto::Denied {
                holder: holder.to_string(),
                expires_at: expires_at.to_rfc3339(),
                reason,
            },
            LockResult::Released => LockResultDto::Released,
            LockResult::NotLocked => LockResultDto::NotLocked,
            LockResult::Stolen { previous_holder, expires_at } => LockResultDto::Stolen {
                previous_holder: previous_holder.to_string(),
                expires_at: expires_at.to_rfc3339(),
            },
        }
    }
}

impl From<SendResult> for SendResultDto {
    fn from(result: SendResult) -> Self {
        match result {
            SendResult::Delivered { message_id } => SendResultDto::Delivered {
                message_id: message_id.as_str().to_string(),
            },
            SendResult::Broadcast { message_id, recipient_count } => SendResultDto::Broadcast {
                message_id: message_id.as_str().to_string(),
                recipient_count,
            },
            SendResult::LockResult { message_id, result } => SendResultDto::LockResult {
                message_id: message_id.as_str().to_string(),
                lock_result: result.into(),
            },
        }
    }
}

/// DTO for stored message
#[derive(Debug, Serialize)]
pub struct StoredMessageDto {
    pub id: String,
    pub from: String,
    pub to: String,
    pub message_type: MessageType,
    pub timestamp: String,
    pub correlation_id: Option<String>,
    pub status: String,
    pub read_at: Option<String>,
}

impl From<StoredMessage> for StoredMessageDto {
    fn from(msg: StoredMessage) -> Self {
        Self {
            id: msg.message.id.as_str().to_string(),
            from: msg.message.from.to_string(),
            to: msg.message.to.to_string(),
            message_type: msg.message.message_type,
            timestamp: msg.message.timestamp.to_rfc3339(),
            correlation_id: msg.message.correlation_id.map(|id| id.as_str().to_string()),
            status: format!("{:?}", msg.status),
            read_at: msg.read_at.map(|t| t.to_rfc3339()),
        }
    }
}

/// DTO for lock info
#[derive(Debug, Serialize)]
pub struct LockInfoDto {
    pub path: String,
    pub holder: String,
    pub acquired_at: String,
    pub expires_at: String,
    pub reason: Option<String>,
}

impl From<&LockInfo> for LockInfoDto {
    fn from(info: &LockInfo) -> Self {
        Self {
            path: info.path.clone(),
            holder: info.holder.to_string(),
            acquired_at: info.acquired_at.to_rfc3339(),
            expires_at: info.expires_at.to_rfc3339(),
            reason: info.reason.clone(),
        }
    }
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Request for lock operations
#[derive(Debug, Deserialize)]
pub struct LockPathRequest {
    pub path: String,
}

// ============================================================================
// Handlers
// ============================================================================

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn send_message(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SendMessageRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Parse addresses
    let from: Address = req.from.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid 'from' address: {}", req.from),
            }),
        )
    })?;

    let to: Address = req.to.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid 'to' address: {}", req.to),
            }),
        )
    })?;

    // Build message
    let mut message = Message::new(from, to, req.message_type);
    if let Some(corr_id) = req.correlation_id {
        message.correlation_id = Some(MessageId::from_string(corr_id));
    }

    // Send via postmaster
    let mut postmaster = state.postmaster.lock().await;
    let result = postmaster.send(message).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let message_id = match &result {
        SendResult::Delivered { message_id } => message_id.as_str().to_string(),
        SendResult::Broadcast { message_id, .. } => message_id.as_str().to_string(),
        SendResult::LockResult { message_id, .. } => message_id.as_str().to_string(),
    };

    Ok(Json(SendMessageResponse {
        success: true,
        message_id: Some(message_id),
        result: result.into(),
    }))
}

async fn get_inbox(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let addr: Address = address.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid address: {}", address),
            }),
        )
    })?;

    let postmaster = state.postmaster.lock().await;
    let messages = postmaster.inbox(&addr).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let dtos: Vec<StoredMessageDto> = messages.into_iter().map(|m| m.into()).collect();
    Ok(Json(dtos))
}

async fn get_unread(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let addr: Address = address.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid address: {}", address),
            }),
        )
    })?;

    let postmaster = state.postmaster.lock().await;
    let messages = postmaster.unread(&addr).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    let dtos: Vec<StoredMessageDto> = messages.into_iter().map(|m| m.into()).collect();
    Ok(Json(dtos))
}

async fn mark_read(
    State(state): State<Arc<AppState>>,
    Path(message_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let id = MessageId::from_string(message_id);

    let postmaster = state.postmaster.lock().await;
    postmaster.mark_read(&id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn list_locks(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let postmaster = state.postmaster.lock().await;
    let locks = postmaster.lock_manager().active_locks();

    let dtos: Vec<LockInfoDto> = locks.iter().map(|l| (*l).into()).collect();
    Ok(Json(dtos))
}

async fn get_lock_status(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LockPathRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let postmaster = state.postmaster.lock().await;

    if let Some(info) = postmaster.lock_manager().status(&req.path) {
        Ok(Json(serde_json::json!({
            "locked": true,
            "info": LockInfoDto::from(info)
        })))
    } else {
        Ok(Json(serde_json::json!({
            "locked": false,
            "info": null
        })))
    }
}

async fn force_release_lock(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LockPathRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let mut postmaster = state.postmaster.lock().await;
    let result = postmaster.lock_manager_mut().force_release(&req.path);

    Ok(Json(serde_json::json!({
        "success": matches!(result, LockResult::Released),
        "result": LockResultDto::from(result)
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tempfile::TempDir;
    use tower::ServiceExt;

    fn create_test_server() -> (Arc<AppState>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("mail.db");
        let postmaster = Postmaster::with_project_id(db_path, "test").unwrap();
        let state = Arc::new(AppState {
            postmaster: Mutex::new(postmaster),
        });
        (state, temp_dir)
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let (state, _temp) = create_test_server();
        let app = MailServer::router(state);

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_send_message() {
        let (state, _temp) = create_test_server();
        let app = MailServer::router(state);

        let body = serde_json::json!({
            "from": "worker@test",
            "to": "human@localhost",
            "message_type": {
                "type": "Notify",
                "payload": {
                    "message": "Task completed",
                    "severity": "info"
                }
            }
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/send")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_inbox() {
        let (state, _temp) = create_test_server();

        // First send a message
        {
            let mut postmaster = state.postmaster.lock().await;
            let msg = Message::from_strings(
                "worker@test",
                "human@localhost",
                MessageType::Notify(super::super::NotifyPayload::new("Test")),
            );
            postmaster.send(msg).unwrap();
        }

        let app = MailServer::router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/inbox/human@localhost")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_list_locks() {
        let (state, _temp) = create_test_server();
        let app = MailServer::router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/locks")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
