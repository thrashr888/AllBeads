//! JIRA Integration Adapter
//!
//! Bi-directional sync between AllBeads and JIRA using the REST API.

use crate::config::JiraIntegration;
use crate::graph::{Bead, BeadId, ShadowBead};
use crate::Result;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Per-request timeout for search/query operations (large result sets)
const SEARCH_TIMEOUT: Duration = Duration::from_secs(30);
/// Per-request timeout for single issue fetches
const GET_TIMEOUT: Duration = Duration::from_secs(10);
/// Per-request timeout for create/update operations
const WRITE_TIMEOUT: Duration = Duration::from_secs(15);

/// JIRA API client for bi-directional sync
pub struct JiraAdapter {
    client: Client,
    config: JiraIntegration,
    base_url: String,
    auth_token: Option<String>,
}

/// JIRA issue representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssue {
    pub key: String,
    pub id: String,
    pub fields: JiraFields,
}

/// JIRA issue fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraFields {
    pub summary: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "issuetype")]
    pub issue_type: JiraIssueType,
    pub status: JiraStatus,
    #[serde(default)]
    pub priority: Option<JiraPriority>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub assignee: Option<JiraUser>,
    #[serde(default)]
    pub reporter: Option<JiraUser>,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssueType {
    pub name: String,
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraStatus {
    pub name: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "statusCategory", default)]
    pub status_category: Option<JiraStatusCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraStatusCategory {
    pub key: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraPriority {
    pub name: String,
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraUser {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "accountId", default)]
    pub account_id: Option<String>,
    #[serde(rename = "emailAddress", default)]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraSearchResponse {
    pub total: u32,
    #[serde(rename = "startAt")]
    pub start_at: u32,
    #[serde(rename = "maxResults")]
    pub max_results: u32,
    pub issues: Vec<JiraIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraTransition {
    pub id: String,
    pub name: String,
    pub to: JiraStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraTransitionsResponse {
    pub transitions: Vec<JiraTransition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraComment {
    pub id: String,
    pub body: String,
    pub author: JiraUser,
    pub created: String,
    pub updated: String,
}

/// JIRA project version (equivalent to GitHub milestone)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraVersion {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub released: bool,
    #[serde(rename = "releaseDate", default)]
    pub release_date: Option<String>,
    #[serde(rename = "startDate", default)]
    pub start_date: Option<String>,
    #[serde(rename = "projectId", default)]
    pub project_id: Option<i64>,
}

/// Request to create/update a JIRA version
#[derive(Debug, Clone, Serialize)]
pub struct JiraVersionRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "releaseDate", skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    #[serde(rename = "startDate", skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub released: Option<bool>,
    #[serde(rename = "projectId")]
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize)]
struct JiraCommentCreate {
    body: String,
}

#[derive(Debug, Clone, Serialize)]
struct JiraTransitionRequest {
    transition: JiraTransitionId,
}

#[derive(Debug, Clone, Serialize)]
struct JiraTransitionId {
    id: String,
}

/// Sync result for a single issue
#[derive(Debug, Clone)]
pub struct JiraSyncResult {
    pub jira_key: String,
    pub bead_id: Option<BeadId>,
    pub action: JiraSyncAction,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JiraSyncAction {
    CreatedBead,
    UpdatedBead,
    UpdatedJira,
    NoChange,
    Skipped,
    Error,
}

#[derive(Debug, thiserror::Error)]
pub enum JiraError {
    #[error("JIRA API error: {0}")]
    Api(String),
    #[error("Authentication failed: {0}")]
    Auth(String),
    #[error("Issue not found: {0}")]
    NotFound(String),
    #[error("Rate limited, retry after {0} seconds")]
    RateLimited(u64),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

#[derive(Debug, Default, Clone)]
pub struct JiraSyncStats {
    pub issues_pulled: u32,
    pub created: u32,
    pub updated: u32,
    pub jira_updated: u32,
    pub skipped: u32,
    pub errors: u32,
}

impl JiraAdapter {
    /// Create a new JIRA adapter
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn new(config: JiraIntegration) -> Result<Self> {
        let client = Client::builder().timeout(Duration::from_secs(30)).build()?; // reqwest::Error converts to AllBeadsError::Http via #[from]

        let base_url = format!("{}/rest/api/3", config.url.trim_end_matches('/'));

        let auth_token = config
            .token_env
            .as_ref()
            .and_then(|env_var| std::env::var(env_var.trim_start_matches('$')).ok());

        Ok(Self {
            client,
            config,
            base_url,
            auth_token,
        })
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    /// Set the authentication token
    pub fn set_auth_token(&mut self, token: String) {
        self.auth_token = Some(token);
    }

    pub fn is_authenticated(&self) -> bool {
        self.auth_token.is_some()
    }

    pub fn project(&self) -> &str {
        &self.config.project
    }

    /// Map JIRA status to AllBeads Status
    pub fn map_jira_status(&self, status_name: &str) -> crate::graph::Status {
        match status_name.to_lowercase().as_str() {
            "done" | "closed" | "resolved" => crate::graph::Status::Closed,
            "in progress" | "in review" | "in development" => crate::graph::Status::InProgress,
            "blocked" | "on hold" => crate::graph::Status::Blocked,
            "deferred" | "backlog" => crate::graph::Status::Deferred,
            _ => crate::graph::Status::Open,
        }
    }

    /// Search for issues using JQL
    pub async fn search(&self, jql: &str, max_results: u32) -> Result<Vec<JiraIssue>> {
        let url = format!("{}/search", self.base_url);

        let params = [
            ("jql", jql.to_string()),
            ("maxResults", max_results.to_string()),
            ("fields", "summary,description,issuetype,status,priority,labels,assignee,reporter,updated,created".to_string()),
        ];

        debug!(jql = %jql, max_results = %max_results, "Searching JIRA issues");

        let mut request = self.client.get(&url).query(&params);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.timeout(SEARCH_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::OK => {
                let search_result: JiraSearchResponse = response.json().await?;
                info!(
                    total = search_result.total,
                    returned = search_result.issues.len(),
                    "JIRA search complete"
                );
                Ok(search_result.issues)
            }
            StatusCode::UNAUTHORIZED => Err(crate::AllBeadsError::Integration(
                "JIRA authentication failed".to_string(),
            )),
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = response
                    .headers()
                    .get("Retry-After")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60);
                Err(crate::AllBeadsError::Integration(format!(
                    "Rate limited, retry after {} seconds",
                    retry_after
                )))
            }
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "JIRA API error: HTTP {}: {}",
                    status, error_body
                )))
            }
        }
    }

    /// Get a single issue by key
    pub async fn get_issue(&self, key: &str) -> Result<JiraIssue> {
        let url = format!("{}/issue/{}", self.base_url, key);

        debug!(key = %key, "Fetching JIRA issue");

        let mut request = self.client.get(&url).query(&[(
            "fields",
            "summary,description,issuetype,status,priority,labels,assignee,reporter,updated,created",
        )]);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.timeout(GET_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::OK => Ok(response.json().await?),
            StatusCode::NOT_FOUND => Err(crate::AllBeadsError::Integration(format!(
                "JIRA issue not found: {}",
                key
            ))),
            StatusCode::UNAUTHORIZED => Err(crate::AllBeadsError::Integration(
                "JIRA authentication failed".to_string(),
            )),
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "JIRA API error: HTTP {}: {}",
                    status, error_body
                )))
            }
        }
    }

    /// Get available transitions for an issue
    pub async fn get_transitions(&self, key: &str) -> Result<Vec<JiraTransition>> {
        let url = format!("{}/issue/{}/transitions", self.base_url, key);

        let mut request = self.client.get(&url);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.timeout(GET_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::OK => {
                let result: JiraTransitionsResponse = response.json().await?;
                Ok(result.transitions)
            }
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "JIRA API error: HTTP {}: {}",
                    status, error_body
                )))
            }
        }
    }

    /// Transition an issue to a new status
    pub async fn transition_issue(&self, key: &str, transition_id: &str) -> Result<()> {
        let url = format!("{}/issue/{}/transitions", self.base_url, key);

        let body = JiraTransitionRequest {
            transition: JiraTransitionId {
                id: transition_id.to_string(),
            },
        };

        info!(key = %key, transition_id = %transition_id, "Transitioning JIRA issue");

        let mut request = self.client.post(&url).json(&body);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.timeout(WRITE_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::NO_CONTENT | StatusCode::OK => Ok(()),
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "JIRA transition failed: HTTP {}: {}",
                    status, error_body
                )))
            }
        }
    }

    /// Add a comment to an issue
    pub async fn add_comment(&self, key: &str, comment: &str) -> Result<JiraComment> {
        let url = format!("{}/issue/{}/comment", self.base_url, key);

        let body = JiraCommentCreate {
            body: comment.to_string(),
        };

        info!(key = %key, "Adding comment to JIRA issue");

        let mut request = self.client.post(&url).json(&body);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.timeout(WRITE_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::CREATED | StatusCode::OK => Ok(response.json().await?),
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "JIRA comment failed: HTTP {}: {}",
                    status, error_body
                )))
            }
        }
    }

    /// Sanitize a label for safe use in JQL queries.
    /// Only allows alphanumeric characters, hyphens, underscores, and spaces.
    fn sanitize_label(label: &str) -> String {
        label
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == ' ')
            .collect()
    }

    /// Pull issues from JIRA that match the specified label
    pub async fn pull_agent_issues(&self, label: &str) -> Result<Vec<JiraIssue>> {
        // Sanitize label to prevent JQL injection
        let safe_label = Self::sanitize_label(label);
        if safe_label.is_empty() {
            return Err(crate::AllBeadsError::Integration(
                "Invalid label: must contain at least one alphanumeric character".to_string(),
            ));
        }
        let jql = format!(
            "project = {} AND labels = \"{}\" AND status != Done ORDER BY updated DESC",
            self.config.project, safe_label
        );
        self.search(&jql, 100).await
    }

    /// Convert a JIRA issue to a ShadowBead
    pub fn issue_to_shadow_bead(&self, issue: &JiraIssue) -> ShadowBead {
        let priority = issue
            .fields
            .priority
            .as_ref()
            .map(|p| match p.name.to_lowercase().as_str() {
                "highest" | "blocker" => 0u8,
                "high" | "critical" => 1,
                "medium" | "normal" => 2,
                "low" | "minor" => 3,
                _ => 4,
            })
            .unwrap_or(2);

        let status = issue
            .fields
            .status
            .status_category
            .as_ref()
            .map(|cat| match cat.key.as_str() {
                "new" => "open",
                "indeterminate" => "in_progress",
                "done" => "closed",
                _ => "open",
            })
            .unwrap_or("open");

        let issue_type = match issue.fields.issue_type.name.to_lowercase().as_str() {
            "bug" => "bug",
            "epic" => "epic",
            "story" | "task" | "sub-task" => "task",
            _ => "feature",
        };

        // Use the builder pattern
        ShadowBead::external(
            BeadId::new(&issue.key),
            issue.fields.summary.clone(),
            format!("jira:{}", issue.key),
        )
        .with_status(status)
        .with_priority(priority)
        .with_issue_type(issue_type)
        .with_description(issue.fields.description.clone().unwrap_or_default())
        .with_external_ref(format!("jira:{}", issue.key))
        .build()
    }

    /// Sync a local bead status to JIRA
    pub async fn sync_bead_to_jira(&self, bead: &Bead, jira_key: &str) -> Result<JiraSyncResult> {
        let issue = match self.get_issue(jira_key).await {
            Ok(i) => i,
            Err(e) => {
                return Ok(JiraSyncResult {
                    jira_key: jira_key.to_string(),
                    bead_id: Some(bead.id.clone()),
                    action: JiraSyncAction::Error,
                    error: Some(e.to_string()),
                });
            }
        };

        // Map bead status to JIRA target status
        let target_status = match bead.status {
            crate::graph::Status::Closed => "Done",
            crate::graph::Status::InProgress => "In Progress",
            _ => {
                return Ok(JiraSyncResult {
                    jira_key: jira_key.to_string(),
                    bead_id: Some(bead.id.clone()),
                    action: JiraSyncAction::NoChange,
                    error: None,
                });
            }
        };

        if issue.fields.status.name == target_status {
            return Ok(JiraSyncResult {
                jira_key: jira_key.to_string(),
                bead_id: Some(bead.id.clone()),
                action: JiraSyncAction::NoChange,
                error: None,
            });
        }

        let transitions = self.get_transitions(jira_key).await?;
        let transition = transitions.iter().find(|t| t.to.name == target_status);

        match transition {
            Some(t) => {
                self.transition_issue(jira_key, &t.id).await?;

                if bead.status == crate::graph::Status::Closed {
                    let comment = format!(
                        "Issue completed by AllBeads agent.\n\nResolution: {}",
                        bead.notes.as_deref().unwrap_or("Completed")
                    );
                    if let Err(e) = self.add_comment(jira_key, &comment).await {
                        warn!(key = %jira_key, error = %e, "Failed to add completion comment");
                    }
                }

                Ok(JiraSyncResult {
                    jira_key: jira_key.to_string(),
                    bead_id: Some(bead.id.clone()),
                    action: JiraSyncAction::UpdatedJira,
                    error: None,
                })
            }
            None => {
                warn!(
                    key = %jira_key,
                    target = %target_status,
                    available = ?transitions.iter().map(|t| &t.to.name).collect::<Vec<_>>(),
                    "No transition available to target status"
                );
                Ok(JiraSyncResult {
                    jira_key: jira_key.to_string(),
                    bead_id: Some(bead.id.clone()),
                    action: JiraSyncAction::Skipped,
                    error: Some(format!("No transition to: {}", target_status)),
                })
            }
        }
    }

    // ============ Version/Milestone Operations ============

    /// List all versions for the configured project
    pub async fn list_versions(&self) -> Result<Vec<JiraVersion>> {
        let url = format!("{}/project/{}/versions", self.base_url, self.project());

        debug!(project = %self.project(), "Listing JIRA versions");

        let mut request = self.client.get(&url);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.timeout(GET_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::OK => {
                let versions: Vec<JiraVersion> = response.json().await?;
                info!(count = versions.len(), "Listed JIRA versions");
                Ok(versions)
            }
            StatusCode::UNAUTHORIZED => Err(crate::AllBeadsError::Integration(
                "JIRA authentication failed".to_string(),
            )),
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "JIRA API error listing versions: HTTP {}: {}",
                    status, error_body
                )))
            }
        }
    }

    /// Get a specific version by ID
    pub async fn get_version(&self, version_id: &str) -> Result<JiraVersion> {
        let url = format!("{}/version/{}", self.base_url, version_id);

        debug!(version_id = %version_id, "Fetching JIRA version");

        let mut request = self.client.get(&url);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.timeout(GET_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::OK => Ok(response.json().await?),
            StatusCode::NOT_FOUND => Err(crate::AllBeadsError::Integration(format!(
                "JIRA version not found: {}",
                version_id
            ))),
            StatusCode::UNAUTHORIZED => Err(crate::AllBeadsError::Integration(
                "JIRA authentication failed".to_string(),
            )),
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "JIRA API error getting version {}: HTTP {}: {}",
                    version_id, status, error_body
                )))
            }
        }
    }

    /// Create a new version in the project
    pub async fn create_version(&self, request: &JiraVersionRequest) -> Result<JiraVersion> {
        let url = format!("{}/version", self.base_url);

        info!(name = %request.name, "Creating JIRA version");

        let mut req = self.client.post(&url).json(request);
        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.timeout(WRITE_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::CREATED | StatusCode::OK => {
                let version: JiraVersion = response.json().await?;
                info!(version_id = %version.id, name = %version.name, "Created JIRA version");
                Ok(version)
            }
            StatusCode::UNAUTHORIZED => Err(crate::AllBeadsError::Integration(
                "JIRA authentication failed".to_string(),
            )),
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "JIRA API error creating version: HTTP {}: {}",
                    status, error_body
                )))
            }
        }
    }

    /// Update an existing version
    pub async fn update_version(
        &self,
        version_id: &str,
        request: &JiraVersionRequest,
    ) -> Result<JiraVersion> {
        let url = format!("{}/version/{}", self.base_url, version_id);

        info!(version_id = %version_id, name = %request.name, "Updating JIRA version");

        let mut req = self.client.put(&url).json(request);
        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.timeout(WRITE_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::OK => {
                let version: JiraVersion = response.json().await?;
                info!(version_id = %version.id, name = %version.name, "Updated JIRA version");
                Ok(version)
            }
            StatusCode::NOT_FOUND => Err(crate::AllBeadsError::Integration(format!(
                "JIRA version not found: {}",
                version_id
            ))),
            StatusCode::UNAUTHORIZED => Err(crate::AllBeadsError::Integration(
                "JIRA authentication failed".to_string(),
            )),
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "JIRA API error updating version {}: HTTP {}: {}",
                    version_id, status, error_body
                )))
            }
        }
    }

    /// Delete a version
    pub async fn delete_version(&self, version_id: &str) -> Result<()> {
        let url = format!("{}/version/{}", self.base_url, version_id);

        info!(version_id = %version_id, "Deleting JIRA version");

        let mut request = self.client.delete(&url);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.timeout(WRITE_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::NO_CONTENT | StatusCode::OK => {
                info!(version_id = %version_id, "Deleted JIRA version");
                Ok(())
            }
            StatusCode::NOT_FOUND => Err(crate::AllBeadsError::Integration(format!(
                "JIRA version not found: {}",
                version_id
            ))),
            StatusCode::UNAUTHORIZED => Err(crate::AllBeadsError::Integration(
                "JIRA authentication failed".to_string(),
            )),
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "JIRA API error deleting version {}: HTTP {}: {}",
                    version_id, status, error_body
                )))
            }
        }
    }

    /// Assign an issue to a version (fix version)
    pub async fn assign_issue_to_version(&self, issue_key: &str, version_id: &str) -> Result<()> {
        let url = format!("{}/issue/{}", self.base_url, issue_key);

        let body = serde_json::json!({
            "update": {
                "fixVersions": [
                    { "add": { "id": version_id } }
                ]
            }
        });

        info!(issue = %issue_key, version_id = %version_id, "Assigning issue to JIRA version");

        let mut request = self.client.put(&url).json(&body);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.timeout(WRITE_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::NO_CONTENT | StatusCode::OK => {
                info!(issue = %issue_key, version_id = %version_id, "Assigned issue to JIRA version");
                Ok(())
            }
            StatusCode::NOT_FOUND => Err(crate::AllBeadsError::Integration(format!(
                "JIRA issue not found: {}",
                issue_key
            ))),
            StatusCode::UNAUTHORIZED => Err(crate::AllBeadsError::Integration(
                "JIRA authentication failed".to_string(),
            )),
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "JIRA API error assigning issue {} to version {}: HTTP {}: {}",
                    issue_key, version_id, status, error_body
                )))
            }
        }
    }

    /// Remove an issue from a version
    pub async fn remove_issue_from_version(&self, issue_key: &str, version_id: &str) -> Result<()> {
        let url = format!("{}/issue/{}", self.base_url, issue_key);

        let body = serde_json::json!({
            "update": {
                "fixVersions": [
                    { "remove": { "id": version_id } }
                ]
            }
        });

        info!(issue = %issue_key, version_id = %version_id, "Removing issue from JIRA version");

        let mut request = self.client.put(&url).json(&body);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.timeout(WRITE_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::NO_CONTENT | StatusCode::OK => {
                info!(issue = %issue_key, version_id = %version_id, "Removed issue from JIRA version");
                Ok(())
            }
            StatusCode::NOT_FOUND => Err(crate::AllBeadsError::Integration(format!(
                "JIRA issue not found: {}",
                issue_key
            ))),
            StatusCode::UNAUTHORIZED => Err(crate::AllBeadsError::Integration(
                "JIRA authentication failed".to_string(),
            )),
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "JIRA API error removing issue {} from version {}: HTTP {}: {}",
                    issue_key, version_id, status, error_body
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> JiraIntegration {
        JiraIntegration {
            url: "https://jira.example.com".to_string(),
            project: "TEST".to_string(),
            token_env: Some("JIRA_TOKEN".to_string()),
        }
    }

    #[test]
    fn test_adapter_creation() {
        let config = test_config();
        let adapter = JiraAdapter::new(config).expect("Failed to create adapter");
        assert_eq!(adapter.project(), "TEST");
        assert!(adapter.base_url.contains("jira.example.com"));
    }

    #[test]
    fn test_label_sanitization() {
        // Valid labels pass through
        assert_eq!(JiraAdapter::sanitize_label("ai-agent"), "ai-agent");
        assert_eq!(JiraAdapter::sanitize_label("my_label"), "my_label");
        assert_eq!(JiraAdapter::sanitize_label("label 123"), "label 123");

        // Injection attempts are sanitized (hyphens allowed, quotes/equals removed)
        assert_eq!(JiraAdapter::sanitize_label("\" OR 1=1 --"), " OR 11 --");
        assert_eq!(
            JiraAdapter::sanitize_label("label\" AND project = OTHER"),
            "label AND project  OTHER"
        );
    }

    #[test]
    fn test_issue_to_shadow_bead() {
        let config = test_config();
        let adapter = JiraAdapter::new(config).expect("Failed to create adapter");

        let issue = JiraIssue {
            key: "TEST-123".to_string(),
            id: "12345".to_string(),
            fields: JiraFields {
                summary: "Test issue".to_string(),
                description: Some("Test description".to_string()),
                issue_type: JiraIssueType {
                    name: "Bug".to_string(),
                    id: None,
                },
                status: JiraStatus {
                    name: "To Do".to_string(),
                    id: None,
                    status_category: Some(JiraStatusCategory {
                        key: "new".to_string(),
                        name: "To Do".to_string(),
                    }),
                },
                priority: Some(JiraPriority {
                    name: "High".to_string(),
                    id: None,
                }),
                labels: vec!["ai-agent".to_string()],
                assignee: None,
                reporter: None,
                updated: None,
                created: None,
            },
        };

        let shadow = adapter.issue_to_shadow_bead(&issue);
        assert_eq!(shadow.id.as_str(), "TEST-123");
        assert_eq!(shadow.summary, "Test issue");
    }
}
