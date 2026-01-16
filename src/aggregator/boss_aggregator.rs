//! Boss repository aggregator implementation

use crate::config::{AllBeadsConfig, BossContext};
use crate::git::BossRepo;
use crate::graph::{FederatedGraph, Rig, RigAuthStrategy};
use crate::storage::JsonlReader;
use crate::{AllBeadsError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

/// Progress event during parallel refresh
#[derive(Debug, Clone)]
pub enum RefreshProgress {
    /// Starting refresh with total count
    Starting { total: usize },
    /// Started fetching a specific repository
    FetchingRepo {
        name: String,
        index: usize,
        total: usize,
    },
    /// Successfully fetched a repository
    FetchedRepo {
        name: String,
        index: usize,
        total: usize,
    },
    /// Cloning a new repository
    CloningRepo { name: String, url: String },
    /// Successfully cloned a repository
    ClonedRepo { name: String },
    /// Failed to fetch/clone a repository (non-fatal with skip_errors)
    RepoError { name: String, error: String },
    /// All refreshes complete
    Complete {
        succeeded: usize,
        failed: usize,
        total: usize,
    },
}

/// Result of parallel refresh operation
#[derive(Debug, Default)]
pub struct RefreshResult {
    /// Number of successfully refreshed repos
    pub succeeded: usize,
    /// Number of failed repos (skipped)
    pub failed: usize,
    /// Error messages for failed repos
    pub errors: Vec<(String, String)>,
}

/// Sync mode for aggregator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    /// Only read from local cache, don't fetch
    LocalOnly,

    /// Fetch updates from remotes
    Fetch,

    /// Pull updates (fetch + merge)
    Pull,
}

/// Aggregator configuration
#[derive(Debug, Clone)]
pub struct AggregatorConfig {
    /// Sync mode
    pub sync_mode: SyncMode,

    /// Filter to specific contexts (empty = all contexts)
    pub context_filter: Vec<String>,

    /// Skip missing or inaccessible repositories
    pub skip_errors: bool,
}

impl Default for AggregatorConfig {
    fn default() -> Self {
        Self {
            sync_mode: SyncMode::Fetch,
            context_filter: Vec::new(),
            skip_errors: true,
        }
    }
}

/// Multi-Boss repository aggregator
pub struct Aggregator {
    /// Boss repositories (keyed by context name)
    repos: HashMap<String, BossRepo>,

    /// Aggregator configuration
    agg_config: AggregatorConfig,
}

impl Aggregator {
    /// Create a new aggregator from configuration
    pub fn new(config: AllBeadsConfig, agg_config: AggregatorConfig) -> Result<Self> {
        let mut repos = HashMap::new();

        // Filter contexts if needed
        let contexts: Vec<&BossContext> = if agg_config.context_filter.is_empty() {
            config.contexts.iter().collect()
        } else {
            config
                .contexts
                .iter()
                .filter(|c| agg_config.context_filter.contains(&c.name))
                .collect()
        };

        // Initialize BossRepo for each context
        for context in contexts {
            match BossRepo::from_context(context.clone()) {
                Ok(repo) => {
                    repos.insert(context.name.clone(), repo);
                }
                Err(e) => {
                    if agg_config.skip_errors {
                        tracing::warn!(
                            context = %context.name,
                            error = %e,
                            "Skipping context due to error"
                        );
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Ok(Self { repos, agg_config })
    }

    /// Sync all Boss repositories
    pub fn sync_repos(&mut self) -> Result<()> {
        match self.agg_config.sync_mode {
            SyncMode::LocalOnly => {
                tracing::debug!("Local-only mode, skipping sync");
                Ok(())
            }
            SyncMode::Fetch => {
                tracing::info!("Fetching updates from all Boss repositories");
                self.fetch_all()
            }
            SyncMode::Pull => {
                tracing::info!("Pulling updates from all Boss repositories");
                self.pull_all()
            }
        }
    }

    /// Clone all repositories that don't exist locally
    fn clone_all(&mut self) -> Result<()> {
        let mut errors = Vec::new();
        let mut cloned_count = 0;

        for (name, repo) in &mut self.repos {
            // Check if repo needs cloning (doesn't exist)
            let needs_clone = repo.status()? == crate::git::RepoStatus::NotCloned;
            if needs_clone {
                eprintln!("  📦 Cloning {} from {}...", name, repo.context().url);
            }

            if let Err(e) = repo.clone_if_needed() {
                let err_msg = format!("Failed to clone {}: {}", name, e);
                tracing::error!("{}", err_msg);
                eprintln!("  ⚠️  {}", err_msg);
                errors.push(err_msg);

                if !self.agg_config.skip_errors {
                    return Err(e);
                }
            } else if needs_clone {
                // Successfully cloned
                cloned_count += 1;
            }
        }

        if cloned_count > 0 {
            eprintln!("  ✓ Cloned {} repositories", cloned_count);
        }

        if !errors.is_empty() && !self.agg_config.skip_errors {
            return Err(AllBeadsError::Git(format!(
                "Failed to clone repositories: {}",
                errors.join(", ")
            )));
        }

        Ok(())
    }

    /// Fetch updates from all repositories
    fn fetch_all(&mut self) -> Result<()> {
        // Clone any missing repos first
        self.clone_all()?;

        let mut errors = Vec::new();

        for (name, repo) in &mut self.repos {
            if let Err(e) = repo.fetch() {
                let err_msg = format!("Failed to fetch {}: {}", name, e);
                tracing::error!("{}", err_msg);
                errors.push(err_msg);

                if !self.agg_config.skip_errors {
                    return Err(e);
                }
            }
        }

        if !errors.is_empty() && !self.agg_config.skip_errors {
            return Err(AllBeadsError::Git(format!(
                "Failed to fetch repositories: {}",
                errors.join(", ")
            )));
        }

        Ok(())
    }

    /// Pull updates from all repositories
    fn pull_all(&mut self) -> Result<()> {
        // Clone any missing repos first
        self.clone_all()?;

        let mut errors = Vec::new();

        for (name, repo) in &mut self.repos {
            if let Err(e) = repo.pull() {
                let err_msg = format!("Failed to pull {}: {}", name, e);
                tracing::error!("{}", err_msg);
                errors.push(err_msg);

                if !self.agg_config.skip_errors {
                    return Err(e);
                }
            }
        }

        if !errors.is_empty() && !self.agg_config.skip_errors {
            return Err(AllBeadsError::Git(format!(
                "Failed to pull repositories: {}",
                errors.join(", ")
            )));
        }

        Ok(())
    }

    /// Sync all repositories in parallel with progress reporting
    ///
    /// This is the recommended method for refreshing beads as it runs
    /// git operations concurrently for better performance.
    ///
    /// # Arguments
    /// * `progress_callback` - Optional callback to receive progress updates
    /// * `max_concurrent` - Maximum number of concurrent fetch operations (default: 8)
    pub async fn sync_repos_parallel<F>(
        &mut self,
        progress_callback: Option<F>,
        max_concurrent: Option<usize>,
    ) -> Result<RefreshResult>
    where
        F: Fn(RefreshProgress) + Send + Sync + 'static,
    {
        match self.agg_config.sync_mode {
            SyncMode::LocalOnly => {
                tracing::debug!("Local-only mode, skipping sync");
                return Ok(RefreshResult::default());
            }
            SyncMode::Fetch | SyncMode::Pull => {
                // Continue with parallel fetch
            }
        }

        let max_concurrent = max_concurrent.unwrap_or(8);
        let total = self.repos.len();

        // Report starting
        if let Some(ref cb) = progress_callback {
            cb(RefreshProgress::Starting { total });
        }

        // Collect repo names and contexts for parallel processing
        let repo_infos: Vec<(String, BossContext)> = self
            .repos
            .iter()
            .map(|(name, repo)| (name.clone(), repo.context().clone()))
            .collect();

        // Use a shared counter for progress
        let completed = Arc::new(TokioMutex::new(0usize));
        let results = Arc::new(TokioMutex::new(RefreshResult::default()));
        let callback = Arc::new(progress_callback);

        // Process in batches for controlled concurrency
        for chunk in repo_infos.chunks(max_concurrent) {
            let mut handles = Vec::new();

            for (name, context) in chunk {
                let name = name.clone();
                let context = context.clone();
                let completed = Arc::clone(&completed);
                let results = Arc::clone(&results);
                let callback = Arc::clone(&callback);
                let total = total;
                let is_pull = self.agg_config.sync_mode == SyncMode::Pull;
                let skip_errors = self.agg_config.skip_errors;

                let handle = tokio::task::spawn_blocking(move || {
                    // Report fetching started
                    let index = {
                        let c = completed.blocking_lock();
                        *c
                    };

                    if let Some(ref cb) = *callback {
                        cb(RefreshProgress::FetchingRepo {
                            name: name.clone(),
                            index,
                            total,
                        });
                    }

                    // Create repo and sync
                    let sync_result = (|| -> Result<()> {
                        let mut repo = BossRepo::from_context(context)?;

                        // Clone if needed
                        if !repo.path().exists()
                            || repo.status()? == crate::git::RepoStatus::NotCloned
                        {
                            if let Some(ref cb) = *callback {
                                cb(RefreshProgress::CloningRepo {
                                    name: name.clone(),
                                    url: repo.context().url.clone(),
                                });
                            }
                            repo.clone_if_needed()?;
                            if let Some(ref cb) = *callback {
                                cb(RefreshProgress::ClonedRepo { name: name.clone() });
                            }
                        }

                        // Fetch or pull
                        if is_pull {
                            repo.pull()?;
                        } else {
                            repo.fetch()?;
                        }

                        Ok(())
                    })();

                    // Update results
                    let mut res = results.blocking_lock();
                    let mut comp = completed.blocking_lock();
                    *comp += 1;

                    match sync_result {
                        Ok(()) => {
                            res.succeeded += 1;
                            if let Some(ref cb) = *callback {
                                cb(RefreshProgress::FetchedRepo {
                                    name: name.clone(),
                                    index: *comp,
                                    total,
                                });
                            }
                        }
                        Err(e) => {
                            let error_msg = format!("{}", e);
                            res.failed += 1;
                            res.errors.push((name.clone(), error_msg.clone()));

                            if let Some(ref cb) = *callback {
                                cb(RefreshProgress::RepoError {
                                    name: name.clone(),
                                    error: error_msg,
                                });
                            }

                            if !skip_errors {
                                return Err(e);
                            }
                        }
                    }

                    Ok(())
                });

                handles.push(handle);
            }

            // Wait for all handles in this batch
            for handle in handles {
                if let Err(e) = handle.await {
                    tracing::error!("Task panicked: {}", e);
                }
            }
        }

        // Get final results
        let final_results = Arc::try_unwrap(results)
            .map_err(|_| AllBeadsError::Git("Failed to unwrap results".to_string()))?
            .into_inner();

        // Report completion
        if let Some(ref cb) = *callback {
            cb(RefreshProgress::Complete {
                succeeded: final_results.succeeded,
                failed: final_results.failed,
                total,
            });
        }

        // Reload repos after parallel sync (they may have been cloned)
        self.reload_repos()?;

        Ok(final_results)
    }

    /// Reload all repos from the current configuration
    ///
    /// This is needed after parallel sync because repos may have been cloned
    fn reload_repos(&mut self) -> Result<()> {
        for (name, repo) in &mut self.repos {
            let context = repo.context().clone();
            match BossRepo::from_context(context) {
                Ok(new_repo) => {
                    *repo = new_repo;
                }
                Err(e) => {
                    if !self.agg_config.skip_errors {
                        return Err(e);
                    }
                    tracing::warn!(
                        context = %name,
                        error = %e,
                        "Failed to reload repo after sync"
                    );
                }
            }
        }
        Ok(())
    }

    /// Aggregate all Boss repositories into a FederatedGraph
    pub fn aggregate(&mut self) -> Result<FederatedGraph> {
        // Ensure repos are synced
        self.sync_repos()?;

        let mut graph = FederatedGraph::new();

        // Load beads from each Boss repository
        for (context_name, repo) in &self.repos {
            if !repo.has_issues_jsonl() {
                tracing::debug!(
                    context = %context_name,
                    "No issues.jsonl found, skipping"
                );
                continue;
            }

            tracing::info!(
                context = %context_name,
                path = %repo.issues_jsonl_path().display(),
                "Loading beads from Boss repository"
            );

            // Read beads from issues.jsonl
            let mut reader = JsonlReader::open(repo.issues_jsonl_path())?;
            let beads: Vec<crate::graph::Bead> = reader.read_all()?;

            tracing::debug!(
                context = %context_name,
                count = beads.len(),
                "Loaded beads"
            );

            // Add beads to graph with context information
            for bead in beads {
                let mut bead = bead;
                // Tag bead with context
                let label = format!("@{}", context_name);
                bead.add_label(label);

                graph.add_bead(bead);
            }

            // Create a Rig for this Boss repository
            let rig = Rig::builder()
                .id(format!("boss-{}", context_name))
                .path(repo.path().to_string_lossy().to_string())
                .remote(repo.context().url.clone())
                .auth_strategy(RigAuthStrategy::SshAgent) // TODO: Map from BossContext auth
                .prefix("beads")
                .context(context_name.clone())
                .build()?;

            // Add rig to graph
            graph.add_rig(rig);
        }

        tracing::info!(
            total_beads = graph.stats().total_beads,
            "Aggregation complete"
        );

        Ok(graph)
    }

    /// Aggregate all Boss repositories into a FederatedGraph using parallel sync
    ///
    /// This is the recommended method for aggregation as it uses parallel
    /// git operations for better performance.
    ///
    /// # Arguments
    /// * `progress_callback` - Optional callback to receive progress updates during sync
    pub async fn aggregate_parallel<F>(
        &mut self,
        progress_callback: Option<F>,
    ) -> Result<FederatedGraph>
    where
        F: Fn(RefreshProgress) + Send + Sync + 'static,
    {
        // Sync repos in parallel
        self.sync_repos_parallel(progress_callback, None).await?;

        // Load beads (this part is fast, no need to parallelize)
        self.load_beads_into_graph()
    }

    /// Load beads from all repos into a FederatedGraph (no sync)
    ///
    /// This is useful when you've already synced and just want to load.
    pub fn load_beads_into_graph(&self) -> Result<FederatedGraph> {
        let mut graph = FederatedGraph::new();

        // Load beads from each Boss repository
        for (context_name, repo) in &self.repos {
            if !repo.has_issues_jsonl() {
                tracing::debug!(
                    context = %context_name,
                    "No issues.jsonl found, skipping"
                );
                continue;
            }

            tracing::debug!(
                context = %context_name,
                path = %repo.issues_jsonl_path().display(),
                "Loading beads from Boss repository"
            );

            // Read beads from issues.jsonl
            let mut reader = JsonlReader::open(repo.issues_jsonl_path())?;
            let beads: Vec<crate::graph::Bead> = reader.read_all()?;

            tracing::debug!(
                context = %context_name,
                count = beads.len(),
                "Loaded beads"
            );

            // Add beads to graph with context information
            for bead in beads {
                let mut bead = bead;
                // Tag bead with context
                let label = format!("@{}", context_name);
                bead.add_label(label);

                graph.add_bead(bead);
            }

            // Create a Rig for this Boss repository
            let rig = Rig::builder()
                .id(format!("boss-{}", context_name))
                .path(repo.path().to_string_lossy().to_string())
                .remote(repo.context().url.clone())
                .auth_strategy(RigAuthStrategy::SshAgent)
                .prefix("beads")
                .context(context_name.clone())
                .build()?;

            // Add rig to graph
            graph.add_rig(rig);
        }

        tracing::info!(
            total_beads = graph.stats().total_beads,
            "Loaded beads into graph"
        );

        Ok(graph)
    }

    /// Get Boss repository by context name
    pub fn get_repo(&self, context_name: &str) -> Option<&BossRepo> {
        self.repos.get(context_name)
    }

    /// Get all Boss repositories
    pub fn repos(&self) -> &HashMap<String, BossRepo> {
        &self.repos
    }

    /// Get aggregator configuration
    pub fn config(&self) -> &AggregatorConfig {
        &self.agg_config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AllBeadsConfig, AuthStrategy, BossContext};

    #[test]
    fn test_aggregator_creation() {
        let mut config = AllBeadsConfig::new();
        let context = BossContext::new(
            "test",
            "https://github.com/test/boss.git",
            AuthStrategy::SshAgent,
        );
        config.add_context(context);

        let agg_config = AggregatorConfig {
            sync_mode: SyncMode::LocalOnly,
            skip_errors: true,
            ..Default::default()
        };

        let aggregator = Aggregator::new(config, agg_config).unwrap();
        assert_eq!(aggregator.repos().len(), 1);
    }

    #[test]
    fn test_aggregator_context_filter() {
        let mut config = AllBeadsConfig::new();

        config.add_context(BossContext::new(
            "work",
            "https://github.com/work/boss.git",
            AuthStrategy::SshAgent,
        ));

        config.add_context(BossContext::new(
            "personal",
            "git@github.com:user/boss.git",
            AuthStrategy::SshAgent,
        ));

        let agg_config = AggregatorConfig {
            sync_mode: SyncMode::LocalOnly,
            context_filter: vec!["work".to_string()],
            skip_errors: true,
        };

        let aggregator = Aggregator::new(config, agg_config).unwrap();
        assert_eq!(aggregator.repos().len(), 1);
        assert!(aggregator.get_repo("work").is_some());
        assert!(aggregator.get_repo("personal").is_none());
    }

    #[test]
    fn test_sync_mode_variants() {
        assert_eq!(SyncMode::LocalOnly, SyncMode::LocalOnly);
        assert_ne!(SyncMode::LocalOnly, SyncMode::Fetch);
    }
}
