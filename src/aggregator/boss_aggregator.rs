//! Boss repository aggregator implementation

use crate::config::{AllBeadsConfig, BossContext};
use crate::git::BossRepo;
use crate::graph::{FederatedGraph, Rig, RigAuthStrategy};
use crate::storage::JsonlReader;
use crate::{AllBeadsError, Result};
use std::collections::HashMap;

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

        for (name, repo) in &mut self.repos {
            if let Err(e) = repo.clone_if_needed() {
                let err_msg = format!("Failed to clone {}: {}", name, e);
                tracing::error!("{}", err_msg);
                errors.push(err_msg);

                if !self.agg_config.skip_errors {
                    return Err(e);
                }
            }
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

    /// Aggregate all Boss repositories into a FederatedGraph
    pub fn aggregate(&mut self) -> Result<FederatedGraph> {
        // Ensure repos are synced
        self.sync_repos()?;

        let mut graph = FederatedGraph::new();

        // Load beads from each Boss repository
        for (context_name, repo) in &self.repos {
            if !repo.has_issues_jsonl() {
                tracing::warn!(
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
