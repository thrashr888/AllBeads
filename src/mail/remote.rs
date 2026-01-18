//! Remote mail transport for AllBeads web API
//!
//! Sends mail messages to the AllBeads web server at /api/mail
//! when authenticated with a CLI token.

use super::{Message, MessageType, Severity};
use crate::config::AllBeadsConfig;
use crate::Result;
use serde::{Deserialize, Serialize};

/// Remote mail client for AllBeads web API
#[derive(Debug)]
pub struct RemoteMailClient {
    /// Base URL of the AllBeads web server
    host: String,
    /// Bearer token for authentication
    token: String,
    /// HTTP client
    client: reqwest::Client,
    /// Organization ID for mail messages
    org_id: Option<String>,
}

/// Web API request format for sending mail
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SendMailRequest {
    org_id: String,
    from_address: String,
    to_address: String,
    message_type: String,
    subject: String,
    body: String,
    severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    bead_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo_id: Option<String>,
    requires_response: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_id: Option<String>,
    metadata: serde_json::Value,
}

/// Web API response format for sent mail
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailResponse {
    pub id: String,
    pub org_id: String,
    pub from_address: String,
    pub to_address: String,
    pub message_type: String,
    pub subject: String,
    pub body: String,
    pub severity: String,
    pub is_read: bool,
    pub is_archived: bool,
    pub created_at: String,
}

/// Web API response format for listing mail
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailListResponse {
    pub mail: Vec<RemoteStoredMessage>,
    pub total: usize,
    pub unread_count: usize,
}

/// Stored message from web API
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStoredMessage {
    pub id: String,
    pub org_id: String,
    pub from_address: String,
    pub to_address: String,
    pub message_type: String,
    pub subject: String,
    pub body: String,
    pub severity: String,
    pub is_read: bool,
    pub is_archived: bool,
    pub created_at: String,
    #[serde(default)]
    pub bead_id: Option<String>,
    #[serde(default)]
    pub repo_id: Option<String>,
    #[serde(default)]
    pub requires_response: bool,
}

/// Error response from web API
#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
}

impl RemoteMailClient {
    /// Create a new remote mail client from config
    pub fn from_config(config: &AllBeadsConfig) -> Option<Self> {
        let web_auth = &config.web_auth;

        if !web_auth.is_authenticated() {
            return None;
        }

        let token = web_auth.github_token.as_ref()?;
        let host = web_auth.host();

        Some(Self {
            host: host.to_string(),
            token: token.clone(),
            client: reqwest::Client::new(),
            org_id: None,
        })
    }

    /// Create with explicit host and token
    pub fn new(host: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            token: token.into(),
            client: reqwest::Client::new(),
            org_id: None,
        }
    }

    /// Set organization ID
    pub fn with_org_id(mut self, org_id: impl Into<String>) -> Self {
        self.org_id = Some(org_id.into());
        self
    }

    /// Send a mail message to the remote API
    pub async fn send(&self, message: &Message) -> Result<MailResponse> {
        let url = format!("{}/api/mail", self.host);

        // Convert message to API format
        let (message_type, subject, body, severity, bead_id, requires_response) =
            Self::extract_message_parts(&message.message_type);

        // Use first org if we don't have one set
        let org_id = match &self.org_id {
            Some(id) => id.clone(),
            None => {
                // Fetch user's orgs to get an org ID
                let orgs = self.get_orgs().await?;
                orgs.first().map(|o| o.id.clone()).ok_or_else(|| {
                    anyhow::anyhow!("No organizations found. Join or create an org first.")
                })?
            }
        };

        let request = SendMailRequest {
            org_id,
            from_address: message.from.to_string(),
            to_address: message.to.to_string(),
            message_type,
            subject,
            body,
            severity,
            bead_id,
            repo_id: None,
            requires_response,
            correlation_id: message
                .correlation_id
                .as_ref()
                .map(|id| id.as_str().to_string()),
            parent_id: None,
            metadata: serde_json::json!({}),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
                error: "Unknown error".to_string(),
            });
            return Err(
                anyhow::anyhow!("Failed to send mail ({}): {}", status, error.error).into(),
            );
        }

        let mail: MailResponse = response.json().await?;
        Ok(mail)
    }

    /// Get inbox from remote API
    pub async fn inbox(&self) -> Result<MailListResponse> {
        let url = format!("{}/api/mail", self.host);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
                error: "Unknown error".to_string(),
            });
            return Err(
                anyhow::anyhow!("Failed to fetch inbox ({}): {}", status, error.error).into(),
            );
        }

        let list: MailListResponse = response.json().await?;
        Ok(list)
    }

    /// Get unread count from remote API
    pub async fn unread_count(&self) -> Result<usize> {
        let url = format!("{}/api/mail?unread=true&limit=0", self.host);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
                error: "Unknown error".to_string(),
            });
            return Err(anyhow::anyhow!(
                "Failed to fetch unread count ({}): {}",
                status,
                error.error
            )
            .into());
        }

        let list: MailListResponse = response.json().await?;
        Ok(list.unread_count)
    }

    /// Mark a message as read
    /// Note: Currently uses global isRead flag. Per-agent tracking coming in abw-2c8.
    pub async fn mark_read(&self, id: &str) -> Result<()> {
        let url = format!("{}/api/mail/{}", self.host, id);

        let response = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "isRead": true }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
                error: "Unknown error".to_string(),
            });
            return Err(
                anyhow::anyhow!("Failed to mark as read ({}): {}", status, error.error).into(),
            );
        }

        Ok(())
    }

    /// Mark all messages as read
    pub async fn mark_all_read(&self) -> Result<usize> {
        let inbox = self.inbox().await?;
        let mut count = 0;
        for msg in inbox.mail.iter().filter(|m| !m.is_read) {
            self.mark_read(&msg.id).await?;
            count += 1;
        }
        Ok(count)
    }

    /// Archive a message
    pub async fn archive(&self, id: &str) -> Result<()> {
        let url = format!("{}/api/mail/{}", self.host, id);

        let response = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "isArchived": true }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
                error: "Unknown error".to_string(),
            });
            return Err(anyhow::anyhow!("Failed to archive ({}): {}", status, error.error).into());
        }

        Ok(())
    }

    /// Archive all read messages
    pub async fn archive_all_read(&self) -> Result<usize> {
        let inbox = self.inbox().await?;
        let mut count = 0;
        for msg in inbox.mail.iter().filter(|m| m.is_read && !m.is_archived) {
            self.archive(&msg.id).await?;
            count += 1;
        }
        Ok(count)
    }

    /// Delete a message
    pub async fn delete(&self, id: &str) -> Result<()> {
        let url = format!("{}/api/mail/{}", self.host, id);

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
                error: "Unknown error".to_string(),
            });
            return Err(anyhow::anyhow!("Failed to delete ({}): {}", status, error.error).into());
        }

        Ok(())
    }

    /// Get user's organizations
    async fn get_orgs(&self) -> Result<Vec<OrgInfo>> {
        let url = format!("{}/api/orgs", self.host);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(
                anyhow::anyhow!("Failed to fetch organizations ({}): {}", status, body).into(),
            );
        }

        let orgs: Vec<OrgInfo> = response.json().await?;
        Ok(orgs)
    }

    /// Extract message parts for API format
    fn extract_message_parts(
        message_type: &MessageType,
    ) -> (String, String, String, String, Option<String>, bool) {
        match message_type {
            MessageType::Notify(n) => (
                "NOTIFY".to_string(),
                n.message.chars().take(100).collect(),
                n.message.clone(),
                severity_to_string(&n.severity),
                n.bead_id.clone(),
                false,
            ),
            MessageType::Request(r) => (
                "REQUEST".to_string(),
                r.message.chars().take(100).collect(),
                format!("{}\n\nOptions: {}", r.message, r.options.join(", ")),
                "INFO".to_string(),
                None,
                true,
            ),
            MessageType::Lock(l) => (
                "LOCK".to_string(),
                format!("Lock: {}", l.path),
                format!(
                    "Lock requested for: {}\nTTL: {}s{}",
                    l.path,
                    l.ttl.as_secs(),
                    l.reason
                        .as_ref()
                        .map(|r| format!("\nReason: {}", r))
                        .unwrap_or_default()
                ),
                "INFO".to_string(),
                None,
                false,
            ),
            MessageType::Unlock(u) => (
                "UNLOCK".to_string(),
                format!("Unlock: {}", u.path),
                format!("Unlock requested for: {}", u.path),
                "INFO".to_string(),
                None,
                false,
            ),
            MessageType::Broadcast(b) => (
                "BROADCAST".to_string(),
                b.message.chars().take(100).collect(),
                b.message.clone(),
                format!("{:?}", b.category).to_uppercase(),
                None,
                false,
            ),
            MessageType::Heartbeat(h) => (
                "HEARTBEAT".to_string(),
                format!("Status: {:?}", h.status),
                format!(
                    "Status: {:?}{}{}",
                    h.status,
                    h.task
                        .as_ref()
                        .map(|t| format!("\nTask: {}", t))
                        .unwrap_or_default(),
                    h.progress
                        .map(|p| format!("\nProgress: {}%", p))
                        .unwrap_or_default()
                ),
                "INFO".to_string(),
                None,
                false,
            ),
            MessageType::Response(r) => (
                "RESPONSE".to_string(),
                format!("Response: {:?}", r.status),
                r.message
                    .clone()
                    .unwrap_or_else(|| format!("{:?}", r.status)),
                "INFO".to_string(),
                None,
                false,
            ),
            MessageType::AikiEvent(a) => (
                "NOTIFY".to_string(),
                format!("Aiki: {:?} for {}", a.event, a.bead_id),
                format!(
                    "Event: {:?}\nBead: {}\nChange: {}\nAttempts: {}{}",
                    a.event,
                    a.bead_id,
                    a.change_id,
                    a.attempts,
                    a.recommendation
                        .as_ref()
                        .map(|r| format!("\nRecommendation: {}", r))
                        .unwrap_or_default()
                ),
                "INFO".to_string(),
                Some(a.bead_id.clone()),
                false,
            ),
        }
    }
}

/// Organization info from API
#[derive(Debug, Deserialize)]
struct OrgInfo {
    id: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    slug: String,
}

/// Convert severity enum to string
fn severity_to_string(severity: &Severity) -> String {
    match severity {
        Severity::Info => "INFO".to_string(),
        Severity::Warning => "WARNING".to_string(),
        Severity::Error => "ERROR".to_string(),
        Severity::Success => "INFO".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_mail_client_creation() {
        let client = RemoteMailClient::new("https://allbeads.co", "test_token");
        assert_eq!(client.host, "https://allbeads.co");
    }

    #[test]
    fn test_severity_to_string() {
        assert_eq!(severity_to_string(&Severity::Info), "INFO");
        assert_eq!(severity_to_string(&Severity::Warning), "WARNING");
        assert_eq!(severity_to_string(&Severity::Error), "ERROR");
    }
}
