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
}

impl std::fmt::Display for ScanSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanSource::User(u) => write!(f, "user:{}", u),
            ScanSource::Organization(o) => write!(f, "org:{}", o),
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
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            concurrency: 10,
            use_search_api: true,
            show_progress: true,
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

        // Process just this one repo with default filter
        let filter = ScanFilter::default();
        let options = ScanOptions::default();
        self.process_repos_with_options(
            vec![github_repo],
            ScanSource::User(owner.to_string()),
            &filter,
            &options,
        )
        .await
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

        let mut filtered_repos: Vec<(GitHubRepo, Option<DateTime<Utc>>, DateTime<Utc>, Option<i64>, bool)> = Vec::new();

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
        let agent_map: HashMap<String, Vec<AgentType>> = if options.use_search_api && self.token.is_some() {
            self.detect_agents_via_search(&source, options).await?
        } else {
            // Fallback: parallel per-repo checks
            self.detect_agents_parallel(&filtered_repos, options).await?
        };

        // Build final results
        let mut scanned_repos = Vec::new();

        for (repo, last_push, created_at, days_since_push, managed) in filtered_repos {
            let detected_agents = agent_map
                .get(&repo.full_name.to_lowercase())
                .cloned()
                .unwrap_or_default();

            let onboarding_priority = self.calculate_priority(
                &repo,
                days_since_push,
                &detected_agents,
                managed,
            );

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
        let searches = vec![
            ("filename:CLAUDE.md", AgentType::Claude),
            ("filename:.cursorrules", AgentType::Cursor),
            ("path:.github filename:copilot-instructions.md", AgentType::Copilot),
            ("filename:.aider.conf.yml", AgentType::Aider),
            ("path:.kiro", AgentType::Kiro),
            ("path:.codex", AgentType::Codex),
            ("path:.gemini", AgentType::Gemini),
            ("path:.agent", AgentType::GenericAgent),
        ];

        let qualifier = match source {
            ScanSource::User(u) => format!("user:{}", u),
            ScanSource::Organization(o) => format!("org:{}", o),
        };

        if options.show_progress {
            eprintln!("Detecting agents via GitHub Search API ({} file patterns)...", searches.len());
        }

        for (i, (query, agent_type)) in searches.iter().enumerate() {
            if options.show_progress {
                eprint!("  [{}/{}] Searching for {}...", i + 1, searches.len(), agent_type.name());
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
                            let mut unique_repos: std::collections::HashSet<String> = std::collections::HashSet::new();
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
        repos: &[(GitHubRepo, Option<DateTime<Utc>>, DateTime<Utc>, Option<i64>, bool)],
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
                let total = total;

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
                        eprint!("\r  [{}/{}] {:.0}% - {}                    ", count, total, pct, full_name);
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
                if response.status().is_success() {
                    if !agents.contains(&agent_type) {
                        agents.push(agent_type);
                    }
                }
            }
        }

        Ok(agents)
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
        let mut agent_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

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

/// Print scan results in a formatted way
pub fn print_scan_result(result: &ScanResult, show_all: bool) {
    println!("GitHub {} Scan: {}",
        match &result.source {
            ScanSource::User(_) => "User",
            ScanSource::Organization(_) => "Organization",
        },
        match &result.source {
            ScanSource::User(u) => u,
            ScanSource::Organization(o) => o,
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
        println!("Unmanaged Repositories ({}):", result.summary.unmanaged_repos);
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

    println!("Run: ab onboard <repo> to start onboarding");
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
}
