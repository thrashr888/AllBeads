//! GitHub Organization/User Scanner
//!
//! Scans GitHub user or organization to identify:
//! - Repositories not yet managed by AllBeads
//! - Repositories with agent configurations (adoption opportunities)
//! - Repository metadata for prioritization
//!
//! # Scanning Strategy
//!
//! Two modes are available:
//! 1. **Search Mode** (default, fast): Uses GitHub Search API to find agent config files
//!    across all repos in a single query per file type. Much faster for large accounts.
//! 2. **Per-Repo Mode** (thorough): Checks each repo individually via Contents API.
//!    More thorough but slower and uses more API quota.
//!
//! The scanner uses parallel batch processing (configurable concurrency) to maximize
//! throughput while respecting rate limits.

use crate::config::AllBeadsConfig;
use crate::governance::agents::AgentType;
use crate::Result;
use chrono::{DateTime, Utc};
use futures::stream::{self, StreamExt};
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;

/// Available fields for scan output
/// Basic fields are always available (no extra API calls)
/// Detailed fields require the Git Trees API (one call per repo)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScanField {
    // Basic fields (no extra API calls)
    Name,
    FullName,
    Url,
    Language,
    Stars,
    Forks,
    IsFork,
    IsArchived,
    IsPrivate,
    DaysSincePush,
    Managed,
    Priority,
    Agents,

    // Detailed fields (require Git Trees API)
    Settings,  // has_settings + hooks_count + subagent_types
    Workflows, // has_workflows + workflow_count
    Commands,  // has_commands + command_count
    Beads,     // has_beads
}

impl ScanField {
    /// Parse a field name string into a ScanField
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "name" => Some(Self::Name),
            "full_name" | "fullname" => Some(Self::FullName),
            "url" => Some(Self::Url),
            "language" | "lang" => Some(Self::Language),
            "stars" => Some(Self::Stars),
            "forks" => Some(Self::Forks),
            "is_fork" | "fork" => Some(Self::IsFork),
            "is_archived" | "archived" => Some(Self::IsArchived),
            "is_private" | "private" => Some(Self::IsPrivate),
            "days_since_push" | "days" | "activity" => Some(Self::DaysSincePush),
            "managed" => Some(Self::Managed),
            "priority" => Some(Self::Priority),
            "agents" => Some(Self::Agents),
            "settings" => Some(Self::Settings),
            "workflows" | "cicd" | "ci" => Some(Self::Workflows),
            "commands" | "cmds" => Some(Self::Commands),
            "beads" => Some(Self::Beads),
            _ => None,
        }
    }

    /// Get the CSV column name for this field
    pub fn csv_name(&self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::FullName => "full_name",
            Self::Url => "url",
            Self::Language => "language",
            Self::Stars => "stars",
            Self::Forks => "forks",
            Self::IsFork => "is_fork",
            Self::IsArchived => "is_archived",
            Self::IsPrivate => "is_private",
            Self::DaysSincePush => "days_since_push",
            Self::Managed => "managed",
            Self::Priority => "priority",
            Self::Agents => "agents",
            Self::Settings => "settings",
            Self::Workflows => "workflows",
            Self::Commands => "commands",
            Self::Beads => "beads",
        }
    }

    /// Check if this field requires the Git Trees API
    pub fn requires_detailed(&self) -> bool {
        matches!(
            self,
            Self::Settings | Self::Workflows | Self::Commands | Self::Beads
        )
    }

    /// All basic fields (no extra API calls)
    pub fn basic_fields() -> Vec<Self> {
        vec![
            Self::Name,
            Self::FullName,
            Self::Url,
            Self::Language,
            Self::Stars,
            Self::Forks,
            Self::IsFork,
            Self::IsArchived,
            Self::IsPrivate,
            Self::DaysSincePush,
            Self::Managed,
            Self::Priority,
            Self::Agents,
        ]
    }

    /// All detailed fields (require Git Trees API)
    pub fn detailed_fields() -> Vec<Self> {
        vec![Self::Settings, Self::Workflows, Self::Commands, Self::Beads]
    }

    /// All available fields
    pub fn all_fields() -> Vec<Self> {
        let mut fields = Self::basic_fields();
        fields.extend(Self::detailed_fields());
        fields
    }
}

/// A set of fields to include in scan output
#[derive(Debug, Clone)]
pub struct FieldSet {
    fields: HashSet<ScanField>,
    /// Preserve order for CSV headers
    ordered: Vec<ScanField>,
}

impl Default for FieldSet {
    fn default() -> Self {
        Self::basic()
    }
}

impl FieldSet {
    /// Create a new empty field set
    pub fn new() -> Self {
        Self {
            fields: HashSet::new(),
            ordered: Vec::new(),
        }
    }

    /// Create a field set with all basic fields
    pub fn basic() -> Self {
        let fields: HashSet<_> = ScanField::basic_fields().into_iter().collect();
        let ordered = ScanField::basic_fields();
        Self { fields, ordered }
    }

    /// Create a field set with all fields
    pub fn all() -> Self {
        let fields: HashSet<_> = ScanField::all_fields().into_iter().collect();
        let ordered = ScanField::all_fields();
        Self { fields, ordered }
    }

    /// Parse a comma-separated field list
    /// Supports shortcuts: "all", "basic", "detailed"
    pub fn parse(s: &str) -> std::result::Result<Self, String> {
        let mut set = Self::new();

        for part in s.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            match part.to_lowercase().as_str() {
                "all" => {
                    for field in ScanField::all_fields() {
                        set.add(field);
                    }
                }
                "basic" => {
                    for field in ScanField::basic_fields() {
                        set.add(field);
                    }
                }
                "detailed" => {
                    for field in ScanField::detailed_fields() {
                        set.add(field);
                    }
                }
                _ => {
                    if let Some(field) = ScanField::from_str(part) {
                        set.add(field);
                    } else {
                        return Err(format!("Unknown field: '{}'. Available fields: name, full_name, url, language, stars, forks, fork, archived, private, days, managed, priority, agents, settings, workflows, commands, beads. Shortcuts: all, basic, detailed", part));
                    }
                }
            }
        }

        if set.is_empty() {
            return Err("No fields specified".to_string());
        }

        Ok(set)
    }

    /// Add a field to the set
    pub fn add(&mut self, field: ScanField) {
        if self.fields.insert(field) {
            self.ordered.push(field);
        }
    }

    /// Check if the set contains a field
    pub fn contains(&self, field: ScanField) -> bool {
        self.fields.contains(&field)
    }

    /// Check if any field requires detailed info
    pub fn requires_detailed(&self) -> bool {
        self.fields.iter().any(|f| f.requires_detailed())
    }

    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Get the ordered list of fields
    pub fn ordered(&self) -> &[ScanField] {
        &self.ordered
    }

    /// Get the CSV header row
    pub fn csv_header(&self) -> String {
        self.ordered
            .iter()
            .map(|f| f.csv_name())
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// Metadata tuple for a scanned repository
/// (repo, last_push, created_at, days_since_push, managed)
type RepoMetadata = (
    GitHubRepo,
    Option<DateTime<Utc>>,
    DateTime<Utc>,
    Option<i64>,
    bool,
);

/// GitHub repository from REST API
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRepo {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: String,
    #[serde(default)]
    pub language: Option<String>,
    pub stargazers_count: u32,
    pub forks_count: u32,
    pub fork: bool,
    pub archived: bool,
    #[serde(default)]
    pub disabled: bool,
    pub pushed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub default_branch: String,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub private: bool,
}

/// Priority level for onboarding recommendations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnboardingPriority {
    High,   // Has agents, active
    Medium, // Active, no agents
    Low,    // Inactive or small
    Skip,   // Archived, fork, etc.
}

impl std::fmt::Display for OnboardingPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OnboardingPriority::High => write!(f, "high"),
            OnboardingPriority::Medium => write!(f, "medium"),
            OnboardingPriority::Low => write!(f, "low"),
            OnboardingPriority::Skip => write!(f, "skip"),
        }
    }
}

/// Detailed info from Git Trees API (BSICH+ features)
#[derive(Debug, Clone, Default, Serialize)]
pub struct DetailedInfo {
    /// Whether .claude/settings.json exists
    pub has_settings: bool,
    /// Whether .github/workflows directory exists
    pub has_workflows: bool,
    /// Whether .claude/commands directory exists
    pub has_commands: bool,
    /// Whether .beads directory exists
    pub has_beads: bool,
    /// Number of workflow files in .github/workflows
    pub workflow_count: usize,
    /// Number of command files in .claude/commands
    pub command_count: usize,
    /// Subagent types detected from settings.json
    pub subagent_types: Vec<String>,
    /// Number of hooks configured (from settings.json)
    pub hooks_count: usize,
    /// Number of beads issues (requires content fetch, expensive)
    pub beads_count: Option<usize>,
    /// Beads status breakdown (requires content fetch, expensive)
    pub beads_statuses: Option<HashMap<String, usize>>,
}

/// Scanned repository with AllBeads-specific metadata
#[derive(Debug, Clone, Serialize)]
pub struct ScannedRepo {
    pub name: String,
    pub full_name: String,
    pub url: String,
    pub clone_url: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub stars: u32,
    pub forks: u32,
    pub is_fork: bool,
    pub is_archived: bool,
    pub is_private: bool,
    pub last_push: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub default_branch: String,
    pub topics: Vec<String>,

    // AllBeads-specific
    pub managed: bool,
    pub detected_agents: Vec<AgentType>,
    pub onboarding_priority: OnboardingPriority,
    pub days_since_push: Option<i64>,

    // Detailed info (only populated with --detailed flag)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detailed: Option<DetailedInfo>,
}

/// Scan summary statistics
#[derive(Debug, Clone, Default, Serialize)]
pub struct ScanSummary {
    pub total_repos: usize,
    pub managed_repos: usize,
    pub unmanaged_repos: usize,
    pub high_priority: usize,
    pub medium_priority: usize,
    pub low_priority: usize,
    pub skip: usize,
    pub with_agents: usize,
    pub agent_counts: Vec<(String, usize)>,
}

/// Source of the scan
#[derive(Debug, Clone, Serialize)]
pub enum ScanSource {
    User(String),
    Organization(String),
    Repository(String),
}

impl std::fmt::Display for ScanSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanSource::User(u) => write!(f, "user:{}", u),
            ScanSource::Organization(o) => write!(f, "org:{}", o),
            ScanSource::Repository(r) => write!(f, "repo:{}", r),
        }
    }
}

/// Scan result containing all scanned repos
#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    pub timestamp: DateTime<Utc>,
    pub source: ScanSource,
    pub repositories: Vec<ScannedRepo>,
    pub summary: ScanSummary,
}

/// Scan filter options
#[derive(Debug, Clone, Default)]
pub struct ScanFilter {
    pub min_stars: Option<u32>,
    pub language: Option<String>,
    pub activity_days: Option<u32>,
    pub exclude_forks: bool,
    pub exclude_archived: bool,
    pub exclude_private: bool,
    pub topics: Vec<String>,
}

/// Scan options
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// Number of concurrent agent detection requests (default: 10)
    pub concurrency: usize,
    /// Use GitHub Search API instead of per-repo checks (faster)
    pub use_search_api: bool,
    /// Show progress output
    pub show_progress: bool,
    /// Fields to include in output (determines if detailed API calls are needed)
    pub fields: FieldSet,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            concurrency: 10,
            use_search_api: true,
            show_progress: true,
            fields: FieldSet::basic(),
        }
    }
}

/// GitHub code search result
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct SearchCodeResult {
    total_count: u32,
    items: Vec<SearchCodeItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchCodeItem {
    repository: SearchRepoRef,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchRepoRef {
    full_name: String,
}

/// GitHub Git Tree API response
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct GitTreeResponse {
    sha: String,
    url: String,
    tree: Vec<GitTreeEntry>,
    truncated: bool,
}

/// Single entry in a Git tree
#[derive(Debug, Clone, Deserialize)]
struct GitTreeEntry {
    path: String,
    #[serde(rename = "type")]
    entry_type: String,
    #[allow(dead_code)]
    mode: String,
    #[allow(dead_code)]
    sha: String,
    #[allow(dead_code)]
    size: Option<u64>,
    #[allow(dead_code)]
    url: Option<String>,
}

/// Claude settings.json structure (partial)
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct ClaudeSettings {
    hooks: Option<ClaudeHooks>,
    #[serde(rename = "subagentTypes")]
    subagent_types: Option<HashMap<String, serde_json::Value>>,
}

/// Claude hooks configuration
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct ClaudeHooks {
    #[serde(rename = "preToolCall")]
    pre_tool_call: Option<Vec<serde_json::Value>>,
    #[serde(rename = "postToolCall")]
    post_tool_call: Option<Vec<serde_json::Value>>,
    #[serde(rename = "onError")]
    on_error: Option<Vec<serde_json::Value>>,
}

/// GitHub scanner client
pub struct GitHubScanner {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl GitHubScanner {
    /// Create a new scanner with optional authentication token
    pub fn new(token: Option<String>) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("allbeads/1.0"),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            header::HeaderValue::from_static("2022-11-28"),
        );

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .default_headers(headers)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: "https://api.github.com".to_string(),
            token,
        }
    }

    /// Create a scanner for GitHub Enterprise
    pub fn with_base_url(token: Option<String>, base_url: String) -> Self {
        let mut scanner = Self::new(token);
        scanner.base_url = base_url.trim_end_matches('/').to_string();
        scanner
    }

    /// Scan a user's repositories
    pub async fn scan_user(&self, username: &str, filter: &ScanFilter) -> Result<ScanResult> {
        self.scan_user_with_options(username, filter, &ScanOptions::default())
            .await
    }

    /// Scan a user's repositories with options
    pub async fn scan_user_with_options(
        &self,
        username: &str,
        filter: &ScanFilter,
        options: &ScanOptions,
    ) -> Result<ScanResult> {
        if options.show_progress {
            eprint!("Fetching repository list for {}...", username);
            io::stderr().flush().ok();
        }
        let repos = self.list_user_repos(username).await?;
        if options.show_progress {
            eprintln!(" found {} repos", repos.len());
        }
        self.process_repos_with_options(
            repos,
            ScanSource::User(username.to_string()),
            filter,
            options,
        )
        .await
    }

    /// Scan an organization's repositories
    pub async fn scan_org(&self, org: &str, filter: &ScanFilter) -> Result<ScanResult> {
        self.scan_org_with_options(org, filter, &ScanOptions::default())
            .await
    }

    /// Scan an organization's repositories with options
    pub async fn scan_org_with_options(
        &self,
        org: &str,
        filter: &ScanFilter,
        options: &ScanOptions,
    ) -> Result<ScanResult> {
        if options.show_progress {
            eprint!("Fetching repository list for {}...", org);
            io::stderr().flush().ok();
        }
        let repos = self.list_org_repos(org).await?;
        if options.show_progress {
            eprintln!(" found {} repos", repos.len());
        }
        self.process_repos_with_options(
            repos,
            ScanSource::Organization(org.to_string()),
            filter,
            options,
        )
        .await
    }

    /// Scan a single repository by owner and name
    pub async fn scan_single_repo(&self, owner: &str, repo: &str) -> Result<ScanResult> {
        self.scan_single_repo_with_options(owner, repo, &ScanOptions::default())
            .await
    }

    /// Scan a single repository with options
    pub async fn scan_single_repo_with_options(
        &self,
        owner: &str,
        repo: &str,
        options: &ScanOptions,
    ) -> Result<ScanResult> {
        if options.show_progress {
            eprintln!("Fetching repository info for {}/{}...", owner, repo);
        }

        let url = format!("{}/repos/{}/{}", self.base_url, owner, repo);

        let mut request = self.client.get(&url);
        if let Some(ref token) = self.token {
            request = request.bearer_auth(token);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(crate::AllBeadsError::Network(format!(
                "GitHub API error ({}): {}",
                status, body
            )));
        }

        let github_repo: GitHubRepo = response.json().await?;

        // For single repo, directly check for agent files instead of search API
        if options.show_progress {
            eprintln!("Checking for agent configurations...");
        }
        let detected_agents = self
            .detect_agents_in_repo(owner, repo, options.show_progress)
            .await;

        // Check if managed
        let config_path = crate::config::AllBeadsConfig::default_path();
        let config = crate::config::AllBeadsConfig::load(&config_path).unwrap_or_default();
        let managed_repos: std::collections::HashSet<String> = config
            .contexts
            .iter()
            .filter_map(|c| {
                c.url
                    .split('/')
                    .next_back()
                    .map(|s| s.trim_end_matches(".git").to_lowercase())
            })
            .collect();
        let managed = managed_repos.contains(&repo.to_lowercase());

        // Parse timestamps
        let last_push = github_repo.pushed_at.as_ref().and_then(|pushed| {
            chrono::DateTime::parse_from_rfc3339(pushed)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });
        let created_at = chrono::DateTime::parse_from_rfc3339(&github_repo.created_at)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        let days_since_push = last_push.map(|dt| (chrono::Utc::now() - dt).num_days());

        let onboarding_priority = if managed || github_repo.archived || github_repo.disabled {
            OnboardingPriority::Skip
        } else if !detected_agents.is_empty() && days_since_push.map(|d| d <= 90).unwrap_or(false) {
            OnboardingPriority::High
        } else if days_since_push.map(|d| d <= 90).unwrap_or(false) {
            OnboardingPriority::Medium
        } else {
            OnboardingPriority::Low
        };

        // Fetch detailed info if any detailed fields are requested
        let detailed = if options.fields.requires_detailed() {
            if options.show_progress {
                eprintln!("Fetching detailed info via Git Trees API...");
            }
            self.fetch_detailed_info(&github_repo.full_name, &github_repo.default_branch)
                .await
                .ok()
        } else {
            None
        };

        let scanned_repo = ScannedRepo {
            name: github_repo.name.clone(),
            full_name: github_repo.full_name.clone(),
            url: github_repo.html_url.clone(),
            clone_url: github_repo.clone_url.clone(),
            description: github_repo.description.clone(),
            language: github_repo.language.clone(),
            stars: github_repo.stargazers_count,
            forks: github_repo.forks_count,
            is_fork: github_repo.fork,
            is_archived: github_repo.archived,
            is_private: github_repo.private,
            created_at,
            last_push,
            default_branch: github_repo.default_branch.clone(),
            topics: github_repo.topics.clone(),
            managed,
            detected_agents,
            onboarding_priority,
            days_since_push,
            detailed,
        };

        if options.show_progress {
            eprintln!("Scan complete!");
        }

        Ok(ScanResult {
            timestamp: chrono::Utc::now(),
            source: ScanSource::Repository(format!("{}/{}", owner, repo)),
            repositories: vec![scanned_repo],
            summary: ScanSummary {
                total_repos: 1,
                managed_repos: if managed { 1 } else { 0 },
                unmanaged_repos: if managed { 0 } else { 1 },
                ..Default::default()
            },
        })
    }

    /// Detect agents by directly checking repo contents (for single repo scans)
    async fn detect_agents_in_repo(
        &self,
        owner: &str,
        repo: &str,
        show_progress: bool,
    ) -> Vec<AgentType> {
        let mut agents = Vec::new();

        // Check each agent file/directory
        let checks = vec![
            ("CLAUDE.md", AgentType::Claude),
            (".cursorrules", AgentType::Cursor),
            (".github/copilot-instructions.md", AgentType::Copilot),
            (".aider.conf.yml", AgentType::Aider),
            (".kiro", AgentType::Kiro),
            (".codex", AgentType::Codex),
            (".gemini", AgentType::Gemini),
            ("AGENTS.md", AgentType::GenericAgent),
        ];

        for (path, agent_type) in checks {
            let url = format!(
                "{}/repos/{}/{}/contents/{}",
                self.base_url, owner, repo, path
            );

            let mut request = self.client.get(&url);
            if let Some(ref token) = self.token {
                request = request.bearer_auth(token);
            }

            if let Ok(response) = request.send().await {
                if response.status().is_success() {
                    agents.push(agent_type);
                    if show_progress {
                        eprintln!("  Found: {}", agent_type.name());
                    }
                }
            }
        }

        if agents.is_empty() && show_progress {
            eprintln!("  No agent configurations found");
        }

        agents
    }

    /// List all repos for a user
    async fn list_user_repos(&self, username: &str) -> Result<Vec<GitHubRepo>> {
        let mut all_repos = Vec::new();
        let mut page = 1;
        let per_page = 100;

        loop {
            let url = format!(
                "{}/users/{}/repos?per_page={}&page={}&sort=pushed&direction=desc",
                self.base_url, username, per_page, page
            );

            let mut request = self.client.get(&url);
            if let Some(ref token) = self.token {
                request = request.bearer_auth(token);
            }

            let response = request.send().await?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(crate::AllBeadsError::Network(format!(
                    "GitHub API error ({}): {}",
                    status, body
                )));
            }

            let repos: Vec<GitHubRepo> = response.json().await?;

            if repos.is_empty() {
                break;
            }

            all_repos.extend(repos);

            if all_repos.len() < per_page * page {
                break; // Last page
            }

            page += 1;
        }

        Ok(all_repos)
    }

    /// List all repos for an organization
    async fn list_org_repos(&self, org: &str) -> Result<Vec<GitHubRepo>> {
        let mut all_repos = Vec::new();
        let mut page = 1;
        let per_page = 100;

        loop {
            let url = format!(
                "{}/orgs/{}/repos?per_page={}&page={}&sort=pushed&direction=desc",
                self.base_url, org, per_page, page
            );

            let mut request = self.client.get(&url);
            if let Some(ref token) = self.token {
                request = request.bearer_auth(token);
            }

            let response = request.send().await?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(crate::AllBeadsError::Network(format!(
                    "GitHub API error ({}): {}",
                    status, body
                )));
            }

            let repos: Vec<GitHubRepo> = response.json().await?;

            if repos.is_empty() {
                break;
            }

            all_repos.extend(repos);

            if all_repos.len() < per_page * page {
                break; // Last page
            }

            page += 1;
        }

        Ok(all_repos)
    }

    /// Process raw repos into scan results (default options)
    #[allow(dead_code)]
    async fn process_repos(
        &self,
        repos: Vec<GitHubRepo>,
        source: ScanSource,
        filter: &ScanFilter,
    ) -> Result<ScanResult> {
        self.process_repos_with_options(repos, source, filter, &ScanOptions::default())
            .await
    }

    /// Process raw repos into scan results with options
    async fn process_repos_with_options(
        &self,
        repos: Vec<GitHubRepo>,
        source: ScanSource,
        filter: &ScanFilter,
        options: &ScanOptions,
    ) -> Result<ScanResult> {
        let now = Utc::now();
        let managed_repos = self.get_managed_repos();

        // First pass: filter repos and collect metadata
        if options.show_progress {
            eprintln!("Filtering repositories...");
        }

        let mut filtered_repos: Vec<RepoMetadata> = Vec::new();

        for repo in repos {
            let last_push = repo
                .pushed_at
                .as_ref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            let created_at = DateTime::parse_from_rfc3339(&repo.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or(now);

            let days_since_push = last_push.map(|lp| (now - lp).num_days());

            // Apply filters
            if filter.exclude_forks && repo.fork {
                continue;
            }
            if filter.exclude_archived && repo.archived {
                continue;
            }
            if filter.exclude_private && repo.private {
                continue;
            }
            if let Some(min) = filter.min_stars {
                if repo.stargazers_count < min {
                    continue;
                }
            }
            if let Some(ref lang) = filter.language {
                if repo.language.as_ref() != Some(lang) {
                    continue;
                }
            }
            if let Some(days) = filter.activity_days {
                if let Some(d) = days_since_push {
                    if d > days as i64 {
                        continue;
                    }
                }
            }
            if !filter.topics.is_empty() {
                let has_topic = filter.topics.iter().any(|t| repo.topics.contains(t));
                if !has_topic {
                    continue;
                }
            }

            let managed = managed_repos.contains(&repo.full_name.to_lowercase())
                || managed_repos.contains(&repo.name.to_lowercase());

            filtered_repos.push((repo, last_push, created_at, days_since_push, managed));
        }

        if options.show_progress {
            eprintln!("  {} repos after filtering", filtered_repos.len());
        }

        // Second pass: detect agents
        // Use GitHub Search API if enabled (much faster for many repos)
        let agent_map: HashMap<String, Vec<AgentType>> =
            if options.use_search_api && self.token.is_some() {
                self.detect_agents_via_search(&source, options).await?
            } else {
                // Fallback: parallel per-repo checks
                self.detect_agents_parallel(&filtered_repos, options)
                    .await?
            };

        // Third pass: fetch detailed info if any detailed fields are requested (in parallel)
        let detailed_map: HashMap<String, DetailedInfo> = if options.fields.requires_detailed() {
            if options.show_progress {
                eprintln!(
                    "Fetching detailed info for {} repos via Git Trees API...",
                    filtered_repos.len()
                );
            }
            self.fetch_detailed_info_parallel(&filtered_repos, options)
                .await?
        } else {
            HashMap::new()
        };

        // Build final results
        let mut scanned_repos = Vec::new();

        for (repo, last_push, created_at, days_since_push, managed) in filtered_repos {
            let detected_agents = agent_map
                .get(&repo.full_name.to_lowercase())
                .cloned()
                .unwrap_or_default();

            let onboarding_priority =
                self.calculate_priority(&repo, days_since_push, &detected_agents, managed);

            let detailed = detailed_map.get(&repo.full_name.to_lowercase()).cloned();

            scanned_repos.push(ScannedRepo {
                name: repo.name,
                full_name: repo.full_name,
                url: repo.html_url,
                clone_url: repo.clone_url,
                description: repo.description,
                language: repo.language,
                stars: repo.stargazers_count,
                forks: repo.forks_count,
                is_fork: repo.fork,
                is_archived: repo.archived,
                is_private: repo.private,
                last_push,
                created_at,
                default_branch: repo.default_branch,
                topics: repo.topics,
                managed,
                detected_agents,
                onboarding_priority,
                days_since_push,
                detailed,
            });
        }

        // Sort by priority then by stars
        scanned_repos.sort_by(|a, b| {
            let priority_order = |p: &OnboardingPriority| -> u8 {
                match p {
                    OnboardingPriority::High => 0,
                    OnboardingPriority::Medium => 1,
                    OnboardingPriority::Low => 2,
                    OnboardingPriority::Skip => 3,
                }
            };
            let pa = priority_order(&a.onboarding_priority);
            let pb = priority_order(&b.onboarding_priority);
            if pa != pb {
                return pa.cmp(&pb);
            }
            b.stars.cmp(&a.stars)
        });

        let summary = self.calculate_summary(&scanned_repos);

        if options.show_progress {
            eprintln!("Scan complete!");
        }

        Ok(ScanResult {
            timestamp: now,
            source,
            repositories: scanned_repos,
            summary,
        })
    }

    /// Detect agents using GitHub Search API (one search per agent type)
    /// This is MUCH faster than per-repo checks for large accounts
    async fn detect_agents_via_search(
        &self,
        source: &ScanSource,
        options: &ScanOptions,
    ) -> Result<HashMap<String, Vec<AgentType>>> {
        let mut agent_map: HashMap<String, Vec<AgentType>> = HashMap::new();

        // Agent file patterns to search
        let searches = [
            ("filename:CLAUDE.md", AgentType::Claude),
            ("filename:.cursorrules", AgentType::Cursor),
            (
                "path:.github filename:copilot-instructions.md",
                AgentType::Copilot,
            ),
            ("filename:.aider.conf.yml", AgentType::Aider),
            ("path:.kiro", AgentType::Kiro),
            ("path:.codex", AgentType::Codex),
            ("path:.gemini", AgentType::Gemini),
            ("path:.agent", AgentType::GenericAgent),
        ];

        let qualifier = match source {
            ScanSource::User(u) => format!("user:{}", u),
            ScanSource::Organization(o) => format!("org:{}", o),
            ScanSource::Repository(r) => format!("repo:{}", r),
        };

        if options.show_progress {
            eprintln!(
                "Detecting agents via GitHub Search API ({} file patterns)...",
                searches.len()
            );
        }

        for (i, (query, agent_type)) in searches.iter().enumerate() {
            if options.show_progress {
                eprint!(
                    "  [{}/{}] Searching for {}...",
                    i + 1,
                    searches.len(),
                    agent_type.name()
                );
                io::stderr().flush().ok();
            }

            let full_query = format!("{} {}", query, qualifier);
            let url = format!(
                "{}/search/code?q={}&per_page=100",
                self.base_url,
                urlencoding::encode(&full_query)
            );

            let mut request = self.client.get(&url);
            if let Some(ref token) = self.token {
                request = request.bearer_auth(token);
            }

            match request.send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        if let Ok(result) = response.json::<SearchCodeResult>().await {
                            // Deduplicate repos - GitHub returns one result per file
                            let mut unique_repos: std::collections::HashSet<String> =
                                std::collections::HashSet::new();
                            for item in &result.items {
                                unique_repos.insert(item.repository.full_name.to_lowercase());
                            }
                            let count = unique_repos.len();
                            for repo_name in unique_repos {
                                let agents = agent_map.entry(repo_name).or_default();
                                if !agents.contains(agent_type) {
                                    agents.push(*agent_type);
                                }
                            }
                            if options.show_progress {
                                eprintln!(" {} repos", count);
                            }
                        } else if options.show_progress {
                            eprintln!(" (parse error)");
                        }
                    } else if options.show_progress {
                        eprintln!(" (API error: {})", response.status());
                    }
                }
                Err(_) if options.show_progress => {
                    eprintln!(" (network error)");
                }
                Err(_) => {}
            }

            // Small delay to avoid rate limiting
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Ok(agent_map)
    }

    /// Detect agents in parallel batches (fallback when search API unavailable)
    async fn detect_agents_parallel(
        &self,
        repos: &[RepoMetadata],
        options: &ScanOptions,
    ) -> Result<HashMap<String, Vec<AgentType>>> {
        let total = repos.len();
        let agent_map: Arc<tokio::sync::Mutex<HashMap<String, Vec<AgentType>>>> =
            Arc::new(tokio::sync::Mutex::new(HashMap::new()));

        if options.show_progress {
            eprintln!(
                "Detecting agents via per-repo checks ({} repos, {} concurrent)...",
                total, options.concurrency
            );
        }

        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        // Process repos in parallel batches
        let _results: Vec<_> = stream::iter(repos.iter().enumerate())
            .map(|(_idx, (repo, _, _, _, _))| {
                let client = self.client.clone();
                let token = self.token.clone();
                let base_url = self.base_url.clone();
                let full_name = repo.full_name.clone();
                let default_branch = repo.default_branch.clone();
                let agent_map = agent_map.clone();
                let counter = counter.clone();
                let show_progress = options.show_progress;

                async move {
                    let agents = detect_agents_for_repo(
                        &client,
                        token.as_deref(),
                        &base_url,
                        &full_name,
                        &default_branch,
                    )
                    .await;

                    let count = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;

                    if show_progress {
                        let pct = (count as f64 / total as f64) * 100.0;
                        eprint!(
                            "\r  [{}/{}] {:.0}% - {}                    ",
                            count, total, pct, full_name
                        );
                        io::stderr().flush().ok();
                    }

                    let mut map = agent_map.lock().await;
                    if !agents.is_empty() {
                        map.insert(full_name.to_lowercase(), agents);
                    }
                }
            })
            .buffer_unordered(options.concurrency)
            .collect()
            .await;

        if options.show_progress {
            eprintln!();
        }

        let map = agent_map.lock().await;
        Ok(map.clone())
    }

    /// Get list of managed repos from AllBeads config
    fn get_managed_repos(&self) -> HashSet<String> {
        let mut managed = HashSet::new();

        // Try to load AllBeads config
        let config_path = AllBeadsConfig::default_path();
        if let Ok(config) = AllBeadsConfig::load(&config_path) {
            for context in &config.contexts {
                if let Some(ref local_path) = context.path {
                    // Extract repo name from path
                    if let Some(name) = local_path.file_name() {
                        managed.insert(name.to_string_lossy().to_lowercase());
                    }
                }
                // Also add the context name as it might match repo name
                managed.insert(context.name.to_lowercase());
            }
        }

        managed
    }

    /// Detect agents by checking for config files via GitHub Contents API
    #[allow(dead_code)]
    async fn detect_agents_via_api(
        &self,
        full_name: &str,
        default_branch: &str,
    ) -> Result<Vec<AgentType>> {
        let mut agents = Vec::new();

        // Agent config files to check
        let checks = vec![
            ("CLAUDE.md", AgentType::Claude),
            (".claude", AgentType::Claude),
            (".github/copilot-instructions.md", AgentType::Copilot),
            (".cursorrules", AgentType::Cursor),
            (".aider.conf.yml", AgentType::Aider),
            (".cody", AgentType::Cody),
            (".continue", AgentType::Continue),
            (".windsurf", AgentType::Windsurf),
            (".amazonq", AgentType::AmazonQ),
            (".kiro", AgentType::Kiro),
            (".opencode", AgentType::OpenCode),
            (".factory", AgentType::Droid),
            (".codex", AgentType::Codex),
            (".gemini", AgentType::Gemini),
            (".agent", AgentType::GenericAgent),
        ];

        for (path, agent_type) in checks {
            let url = format!(
                "{}/repos/{}/contents/{}?ref={}",
                self.base_url, full_name, path, default_branch
            );

            let mut request = self.client.head(&url);
            if let Some(ref token) = self.token {
                request = request.bearer_auth(token);
            }

            if let Ok(response) = request.send().await {
                if response.status().is_success() && !agents.contains(&agent_type) {
                    agents.push(agent_type);
                }
            }
        }

        Ok(agents)
    }

    /// Fetch detailed info for a single repo using Git Trees API
    /// This is the most efficient way to check file existence in bulk
    async fn fetch_detailed_info(
        &self,
        full_name: &str,
        default_branch: &str,
    ) -> Result<DetailedInfo> {
        // Fetch the tree recursively in a single API call
        let url = format!(
            "{}/repos/{}/git/trees/{}?recursive=1",
            self.base_url, full_name, default_branch
        );

        let mut request = self.client.get(&url);
        if let Some(ref token) = self.token {
            request = request.bearer_auth(token);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(crate::AllBeadsError::Network(format!(
                "Failed to fetch tree for {}: {}",
                full_name,
                response.status()
            )));
        }

        let tree: GitTreeResponse = response.json().await?;
        self.parse_detailed_info_from_tree(&tree, full_name).await
    }

    /// Fetch detailed info for multiple repos in parallel
    async fn fetch_detailed_info_parallel(
        &self,
        repos: &[RepoMetadata],
        options: &ScanOptions,
    ) -> Result<HashMap<String, DetailedInfo>> {
        let total = repos.len();
        let detailed_map: Arc<tokio::sync::Mutex<HashMap<String, DetailedInfo>>> =
            Arc::new(tokio::sync::Mutex::new(HashMap::new()));

        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let _results: Vec<_> = stream::iter(repos.iter())
            .map(|(repo, _, _, _, _)| {
                let client = self.client.clone();
                let token = self.token.clone();
                let base_url = self.base_url.clone();
                let full_name = repo.full_name.clone();
                let default_branch = repo.default_branch.clone();
                let detailed_map = detailed_map.clone();
                let counter = counter.clone();
                let show_progress = options.show_progress;

                async move {
                    let url = format!(
                        "{}/repos/{}/git/trees/{}?recursive=1",
                        base_url, full_name, default_branch
                    );

                    let mut request = client.get(&url);
                    if let Some(ref t) = token {
                        request = request.bearer_auth(t);
                    }

                    let count = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;

                    if show_progress {
                        let pct = (count as f64 / total as f64) * 100.0;
                        eprint!(
                            "\r  [{}/{}] {:.0}% - {}                    ",
                            count, total, pct, full_name
                        );
                        io::stderr().flush().ok();
                    }

                    if let Ok(response) = request.send().await {
                        if response.status().is_success() {
                            if let Ok(tree) = response.json::<GitTreeResponse>().await {
                                let info = parse_tree_entries(&tree.tree);
                                let mut map = detailed_map.lock().await;
                                map.insert(full_name.to_lowercase(), info);
                            }
                        }
                    }
                }
            })
            .buffer_unordered(options.concurrency)
            .collect()
            .await;

        if options.show_progress {
            eprintln!();
        }

        let map = detailed_map.lock().await;
        Ok(map.clone())
    }

    /// Parse detailed info from a Git tree response
    async fn parse_detailed_info_from_tree(
        &self,
        tree: &GitTreeResponse,
        full_name: &str,
    ) -> Result<DetailedInfo> {
        let mut info = parse_tree_entries(&tree.tree);

        // If settings.json exists, fetch and parse it for more details
        if info.has_settings {
            if let Ok(settings) = self.fetch_settings_json(full_name).await {
                // Count hooks
                if let Some(ref hooks) = settings.hooks {
                    let mut hook_count = 0;
                    if let Some(ref h) = hooks.pre_tool_call {
                        hook_count += h.len();
                    }
                    if let Some(ref h) = hooks.post_tool_call {
                        hook_count += h.len();
                    }
                    if let Some(ref h) = hooks.on_error {
                        hook_count += h.len();
                    }
                    info.hooks_count = hook_count;
                }

                // Extract subagent types
                if let Some(ref types) = settings.subagent_types {
                    info.subagent_types = types.keys().cloned().collect();
                }
            }
        }

        Ok(info)
    }

    /// Fetch and parse .claude/settings.json
    async fn fetch_settings_json(&self, full_name: &str) -> Result<ClaudeSettings> {
        let url = format!(
            "{}/repos/{}/contents/.claude/settings.json",
            self.base_url, full_name
        );

        let mut request = self.client.get(&url);
        if let Some(ref token) = self.token {
            request = request.bearer_auth(token);
        }
        request = request.header("Accept", "application/vnd.github.raw");

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(crate::AllBeadsError::Network(format!(
                "Failed to fetch settings.json for {}",
                full_name
            )));
        }

        let settings: ClaudeSettings = response.json().await?;
        Ok(settings)
    }

    /// Calculate onboarding priority for a repository
    fn calculate_priority(
        &self,
        repo: &GitHubRepo,
        days_since_push: Option<i64>,
        detected_agents: &[AgentType],
        managed: bool,
    ) -> OnboardingPriority {
        // Already managed - skip
        if managed {
            return OnboardingPriority::Skip;
        }

        // Archived or disabled - skip
        if repo.archived || repo.disabled {
            return OnboardingPriority::Skip;
        }

        // Fork - skip (usually)
        if repo.fork {
            return OnboardingPriority::Skip;
        }

        // Has agent config and is active - high priority
        let is_active = days_since_push.map(|d| d <= 90).unwrap_or(false);
        let has_agents = !detected_agents.is_empty();

        if has_agents && is_active {
            return OnboardingPriority::High;
        }

        // Active but no agents - medium priority
        if is_active {
            return OnboardingPriority::Medium;
        }

        // Inactive - low priority
        OnboardingPriority::Low
    }

    /// Calculate summary statistics
    fn calculate_summary(&self, repos: &[ScannedRepo]) -> ScanSummary {
        let mut summary = ScanSummary::default();
        let mut agent_map: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for repo in repos {
            summary.total_repos += 1;

            if repo.managed {
                summary.managed_repos += 1;
            } else {
                summary.unmanaged_repos += 1;
            }

            match repo.onboarding_priority {
                OnboardingPriority::High => summary.high_priority += 1,
                OnboardingPriority::Medium => summary.medium_priority += 1,
                OnboardingPriority::Low => summary.low_priority += 1,
                OnboardingPriority::Skip => summary.skip += 1,
            }

            if !repo.detected_agents.is_empty() {
                summary.with_agents += 1;
                for agent in &repo.detected_agents {
                    *agent_map.entry(agent.name().to_string()).or_insert(0) += 1;
                }
            }
        }

        // Sort agent counts
        let mut agent_counts: Vec<_> = agent_map.into_iter().collect();
        agent_counts.sort_by(|a, b| b.1.cmp(&a.1));
        summary.agent_counts = agent_counts;

        summary
    }
}

/// Standalone function to detect agents for a single repo (used in parallel processing)
async fn detect_agents_for_repo(
    client: &Client,
    token: Option<&str>,
    base_url: &str,
    full_name: &str,
    default_branch: &str,
) -> Vec<AgentType> {
    let mut agents = Vec::new();

    let checks = vec![
        ("CLAUDE.md", AgentType::Claude),
        (".claude", AgentType::Claude),
        (".github/copilot-instructions.md", AgentType::Copilot),
        (".cursorrules", AgentType::Cursor),
        (".aider.conf.yml", AgentType::Aider),
        (".cody", AgentType::Cody),
        (".continue", AgentType::Continue),
        (".windsurf", AgentType::Windsurf),
        (".amazonq", AgentType::AmazonQ),
        (".kiro", AgentType::Kiro),
        (".opencode", AgentType::OpenCode),
        (".factory", AgentType::Droid),
        (".codex", AgentType::Codex),
        (".gemini", AgentType::Gemini),
        (".agent", AgentType::GenericAgent),
    ];

    for (path, agent_type) in checks {
        let url = format!(
            "{}/repos/{}/contents/{}?ref={}",
            base_url, full_name, path, default_branch
        );

        let mut request = client.head(&url);
        if let Some(t) = token {
            request = request.bearer_auth(t);
        }

        if let Ok(response) = request.send().await {
            if response.status().is_success() && !agents.contains(&agent_type) {
                agents.push(agent_type);
            }
        }
    }

    agents
}

/// Parse tree entries to extract detailed info (BSICH+ features)
fn parse_tree_entries(entries: &[GitTreeEntry]) -> DetailedInfo {
    let mut info = DetailedInfo::default();

    let mut workflow_files = 0;
    let mut command_files = 0;

    for entry in entries {
        let path = &entry.path;

        // Check for .claude/settings.json (or settings.local.json)
        if path == ".claude/settings.json" || path == ".claude/settings.local.json" {
            info.has_settings = true;
        }

        // Check for .github/workflows directory
        if path.starts_with(".github/workflows/") && entry.entry_type == "blob" {
            if path.ends_with(".yml") || path.ends_with(".yaml") {
                workflow_files += 1;
            }
        }
        if path == ".github/workflows" && entry.entry_type == "tree" {
            info.has_workflows = true;
        }

        // Check for .claude/commands directory
        if path.starts_with(".claude/commands/") && entry.entry_type == "blob" {
            if path.ends_with(".md") {
                command_files += 1;
            }
        }
        if path == ".claude/commands" && entry.entry_type == "tree" {
            info.has_commands = true;
        }

        // Check for .beads directory
        if path == ".beads" && entry.entry_type == "tree" {
            info.has_beads = true;
        }
    }

    // Set counts if directories exist
    if workflow_files > 0 {
        info.has_workflows = true;
    }
    info.workflow_count = workflow_files;

    if command_files > 0 {
        info.has_commands = true;
    }
    info.command_count = command_files;

    info
}

/// Print scan results in a formatted way
pub fn print_scan_result(result: &ScanResult, show_all: bool) {
    println!(
        "GitHub {} Scan: {}",
        match &result.source {
            ScanSource::User(_) => "User",
            ScanSource::Organization(_) => "Organization",
            ScanSource::Repository(_) => "Repository",
        },
        match &result.source {
            ScanSource::User(u) => u,
            ScanSource::Organization(o) => o,
            ScanSource::Repository(r) => r,
        }
    );
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!(
        "Found {} repositories, {} already managed by AllBeads",
        result.summary.total_repos, result.summary.managed_repos
    );
    println!();

    // Group by priority
    let high: Vec<_> = result
        .repositories
        .iter()
        .filter(|r| r.onboarding_priority == OnboardingPriority::High && !r.managed)
        .collect();
    let medium: Vec<_> = result
        .repositories
        .iter()
        .filter(|r| r.onboarding_priority == OnboardingPriority::Medium && !r.managed)
        .collect();
    let low: Vec<_> = result
        .repositories
        .iter()
        .filter(|r| r.onboarding_priority == OnboardingPriority::Low && !r.managed)
        .collect();

    if result.summary.unmanaged_repos > 0 {
        println!(
            "Unmanaged Repositories ({}):",
            result.summary.unmanaged_repos
        );
        println!();
    }

    // Print high priority
    if !high.is_empty() {
        println!("  Priority: High (has agent config, active)");
        for (i, repo) in high.iter().enumerate() {
            let limit = if show_all { usize::MAX } else { 5 };
            if i >= limit {
                println!("  └── ... ({} more)", high.len() - limit);
                break;
            }
            let agents: Vec<_> = repo.detected_agents.iter().map(|a| a.name()).collect();
            let agent_str = if agents.is_empty() {
                "".to_string()
            } else {
                format!("  [{}]", agents.join(", "))
            };
            let lang = repo.language.as_deref().unwrap_or("-");
            let activity = repo
                .days_since_push
                .map(|d| format!("{}d ago", d))
                .unwrap_or_else(|| "unknown".to_string());
            println!(
                "  ├── {:<24} ★{:<4} {:<8} {:>10}{}",
                repo.name, repo.stars, lang, activity, agent_str
            );
        }
        println!();
    }

    // Print medium priority
    if !medium.is_empty() {
        println!("  Priority: Medium (active, no agent config)");
        let limit = if show_all { usize::MAX } else { 5 };
        for (i, repo) in medium.iter().enumerate() {
            if i >= limit {
                println!("  └── ... ({} more)", medium.len() - limit);
                break;
            }
            let lang = repo.language.as_deref().unwrap_or("-");
            let activity = repo
                .days_since_push
                .map(|d| format!("{}d ago", d))
                .unwrap_or_else(|| "unknown".to_string());
            println!(
                "  ├── {:<24} ★{:<4} {:<8} {:>10}",
                repo.name, repo.stars, lang, activity
            );
        }
        println!();
    }

    // Print low priority
    if !low.is_empty() && show_all {
        println!("  Priority: Low (inactive or small)");
        for repo in low.iter() {
            let lang = repo.language.as_deref().unwrap_or("-");
            let activity = repo
                .days_since_push
                .map(|d| format!("{}d ago", d))
                .unwrap_or_else(|| "unknown".to_string());
            println!(
                "  ├── {:<24} ★{:<4} {:<8} {:>10}",
                repo.name, repo.stars, lang, activity
            );
        }
        println!();
    } else if !low.is_empty() {
        println!("  Priority: Low ({} repos, use --all to show)", low.len());
        println!();
    }

    // Recommendations
    if result.summary.with_agents > 0 || result.summary.medium_priority > 0 {
        println!("Recommendations:");
        if result.summary.with_agents > 0 {
            println!(
                "  • {} repos have agent configs - consider onboarding first",
                result.summary.with_agents
            );
        }
        if result.summary.medium_priority > 0 {
            println!(
                "  • {} active repos have no agent config - opportunity for adoption",
                result.summary.medium_priority
            );
        }
        if result.summary.low_priority > 0 {
            println!(
                "  • {} repos inactive >90 days - consider archiving",
                result.summary.low_priority
            );
        }
        println!();
    }

    // Agent adoption summary
    if !result.summary.agent_counts.is_empty() {
        println!("Agent Adoption:");
        for (agent, count) in &result.summary.agent_counts {
            println!("  {}: {} repos", agent, count);
        }
        println!();
    }

    // Detailed info summary (BSICH+ features)
    let detailed_repos: Vec<_> = result
        .repositories
        .iter()
        .filter_map(|r| r.detailed.as_ref().map(|d| (r, d)))
        .collect();

    if !detailed_repos.is_empty() {
        println!("Detailed Analysis (BSICH+):");
        let with_settings = detailed_repos
            .iter()
            .filter(|(_, d)| d.has_settings)
            .count();
        let with_workflows = detailed_repos
            .iter()
            .filter(|(_, d)| d.has_workflows)
            .count();
        let with_commands = detailed_repos
            .iter()
            .filter(|(_, d)| d.has_commands)
            .count();
        let with_beads = detailed_repos.iter().filter(|(_, d)| d.has_beads).count();
        let total_workflows: usize = detailed_repos.iter().map(|(_, d)| d.workflow_count).sum();
        let total_commands: usize = detailed_repos.iter().map(|(_, d)| d.command_count).sum();

        println!(
            "  Settings (.claude/settings.json): {} repos",
            with_settings
        );
        println!(
            "  CI/CD (.github/workflows):        {} repos ({} workflows)",
            with_workflows, total_workflows
        );
        println!(
            "  Commands (.claude/commands):      {} repos ({} commands)",
            with_commands, total_commands
        );
        println!("  Beads (.beads):                   {} repos", with_beads);

        // Show subagent breakdown if any
        let mut subagent_counts: HashMap<String, usize> = HashMap::new();
        for (_, d) in &detailed_repos {
            for agent_type in &d.subagent_types {
                *subagent_counts.entry(agent_type.clone()).or_insert(0) += 1;
            }
        }
        if !subagent_counts.is_empty() {
            println!("\n  Subagent Types:");
            let mut counts: Vec<_> = subagent_counts.into_iter().collect();
            counts.sort_by(|a, b| b.1.cmp(&a.1));
            for (agent_type, count) in counts.iter().take(5) {
                println!("    {}: {} repos", agent_type, count);
            }
            if counts.len() > 5 {
                println!("    ... and {} more", counts.len() - 5);
            }
        }
        println!();
    }

    println!("Run: ab onboard <repo> to start onboarding");
}

/// Format scan results as CSV (uses default fields)
pub fn format_scan_result_csv(result: &ScanResult) -> String {
    // Use basic fields or all fields based on whether detailed info is present
    let fields = if result.repositories.iter().any(|r| r.detailed.is_some()) {
        FieldSet::all()
    } else {
        FieldSet::basic()
    };
    format_scan_result_csv_with_fields(result, &fields)
}

/// Format scan results as CSV with specific fields
pub fn format_scan_result_csv_with_fields(result: &ScanResult, fields: &FieldSet) -> String {
    let mut csv = String::new();

    // Header from field set
    csv.push_str(&fields.csv_header());
    csv.push('\n');

    // Data rows
    for repo in &result.repositories {
        let mut values: Vec<String> = Vec::new();

        for field in fields.ordered() {
            let value = match field {
                ScanField::Name => escape_csv(&repo.name),
                ScanField::FullName => escape_csv(&repo.full_name),
                ScanField::Url => escape_csv(&repo.url),
                ScanField::Language => escape_csv(repo.language.as_deref().unwrap_or("")),
                ScanField::Stars => repo.stars.to_string(),
                ScanField::Forks => repo.forks.to_string(),
                ScanField::IsFork => repo.is_fork.to_string(),
                ScanField::IsArchived => repo.is_archived.to_string(),
                ScanField::IsPrivate => repo.is_private.to_string(),
                ScanField::DaysSincePush => repo
                    .days_since_push
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                ScanField::Managed => repo.managed.to_string(),
                ScanField::Priority => format!("{:?}", repo.onboarding_priority),
                ScanField::Agents => {
                    let agents: Vec<_> = repo.detected_agents.iter().map(|a| a.name()).collect();
                    escape_csv(&agents.join(";"))
                }
                ScanField::Settings => {
                    // Format: "yes (3 hooks, 2 subagents)" or "no"
                    repo.detailed
                        .as_ref()
                        .map(|d| {
                            if d.has_settings {
                                let parts: Vec<String> = [
                                    if d.hooks_count > 0 {
                                        Some(format!("{} hooks", d.hooks_count))
                                    } else {
                                        None
                                    },
                                    if !d.subagent_types.is_empty() {
                                        Some(format!("{} subagents", d.subagent_types.len()))
                                    } else {
                                        None
                                    },
                                ]
                                .into_iter()
                                .flatten()
                                .collect();
                                if parts.is_empty() {
                                    "yes".to_string()
                                } else {
                                    format!("yes ({})", parts.join(", "))
                                }
                            } else {
                                "no".to_string()
                            }
                        })
                        .unwrap_or_default()
                }
                ScanField::Workflows => {
                    // Format: "yes (5)" or "no"
                    repo.detailed
                        .as_ref()
                        .map(|d| {
                            if d.has_workflows {
                                format!("yes ({})", d.workflow_count)
                            } else {
                                "no".to_string()
                            }
                        })
                        .unwrap_or_default()
                }
                ScanField::Commands => {
                    // Format: "yes (3)" or "no"
                    repo.detailed
                        .as_ref()
                        .map(|d| {
                            if d.has_commands {
                                format!("yes ({})", d.command_count)
                            } else {
                                "no".to_string()
                            }
                        })
                        .unwrap_or_default()
                }
                ScanField::Beads => {
                    // Format: "yes" or "no"
                    repo.detailed
                        .as_ref()
                        .map(|d| if d.has_beads { "yes" } else { "no" }.to_string())
                        .unwrap_or_default()
                }
            };
            values.push(value);
        }

        csv.push_str(&values.join(","));
        csv.push('\n');
    }

    csv
}

/// Escape a field for CSV output
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Format scan results as JUnit XML (for CI integration)
///
/// Each repository is a test case:
/// - Managed repos are "passed" tests
/// - High priority unmanaged repos are "failures" (needs attention)
/// - Medium priority are "warnings" (skipped with message)
/// - Low priority are "passed" (info only)
pub fn format_scan_result_junit(result: &ScanResult) -> String {
    let mut xml = String::new();

    let source_name = match &result.source {
        ScanSource::User(u) => format!("user:{}", u),
        ScanSource::Organization(o) => format!("org:{}", o),
        ScanSource::Repository(r) => format!("repo:{}", r),
    };

    let total = result.repositories.len();
    let failures = result
        .repositories
        .iter()
        .filter(|r| !r.managed && r.onboarding_priority == OnboardingPriority::High)
        .count();
    let skipped = result
        .repositories
        .iter()
        .filter(|r| !r.managed && r.onboarding_priority == OnboardingPriority::Medium)
        .count();

    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(&format!(
        "<testsuites name=\"allbeads-scan\" tests=\"{}\" failures=\"{}\" skipped=\"{}\">\n",
        total, failures, skipped
    ));
    xml.push_str(&format!(
        "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" skipped=\"{}\" timestamp=\"{}\">\n",
        escape_xml(&source_name),
        total,
        failures,
        skipped,
        result.timestamp.to_rfc3339()
    ));

    for repo in &result.repositories {
        let agents: Vec<_> = repo.detected_agents.iter().map(|a| a.name()).collect();
        let agents_str = if agents.is_empty() {
            "none".to_string()
        } else {
            agents.join(", ")
        };

        xml.push_str(&format!(
            "    <testcase name=\"{}\" classname=\"{}\"",
            escape_xml(&repo.name),
            escape_xml(&repo.full_name)
        ));

        if repo.managed {
            // Managed repos pass
            xml.push_str(" />\n");
        } else {
            match repo.onboarding_priority {
                OnboardingPriority::High => {
                    // High priority unmanaged = failure
                    xml.push_str(">\n");
                    xml.push_str(&format!(
                        "      <failure message=\"Unmanaged repo with agent configs: {}\" type=\"high_priority\">\n",
                        agents_str
                    ));
                    xml.push_str(&format!(
                        "Repository {} has agent configuration but is not managed by AllBeads.\n",
                        repo.full_name
                    ));
                    xml.push_str(&format!(
                        "Stars: {}, Language: {}\n",
                        repo.stars,
                        repo.language.as_deref().unwrap_or("unknown")
                    ));
                    xml.push_str(&format!("URL: {}\n", repo.url));
                    xml.push_str("Run: ab onboard ");
                    xml.push_str(&repo.full_name);
                    xml.push_str("\n");
                    xml.push_str("      </failure>\n");
                    xml.push_str("    </testcase>\n");
                }
                OnboardingPriority::Medium => {
                    // Medium priority = skipped with message
                    xml.push_str(">\n");
                    xml.push_str(&format!(
                        "      <skipped message=\"Active repo without agent config\" />\n"
                    ));
                    xml.push_str("    </testcase>\n");
                }
                OnboardingPriority::Low | OnboardingPriority::Skip => {
                    // Low priority = pass (info only)
                    xml.push_str(" />\n");
                }
            }
        }
    }

    xml.push_str("  </testsuite>\n");
    xml.push_str("</testsuites>\n");

    xml
}

/// Escape a string for XML output
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_order() {
        assert!(OnboardingPriority::High < OnboardingPriority::Medium);
        assert!(OnboardingPriority::Medium < OnboardingPriority::Low);
        assert!(OnboardingPriority::Low < OnboardingPriority::Skip);
    }

    #[test]
    fn test_scan_filter_default() {
        let filter = ScanFilter::default();
        assert!(!filter.exclude_forks);
        assert!(!filter.exclude_archived);
        assert!(filter.min_stars.is_none());
    }

    #[test]
    fn test_scan_source_display() {
        let user = ScanSource::User("testuser".to_string());
        let org = ScanSource::Organization("testorg".to_string());
        let repo = ScanSource::Repository("user/repo".to_string());

        assert_eq!(format!("{}", user), "user:testuser");
        assert_eq!(format!("{}", org), "org:testorg");
        assert_eq!(format!("{}", repo), "repo:user/repo");
    }

    #[test]
    fn test_scan_summary_default() {
        let summary = ScanSummary::default();
        assert_eq!(summary.total_repos, 0);
        assert_eq!(summary.managed_repos, 0);
        assert_eq!(summary.unmanaged_repos, 0);
        assert_eq!(summary.with_agents, 0);
    }

    #[test]
    fn test_scanned_repo_priority_skip_for_managed() {
        // A managed repo should be Skip priority
        let repo = ScannedRepo {
            name: "test".to_string(),
            full_name: "user/test".to_string(),
            url: "https://github.com/user/test".to_string(),
            clone_url: "https://github.com/user/test.git".to_string(),
            description: None,
            language: None,
            stars: 0,
            forks: 0,
            is_fork: false,
            is_archived: false,
            is_private: false,
            created_at: chrono::Utc::now(),
            last_push: Some(chrono::Utc::now()),
            default_branch: "main".to_string(),
            topics: vec![],
            managed: true,
            detected_agents: vec![],
            onboarding_priority: OnboardingPriority::Skip,
            days_since_push: Some(0),
            detailed: None,
        };

        assert!(repo.managed);
        assert_eq!(repo.onboarding_priority, OnboardingPriority::Skip);
    }

    #[test]
    fn test_github_scanner_new() {
        let scanner = GitHubScanner::new(None);
        assert!(scanner.token.is_none());

        let scanner_with_token = GitHubScanner::new(Some("test_token".to_string()));
        assert!(scanner_with_token.token.is_some());
        assert_eq!(scanner_with_token.token.unwrap(), "test_token");
    }

    #[test]
    fn test_scan_options_default() {
        let options = ScanOptions::default();
        assert!(options.show_progress);
        assert!(options.use_search_api);
        assert_eq!(options.concurrency, 10);
    }

    #[test]
    fn test_scan_result_creation() {
        let result = ScanResult {
            timestamp: chrono::Utc::now(),
            source: ScanSource::Repository("user/repo".to_string()),
            repositories: vec![],
            summary: ScanSummary::default(),
        };

        assert!(matches!(result.source, ScanSource::Repository(_)));
        assert!(result.repositories.is_empty());
    }
}
