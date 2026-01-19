//! GitHub Issues Integration Adapter
//!
//! Bi-directional sync between AllBeads and GitHub Issues using the GraphQL API.

use crate::config::GitHubIntegration;
use crate::graph::{Bead, BeadId, ShadowBead};
use crate::Result;
use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Per-request timeout for GraphQL queries (can return large result sets)
const GRAPHQL_TIMEOUT: Duration = Duration::from_secs(30);
/// Per-request timeout for single issue fetches
const GET_TIMEOUT: Duration = Duration::from_secs(10);
/// Per-request timeout for create/update operations
const WRITE_TIMEOUT: Duration = Duration::from_secs(15);

/// GitHub API client for bi-directional sync
pub struct GitHubAdapter {
    client: Client,
    config: GitHubIntegration,
    rest_base_url: String,
    graphql_url: String,
    auth_token: Option<String>,
}

/// GitHub issue (REST API format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIssue {
    pub number: u64,
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub node_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub body: Option<String>,
    pub state: String,
    #[serde(default)]
    pub labels: Vec<GitHubLabel>,
    #[serde(default)]
    pub assignees: Vec<GitHubUser>,
    #[serde(default)]
    pub user: Option<GitHubUser>,
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub closed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubLabel {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubComment {
    pub id: u64,
    pub body: String,
    pub user: GitHubUser,
    pub created_at: String,
    pub updated_at: String,
}

/// GraphQL response wrapper
#[derive(Debug, Clone, Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Clone, Deserialize)]
struct GraphQLError {
    message: String,
}

/// Search issues data
#[derive(Debug, Clone, Deserialize)]
struct SearchIssuesData {
    search: SearchConnection,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchConnection {
    #[serde(rename = "issueCount")]
    issue_count: u32,
    edges: Vec<SearchEdge>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchEdge {
    node: IssueNode,
}

/// Issue node from GraphQL
#[derive(Debug, Clone, Deserialize)]
pub struct IssueNode {
    pub id: String,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub url: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "closedAt")]
    pub closed_at: Option<String>,
    pub labels: LabelsConnection,
    pub assignees: AssigneesConnection,
    pub author: Option<ActorNode>,
    pub repository: RepositoryNode,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LabelsConnection {
    pub nodes: Vec<LabelNode>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LabelNode {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssigneesConnection {
    pub nodes: Vec<ActorNode>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActorNode {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RepositoryNode {
    pub name: String,
    #[serde(rename = "nameWithOwner")]
    pub name_with_owner: String,
    pub owner: OwnerNode,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OwnerNode {
    pub login: String,
}

/// Issue creation request
#[derive(Debug, Clone, Serialize)]
pub struct CreateIssueRequest {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignees: Option<Vec<String>>,
}

/// Issue update request
#[derive(Debug, Clone, Serialize)]
pub struct UpdateIssueRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
struct CreateCommentRequest {
    body: String,
}

/// Sync result
#[derive(Debug, Clone)]
pub struct GitHubSyncResult {
    pub issue_number: u64,
    pub repo: String,
    pub bead_id: Option<BeadId>,
    pub action: GitHubSyncAction,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitHubSyncAction {
    CreatedBead,
    UpdatedBead,
    UpdatedGitHub,
    CreatedIssue,
    NoChange,
    Skipped,
    Error,
}

#[derive(Debug, thiserror::Error)]
pub enum GitHubError {
    #[error("GitHub API error: {0}")]
    Api(String),
    #[error("GraphQL error: {0}")]
    GraphQL(String),
    #[error("Authentication failed: {0}")]
    Auth(String),
    #[error("Issue not found: {0}")]
    NotFound(String),
    #[error("Rate limited")]
    RateLimited,
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

#[derive(Debug, Default, Clone)]
pub struct GitHubSyncStats {
    pub issues_pulled: u32,
    pub created: u32,
    pub updated: u32,
    pub github_updated: u32,
    pub issues_created: u32,
    pub skipped: u32,
    pub errors: u32,
}

impl GitHubAdapter {
    /// Create a new GitHub adapter
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn new(config: GitHubIntegration) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .default_headers({
                let mut headers = header::HeaderMap::new();
                headers.insert(
                    header::USER_AGENT,
                    header::HeaderValue::from_static("allbeads/1.0"),
                );
                headers.insert(
                    header::ACCEPT,
                    header::HeaderValue::from_static("application/vnd.github.v3+json"),
                );
                headers
            })
            .build()?; // reqwest::Error converts to AllBeadsError::Http via #[from]

        let base_url = config.url.trim_end_matches('/');
        let (rest_base_url, graphql_url) =
            if base_url.contains("github.com") && !base_url.contains("api.github.com") {
                (
                    "https://api.github.com".to_string(),
                    "https://api.github.com/graphql".to_string(),
                )
            } else if base_url.contains("api.github.com") {
                (
                    base_url.to_string(),
                    "https://api.github.com/graphql".to_string(),
                )
            } else {
                (
                    format!("{}/api/v3", base_url),
                    format!("{}/api/graphql", base_url),
                )
            };

        let auth_token = std::env::var("GITHUB_TOKEN").ok();

        Ok(Self {
            client,
            config,
            rest_base_url,
            graphql_url,
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

    pub fn owner(&self) -> &str {
        &self.config.owner
    }

    /// Map GitHub issue state to AllBeads Status
    pub fn map_github_state(&self, state: &str) -> crate::graph::Status {
        match state.to_uppercase().as_str() {
            "CLOSED" => crate::graph::Status::Closed,
            "OPEN" => crate::graph::Status::Open,
            _ => crate::graph::Status::Open,
        }
    }

    /// Execute a GraphQL query
    async fn graphql<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<T> {
        let body = serde_json::json!({
            "query": query,
            "variables": variables,
        });

        let mut request = self.client.post(&self.graphql_url).json(&body);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.timeout(GRAPHQL_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::OK => {
                let result: GraphQLResponse<T> = response.json().await?;
                if let Some(errors) = result.errors {
                    let error_msg = errors
                        .iter()
                        .map(|e| e.message.clone())
                        .collect::<Vec<_>>()
                        .join("; ");
                    return Err(crate::AllBeadsError::Integration(format!(
                        "GraphQL error: {}",
                        error_msg
                    )));
                }
                result.data.ok_or_else(|| {
                    crate::AllBeadsError::Integration("No data in GraphQL response".to_string())
                })
            }
            StatusCode::UNAUTHORIZED => Err(crate::AllBeadsError::Integration(
                "GitHub authentication failed".to_string(),
            )),
            StatusCode::FORBIDDEN => Err(crate::AllBeadsError::Integration(
                "GitHub API forbidden (rate limit?)".to_string(),
            )),
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "GitHub API error: HTTP {}: {}",
                    status, error_body
                )))
            }
        }
    }

    /// Search for issues using GraphQL
    pub async fn search_issues(&self, query: &str, first: u32) -> Result<Vec<IssueNode>> {
        let graphql_query = r#"
            query($query: String!, $first: Int!) {
                search(query: $query, type: ISSUE, first: $first) {
                    issueCount
                    edges {
                        node {
                            ... on Issue {
                                id
                                number
                                title
                                body
                                state
                                url
                                createdAt
                                updatedAt
                                closedAt
                                labels(first: 10) {
                                    nodes { name color }
                                }
                                assignees(first: 5) {
                                    nodes { login }
                                }
                                author { login }
                                repository {
                                    name
                                    nameWithOwner
                                    owner { login }
                                }
                            }
                        }
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "query": query,
            "first": first,
        });

        debug!(query = %query, first = %first, "Searching GitHub issues");

        let data: SearchIssuesData = self.graphql(graphql_query, variables).await?;

        info!(
            count = data.search.issue_count,
            returned = data.search.edges.len(),
            "GitHub search complete"
        );

        Ok(data.search.edges.into_iter().map(|e| e.node).collect())
    }

    /// Get a single issue by number (REST API)
    pub async fn get_issue(&self, repo: &str, number: u64) -> Result<GitHubIssue> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}",
            self.rest_base_url, self.config.owner, repo, number
        );

        debug!(repo = %repo, number = %number, "Fetching GitHub issue");

        let mut request = self.client.get(&url);
        if let Some(ref token) = self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.timeout(GET_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::OK => Ok(response.json().await?),
            StatusCode::NOT_FOUND => Err(crate::AllBeadsError::Integration(format!(
                "Issue not found: {}#{}",
                repo, number
            ))),
            StatusCode::UNAUTHORIZED => Err(crate::AllBeadsError::Integration(
                "GitHub authentication failed".to_string(),
            )),
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "GitHub API error: HTTP {}: {}",
                    status, error_body
                )))
            }
        }
    }

    /// Create a new issue (REST API)
    pub async fn create_issue(
        &self,
        repo: &str,
        request: CreateIssueRequest,
    ) -> Result<GitHubIssue> {
        let url = format!(
            "{}/repos/{}/{}/issues",
            self.rest_base_url, self.config.owner, repo
        );

        info!(repo = %repo, title = %request.title, "Creating GitHub issue");

        let mut http_request = self.client.post(&url).json(&request);
        if let Some(ref token) = self.auth_token {
            http_request = http_request.bearer_auth(token);
        }

        let response = http_request.timeout(WRITE_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::CREATED => {
                let issue: GitHubIssue = response.json().await?;
                info!(number = issue.number, "GitHub issue created");
                Ok(issue)
            }
            StatusCode::UNAUTHORIZED => Err(crate::AllBeadsError::Integration(
                "GitHub authentication failed".to_string(),
            )),
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "GitHub create issue failed: HTTP {}: {}",
                    status, error_body
                )))
            }
        }
    }

    /// Update an existing issue (REST API)
    pub async fn update_issue(
        &self,
        repo: &str,
        number: u64,
        request: UpdateIssueRequest,
    ) -> Result<GitHubIssue> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}",
            self.rest_base_url, self.config.owner, repo, number
        );

        info!(repo = %repo, number = %number, "Updating GitHub issue");

        let mut http_request = self.client.patch(&url).json(&request);
        if let Some(ref token) = self.auth_token {
            http_request = http_request.bearer_auth(token);
        }

        let response = http_request.timeout(WRITE_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::OK => Ok(response.json().await?),
            StatusCode::NOT_FOUND => Err(crate::AllBeadsError::Integration(format!(
                "Issue not found: {}#{}",
                repo, number
            ))),
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "GitHub update failed: HTTP {}: {}",
                    status, error_body
                )))
            }
        }
    }

    /// Add a comment to an issue (REST API)
    pub async fn add_comment(&self, repo: &str, number: u64, body: &str) -> Result<GitHubComment> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}/comments",
            self.rest_base_url, self.config.owner, repo, number
        );

        info!(repo = %repo, number = %number, "Adding comment to GitHub issue");

        let request_body = CreateCommentRequest {
            body: body.to_string(),
        };

        let mut http_request = self.client.post(&url).json(&request_body);
        if let Some(ref token) = self.auth_token {
            http_request = http_request.bearer_auth(token);
        }

        let response = http_request.timeout(WRITE_TIMEOUT).send().await?;

        match response.status() {
            StatusCode::CREATED => Ok(response.json().await?),
            status => {
                let error_body = response.text().await.unwrap_or_default();
                Err(crate::AllBeadsError::Integration(format!(
                    "GitHub comment failed: HTTP {}: {}",
                    status, error_body
                )))
            }
        }
    }

    /// Sanitize a label for safe use in GitHub search queries.
    /// Only allows alphanumeric characters, hyphens, underscores, and spaces.
    fn sanitize_label(label: &str) -> String {
        label
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == ' ')
            .collect()
    }

    /// Pull issues from GitHub that have the specified label
    pub async fn pull_agent_issues(&self, label: &str) -> Result<Vec<IssueNode>> {
        // Sanitize label to prevent search query injection
        let safe_label = Self::sanitize_label(label);
        if safe_label.is_empty() {
            return Err(crate::AllBeadsError::Integration(
                "Invalid label: must contain at least one alphanumeric character".to_string(),
            ));
        }
        let query = format!(
            "org:{} is:issue is:open label:{}",
            self.config.owner, safe_label
        );
        self.search_issues(&query, 100).await
    }

    /// Convert a GitHub issue node to a ShadowBead
    pub fn issue_to_shadow_bead(&self, issue: &IssueNode) -> ShadowBead {
        let priority = issue
            .labels
            .nodes
            .iter()
            .find_map(|label| {
                let name = label.name.to_lowercase();
                if name.starts_with("p0") || name == "critical" {
                    Some(0u8)
                } else if name.starts_with("p1") || name == "high" {
                    Some(1)
                } else if name.starts_with("p2") || name == "medium" {
                    Some(2)
                } else if name.starts_with("p3") || name == "low" {
                    Some(3)
                } else if name.starts_with("p4") || name == "backlog" {
                    Some(4)
                } else {
                    None
                }
            })
            .unwrap_or(2);

        let status = match issue.state.to_uppercase().as_str() {
            "OPEN" => "open",
            "CLOSED" => "closed",
            _ => "open",
        };

        let issue_type = issue
            .labels
            .nodes
            .iter()
            .find_map(|label| {
                let name = label.name.to_lowercase();
                if name == "bug" {
                    Some("bug")
                } else if name == "enhancement" || name == "feature" {
                    Some("feature")
                } else if name == "epic" {
                    Some("epic")
                } else {
                    None
                }
            })
            .unwrap_or("task");

        let external_ref = format!(
            "github:{}#{}",
            issue.repository.name_with_owner, issue.number
        );

        ShadowBead::external(
            BeadId::new(format!("gh-{}", issue.number)),
            issue.title.clone(),
            issue.url.clone(),
        )
        .with_status(status)
        .with_priority(priority)
        .with_issue_type(issue_type)
        .with_description(issue.body.clone().unwrap_or_default())
        .with_external_ref(external_ref)
        .build()
    }

    /// Sync a local bead status to GitHub
    pub async fn sync_bead_to_github(
        &self,
        bead: &Bead,
        repo: &str,
        issue_number: u64,
    ) -> Result<GitHubSyncResult> {
        let issue = match self.get_issue(repo, issue_number).await {
            Ok(i) => i,
            Err(e) => {
                return Ok(GitHubSyncResult {
                    issue_number,
                    repo: repo.to_string(),
                    bead_id: Some(bead.id.clone()),
                    action: GitHubSyncAction::Error,
                    error: Some(e.to_string()),
                });
            }
        };

        let target_state = match bead.status {
            crate::graph::Status::Closed => "closed",
            _ => "open",
        };

        if issue.state == target_state {
            return Ok(GitHubSyncResult {
                issue_number,
                repo: repo.to_string(),
                bead_id: Some(bead.id.clone()),
                action: GitHubSyncAction::NoChange,
                error: None,
            });
        }

        let update = UpdateIssueRequest {
            title: None,
            body: None,
            state: Some(target_state.to_string()),
            labels: None,
        };

        match self.update_issue(repo, issue_number, update).await {
            Ok(_) => {
                if bead.status == crate::graph::Status::Closed {
                    let comment = format!(
                        "Issue completed by AllBeads agent.\n\nResolution: {}",
                        bead.notes.as_deref().unwrap_or("Completed")
                    );
                    if let Err(e) = self.add_comment(repo, issue_number, &comment).await {
                        warn!(repo = %repo, number = %issue_number, error = %e, "Failed to add comment");
                    }
                }

                Ok(GitHubSyncResult {
                    issue_number,
                    repo: repo.to_string(),
                    bead_id: Some(bead.id.clone()),
                    action: GitHubSyncAction::UpdatedGitHub,
                    error: None,
                })
            }
            Err(e) => Ok(GitHubSyncResult {
                issue_number,
                repo: repo.to_string(),
                bead_id: Some(bead.id.clone()),
                action: GitHubSyncAction::Error,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Create a new GitHub issue from a Bead
    pub async fn create_issue_from_bead(
        &self,
        bead: &Bead,
        repo: &str,
    ) -> Result<GitHubSyncResult> {
        let mut labels = vec!["ai-agent".to_string()];

        let priority: u8 = bead.priority.into();
        labels.push(format!("P{}", priority));

        let type_label = match bead.issue_type {
            crate::graph::IssueType::Bug => "bug",
            crate::graph::IssueType::Feature => "enhancement",
            crate::graph::IssueType::Epic => "epic",
            _ => "task",
        };
        labels.push(type_label.to_string());

        let request = CreateIssueRequest {
            title: bead.title.clone(),
            body: bead.description.clone(),
            labels: Some(labels),
            assignees: None,
        };

        match self.create_issue(repo, request).await {
            Ok(issue) => Ok(GitHubSyncResult {
                issue_number: issue.number,
                repo: repo.to_string(),
                bead_id: Some(bead.id.clone()),
                action: GitHubSyncAction::CreatedIssue,
                error: None,
            }),
            Err(e) => Ok(GitHubSyncResult {
                issue_number: 0,
                repo: repo.to_string(),
                bead_id: Some(bead.id.clone()),
                action: GitHubSyncAction::Error,
                error: Some(e.to_string()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> GitHubIntegration {
        GitHubIntegration {
            url: "https://github.com".to_string(),
            owner: "testorg".to_string(),
            repo_pattern: None,
        }
    }

    #[test]
    fn test_adapter_creation() {
        let config = test_config();
        let adapter = GitHubAdapter::new(config).expect("Failed to create adapter");
        assert_eq!(adapter.owner(), "testorg");
        assert!(adapter.graphql_url.contains("api.github.com"));
    }

    #[test]
    fn test_enterprise_urls() {
        let config = GitHubIntegration {
            url: "https://github.ibm.com".to_string(),
            owner: "cloud-team".to_string(),
            repo_pattern: None,
        };
        let adapter = GitHubAdapter::new(config).expect("Failed to create adapter");
        assert!(adapter.rest_base_url.contains("github.ibm.com/api/v3"));
        assert!(adapter.graphql_url.contains("github.ibm.com/api/graphql"));
    }

    #[test]
    fn test_label_sanitization() {
        // Valid labels pass through
        assert_eq!(GitHubAdapter::sanitize_label("ai-agent"), "ai-agent");
        assert_eq!(GitHubAdapter::sanitize_label("my_label"), "my_label");
        assert_eq!(GitHubAdapter::sanitize_label("label 123"), "label 123");

        // Injection attempts are sanitized
        assert_eq!(GitHubAdapter::sanitize_label("\" OR label:*"), " OR label");
        assert_eq!(
            GitHubAdapter::sanitize_label("bug\" org:other"),
            "bug orgother"
        );
    }

    #[test]
    fn test_issue_to_shadow_bead() {
        let config = test_config();
        let adapter = GitHubAdapter::new(config).expect("Failed to create adapter");

        let issue = IssueNode {
            id: "MDU6SXNzdWUx".to_string(),
            number: 123,
            title: "Test issue".to_string(),
            body: Some("Test description".to_string()),
            state: "OPEN".to_string(),
            url: "https://github.com/testorg/repo/issues/123".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-02T00:00:00Z".to_string(),
            closed_at: None,
            labels: LabelsConnection {
                nodes: vec![
                    LabelNode {
                        name: "bug".to_string(),
                        color: "d73a4a".to_string(),
                    },
                    LabelNode {
                        name: "P1".to_string(),
                        color: "ff0000".to_string(),
                    },
                ],
            },
            assignees: AssigneesConnection { nodes: vec![] },
            author: Some(ActorNode {
                login: "testuser".to_string(),
            }),
            repository: RepositoryNode {
                name: "repo".to_string(),
                name_with_owner: "testorg/repo".to_string(),
                owner: OwnerNode {
                    login: "testorg".to_string(),
                },
            },
        };

        let shadow = adapter.issue_to_shadow_bead(&issue);
        assert_eq!(shadow.id.as_str(), "gh-123");
        assert_eq!(shadow.summary, "Test issue");
    }
}
