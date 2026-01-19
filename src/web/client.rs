//! Web API client for AllBeads web platform

use crate::config::AllBeadsConfig;
use crate::graph::Bead;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Web API client for AllBeads
#[derive(Debug)]
pub struct WebClient {
    host: String,
    token: String,
    client: reqwest::Client,
}

/// Statistics response from /api/beads/stats
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsResponse {
    pub total: usize,
    pub by_status: HashMap<String, usize>,
    pub by_priority: HashMap<String, usize>,
    pub top_repos: Vec<RepoStats>,
    pub top_authors: Vec<AuthorStats>,
}

/// Repository statistics
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoStats {
    pub id: String,
    pub name: String,
    pub project_id: Option<String>,
    pub bead_count: usize,
}

/// Author statistics
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorStats {
    pub id: String,
    pub name: String,
    pub image: Option<String>,
    pub bead_count: usize,
}

/// Comment from web API
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: String,
    pub bead_id: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    pub author: Option<CommentAuthor>,
}

/// Comment author
#[derive(Debug, Deserialize)]
pub struct CommentAuthor {
    pub id: String,
    pub name: Option<String>,
    pub login: Option<String>,
}

/// Repository from web API
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub id: String,
    pub name: String,
    pub remote_url: String,
    pub context_name: Option<String>,
    pub default_branch: Option<String>,
    pub sync_status: Option<String>,
    pub project_id: String,
    pub project: Option<RepositoryProject>,
    pub bead_count: Option<usize>,
}

/// Project info nested in repository
#[derive(Debug, Clone, Deserialize)]
pub struct RepositoryProject {
    pub name: String,
    pub slug: String,
}

/// Import request for /api/beads/import
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportRequest {
    repo_id: String,
    beads: Vec<ImportBead>,
    full_sync: bool,
}

/// Bead format for import (matches CLI JSONL)
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ImportBead {
    id: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    status: String,
    priority: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    issue_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    assignee: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    labels: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    epic_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    depends_on: Vec<String>,
}

/// Import response from /api/beads/import
#[derive(Debug, Deserialize)]
pub struct ImportResponse {
    pub success: bool,
    pub stats: ImportStats,
    pub total: usize,
}

/// Import statistics
#[derive(Debug, Deserialize)]
pub struct ImportStats {
    pub created: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub deleted: usize,
    #[serde(default)]
    pub errors: Vec<String>,
}

/// Error response from API
#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
}

impl WebClient {
    /// Create a new web client from config
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
        })
    }

    /// Create with explicit host and token
    pub fn new(host: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            token: token.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Get bead statistics from web API
    pub async fn get_stats(&self) -> Result<StatsResponse> {
        let url = format!("{}/api/beads/stats", self.host);

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
                anyhow::anyhow!("Failed to get stats ({}): {}", status, error.error).into(),
            );
        }

        let stats: StatsResponse = response.json().await?;
        Ok(stats)
    }

    /// Get comments for a bead
    pub async fn get_comments(&self, bead_id: &str) -> Result<Vec<Comment>> {
        let url = format!("{}/api/beads/{}/comments", self.host, bead_id);

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
                anyhow::anyhow!("Failed to get comments ({}): {}", status, error.error).into(),
            );
        }

        let comments: Vec<Comment> = response.json().await?;
        Ok(comments)
    }

    /// Add a comment to a bead
    pub async fn add_comment(&self, bead_id: &str, content: &str) -> Result<Comment> {
        let url = format!("{}/api/beads/{}/comments", self.host, bead_id);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "content": content }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error: ErrorResponse = response.json().await.unwrap_or(ErrorResponse {
                error: "Unknown error".to_string(),
            });
            return Err(
                anyhow::anyhow!("Failed to add comment ({}): {}", status, error.error).into(),
            );
        }

        let comment: Comment = response.json().await?;
        Ok(comment)
    }

    /// Import beads to web platform
    pub async fn import_beads(
        &self,
        repo_id: &str,
        beads: &[Bead],
        full_sync: bool,
    ) -> Result<ImportResponse> {
        let url = format!("{}/api/beads/import", self.host);

        // Convert beads to import format
        let import_beads: Vec<ImportBead> = beads
            .iter()
            .map(|b| ImportBead {
                id: b.id.as_str().to_string(),
                title: b.title.clone(),
                description: b.description.clone(),
                status: format!("{:?}", b.status).to_lowercase(),
                priority: u8::from(b.priority) as i32,
                issue_type: Some(format!("{:?}", b.issue_type).to_lowercase()),
                created_at: Some(b.created_at.clone()),
                updated_at: Some(b.updated_at.clone()),
                created_by: if b.created_by.is_empty() {
                    None
                } else {
                    Some(b.created_by.clone())
                },
                assignee: b.assignee.clone(),
                labels: b.labels.iter().cloned().collect(),
                epic_id: None, // No epic_id field in Bead struct
                depends_on: b
                    .dependencies
                    .iter()
                    .map(|d| d.as_str().to_string())
                    .collect(),
            })
            .collect();

        let request = ImportRequest {
            repo_id: repo_id.to_string(),
            beads: import_beads,
            full_sync,
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
                anyhow::anyhow!("Failed to import beads ({}): {}", status, error.error).into(),
            );
        }

        let result: ImportResponse = response.json().await?;
        Ok(result)
    }

    /// List repositories from web API
    pub async fn list_repositories(&self) -> Result<Vec<Repository>> {
        let url = format!("{}/api/repositories", self.host);

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
                "Failed to list repositories ({}): {}",
                status,
                error.error
            )
            .into());
        }

        let repos: Vec<Repository> = response.json().await?;
        Ok(repos)
    }

    /// Find a repository by context name or remote URL
    pub async fn find_repository(
        &self,
        context_name: Option<&str>,
        remote_url: Option<&str>,
    ) -> Result<Option<Repository>> {
        let repos = self.list_repositories().await?;

        // First try exact context name match
        if let Some(name) = context_name {
            if let Some(repo) = repos
                .iter()
                .find(|r| r.context_name.as_deref() == Some(name))
            {
                return Ok(Some(repo.clone()));
            }
        }

        // Then try remote URL match (normalize URLs for comparison)
        if let Some(url) = remote_url {
            let normalized = normalize_git_url(url);
            if let Some(repo) = repos
                .iter()
                .find(|r| normalize_git_url(&r.remote_url) == normalized)
            {
                return Ok(Some(repo.clone()));
            }
        }

        Ok(None)
    }
}

/// Normalize a git URL for comparison (strip .git suffix, normalize protocol)
fn normalize_git_url(url: &str) -> String {
    let url = url.trim();
    let url = url.strip_suffix(".git").unwrap_or(url);

    // Convert SSH URLs to HTTPS-like format for comparison
    if url.starts_with("git@") {
        // git@github.com:owner/repo -> github.com/owner/repo
        url.strip_prefix("git@")
            .unwrap_or(url)
            .replace(':', "/")
            .to_lowercase()
    } else if url.starts_with("https://") || url.starts_with("http://") {
        // https://github.com/owner/repo -> github.com/owner/repo
        url.split("://").nth(1).unwrap_or(url).to_lowercase()
    } else {
        url.to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_client_creation() {
        let client = WebClient::new("https://allbeads.co", "test_token");
        assert_eq!(client.host, "https://allbeads.co");
    }

    #[test]
    fn test_normalize_git_url() {
        // SSH URLs
        assert_eq!(
            normalize_git_url("git@github.com:owner/repo.git"),
            "github.com/owner/repo"
        );
        assert_eq!(
            normalize_git_url("git@github.com:owner/repo"),
            "github.com/owner/repo"
        );

        // HTTPS URLs
        assert_eq!(
            normalize_git_url("https://github.com/owner/repo.git"),
            "github.com/owner/repo"
        );
        assert_eq!(
            normalize_git_url("https://github.com/owner/repo"),
            "github.com/owner/repo"
        );

        // Case insensitive
        assert_eq!(
            normalize_git_url("https://GitHub.com/Owner/Repo"),
            "github.com/owner/repo"
        );
    }
}
