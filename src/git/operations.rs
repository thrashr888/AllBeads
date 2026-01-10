//! Git repository operations for Boss repos

use crate::config::{AuthStrategy, BossContext};
use crate::{AllBeadsError, Result};
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository};
use std::path::{Path, PathBuf};

/// Repository status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoStatus {
    /// Repository doesn't exist locally
    NotCloned,

    /// Repository exists and is up to date
    UpToDate,

    /// Repository exists but has pending updates
    UpdatesAvailable,

    /// Repository exists but has uncommitted changes
    Dirty,
}

/// Git credentials configuration
#[derive(Debug, Clone, Default)]
pub struct GitCredentials {
    /// SSH key path (for SSH agent)
    pub ssh_key_path: Option<PathBuf>,

    /// Personal access token
    pub token: Option<String>,

    /// Username for token authentication
    pub username: Option<String>,
}

impl GitCredentials {
    /// Create credentials from a Boss context
    pub fn from_context(context: &BossContext) -> Result<Self> {
        match context.auth_strategy {
            AuthStrategy::SshAgent => {
                // SSH agent will be used automatically by git2
                Ok(Self {
                    ssh_key_path: None,
                    token: None,
                    username: None,
                })
            }
            AuthStrategy::GhEnterpriseToken | AuthStrategy::PersonalAccessToken => {
                // Look for token in env_vars first
                let mut token = context
                    .env_vars
                    .values()
                    .find(|v| v.starts_with('$'))
                    .and_then(|v| {
                        let env_var = v.trim_start_matches('$');
                        std::env::var(env_var).ok()
                    });

                // Try GITHUB_TOKEN env var as fallback
                if token.is_none() {
                    token = std::env::var("GITHUB_TOKEN").ok();
                }

                // Try `gh auth token` as final fallback
                if token.is_none() {
                    if let Ok(output) = std::process::Command::new("gh")
                        .args(["auth", "token"])
                        .output()
                    {
                        if output.status.success() {
                            let gh_token =
                                String::from_utf8_lossy(&output.stdout).trim().to_string();
                            if !gh_token.is_empty() {
                                tracing::debug!("Using token from `gh auth token`");
                                token = Some(gh_token);
                            }
                        }
                    }
                }

                if token.is_none() {
                    tracing::warn!(
                        context = %context.name,
                        "No token found for HTTPS auth. Try: gh auth login, or set GITHUB_TOKEN"
                    );
                }

                Ok(Self {
                    ssh_key_path: None,
                    token,
                    username: Some("git".to_string()),
                })
            }
        }
    }

    /// Create callback for git2 authentication
    fn create_callbacks(&self) -> RemoteCallbacks<'_> {
        let mut callbacks = RemoteCallbacks::new();

        let token = self.token.clone();
        let username = self.username.clone();

        callbacks.credentials(move |_url, username_from_url, _allowed_types| {
            tracing::debug!(url = _url, "Git credentials callback invoked");

            // Try SSH agent first if no token
            if token.is_none() {
                if let Some(username) = username_from_url {
                    return Cred::ssh_key_from_agent(username);
                }
            }

            // Try token authentication
            if let Some(ref token) = token {
                let user = username.as_deref().or(username_from_url).unwrap_or("git");
                return Cred::userpass_plaintext(user, token);
            }

            // Fallback to default credentials
            Cred::default()
        });

        callbacks
    }
}

/// Boss repository wrapper
pub struct BossRepo {
    /// Local path to repository
    path: PathBuf,

    /// Git repository handle
    repo: Option<Repository>,

    /// Boss context configuration
    context: BossContext,

    /// Git credentials
    credentials: GitCredentials,
}

impl BossRepo {
    /// Open or clone a Boss repository from a context
    pub fn from_context(context: BossContext) -> Result<Self> {
        let path = context.get_path();
        let credentials = GitCredentials::from_context(&context)?;

        let repo = if path.exists() {
            Some(Repository::open(&path)?)
        } else {
            None
        };

        Ok(Self {
            path,
            repo,
            context,
            credentials,
        })
    }

    /// Open a local repository without authentication (for local-only operations)
    ///
    /// This is useful for operations that don't require remote access like
    /// staging files and committing changes.
    pub fn from_local(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();

        let repo = Repository::open(&path).map_err(|e| {
            AllBeadsError::Git(format!(
                "Failed to open repository at {}: {}",
                path.display(),
                e
            ))
        })?;

        // Create a minimal context for the local repo
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("local")
            .to_string();

        let context = BossContext::new(&name, "", AuthStrategy::SshAgent).with_path(&path);

        Ok(Self {
            path,
            repo: Some(repo),
            context,
            credentials: GitCredentials::default(),
        })
    }

    /// Get repository status
    pub fn status(&self) -> Result<RepoStatus> {
        if let Some(ref repo) = self.repo {
            // Check if there are uncommitted changes
            let statuses = repo.statuses(None)?;
            if !statuses.is_empty() {
                return Ok(RepoStatus::Dirty);
            }

            // Check if there are updates available
            // This requires fetching, which we'll do separately
            Ok(RepoStatus::UpToDate)
        } else {
            Ok(RepoStatus::NotCloned)
        }
    }

    /// Clone the repository if it doesn't exist
    pub fn clone_if_needed(&mut self) -> Result<()> {
        if self.repo.is_some() {
            tracing::debug!(path = %self.path.display(), "Repository already exists");
            return Ok(());
        }

        tracing::info!(
            context = %self.context.name,
            url = %self.context.url,
            path = %self.path.display(),
            "Cloning Boss repository"
        );

        // Create parent directory
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Set up fetch options with credentials
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(self.credentials.create_callbacks());

        // Clone the repository
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);

        let repo = builder.clone(&self.context.url, &self.path)?;

        tracing::info!(path = %self.path.display(), "Repository cloned successfully");

        self.repo = Some(repo);
        Ok(())
    }

    /// Fetch updates from remote
    pub fn fetch(&mut self) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| AllBeadsError::Git("Repository not cloned yet".to_string()))?;

        tracing::debug!(context = %self.context.name, "Fetching updates from remote");

        let mut remote = repo.find_remote("origin")?;

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(self.credentials.create_callbacks());

        remote.fetch(&["main", "master"], Some(&mut fetch_options), None)?;

        tracing::debug!("Fetch completed successfully");
        Ok(())
    }

    /// Pull updates from remote (fetch + merge)
    pub fn pull(&mut self) -> Result<()> {
        self.fetch()?;

        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| AllBeadsError::Git("Repository not cloned yet".to_string()))?;

        // Find the current branch
        let head = repo.head()?;
        let branch_name = head
            .shorthand()
            .ok_or_else(|| AllBeadsError::Git("Could not determine current branch".to_string()))?;

        // Find the upstream branch
        let upstream_name = format!("origin/{}", branch_name);
        let upstream_ref = repo.find_reference(&upstream_name)?;
        let upstream_commit = upstream_ref.peel_to_commit()?;

        // Fast-forward merge
        let mut checkout_builder = git2::build::CheckoutBuilder::new();
        checkout_builder.force();

        repo.checkout_tree(upstream_commit.as_object(), Some(&mut checkout_builder))?;

        // Update HEAD
        repo.head()?.set_target(
            upstream_commit.id(),
            &format!("Fast-forward to {}", upstream_name),
        )?;

        tracing::info!(context = %self.context.name, "Pulled updates successfully");
        Ok(())
    }

    /// Get path to .beads directory
    pub fn beads_dir(&self) -> PathBuf {
        self.path.join(".beads")
    }

    /// Check if .beads directory exists
    pub fn has_beads_dir(&self) -> bool {
        self.beads_dir().exists()
    }

    /// Get path to issues.jsonl file
    pub fn issues_jsonl_path(&self) -> PathBuf {
        self.beads_dir().join("issues.jsonl")
    }

    /// Check if issues.jsonl exists
    pub fn has_issues_jsonl(&self) -> bool {
        self.issues_jsonl_path().exists()
    }

    /// Get the Boss context
    pub fn context(&self) -> &BossContext {
        &self.context
    }

    /// Get the local path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Stage files for commit
    ///
    /// # Arguments
    /// * `paths` - Paths to stage (relative to repo root)
    pub fn add(&self, paths: &[&Path]) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| AllBeadsError::Git("Repository not cloned yet".to_string()))?;

        let mut index = repo.index()?;

        for path in paths {
            tracing::debug!(path = %path.display(), "Staging file");
            index.add_path(path)?;
        }

        index.write()?;
        tracing::debug!("Staged {} files", paths.len());
        Ok(())
    }

    /// Stage all changes in .beads directory
    pub fn add_beads(&self) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| AllBeadsError::Git("Repository not cloned yet".to_string()))?;

        let mut index = repo.index()?;

        // Add all files in .beads directory
        index.add_all([".beads/*"], git2::IndexAddOption::DEFAULT, None)?;

        index.write()?;
        tracing::debug!("Staged .beads directory");
        Ok(())
    }

    /// Create a commit with the staged changes
    ///
    /// # Arguments
    /// * `message` - Commit message
    /// * `author_name` - Author name
    /// * `author_email` - Author email
    pub fn commit(
        &self,
        message: &str,
        author_name: &str,
        author_email: &str,
    ) -> Result<git2::Oid> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| AllBeadsError::Git("Repository not cloned yet".to_string()))?;

        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        let signature = git2::Signature::now(author_name, author_email)?;

        // Get parent commit (HEAD)
        let parent = match repo.head() {
            Ok(head) => Some(head.peel_to_commit()?),
            Err(_) => None, // No commits yet
        };

        let parents: Vec<&git2::Commit> = parent.as_ref().map(|p| vec![p]).unwrap_or_default();

        let commit_id = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )?;

        tracing::info!(commit = %commit_id, message = %message, "Created commit");
        Ok(commit_id)
    }

    /// Push changes to remote
    ///
    /// # Arguments
    /// * `branch` - Branch name to push (defaults to current branch)
    pub fn push(&self, branch: Option<&str>) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| AllBeadsError::Git("Repository not cloned yet".to_string()))?;

        let branch_name = if let Some(b) = branch {
            b.to_string()
        } else {
            // Get current branch
            let head = repo.head()?;
            head.shorthand()
                .ok_or_else(|| {
                    AllBeadsError::Git("Could not determine current branch".to_string())
                })?
                .to_string()
        };

        tracing::info!(context = %self.context.name, branch = %branch_name, "Pushing to remote");

        let mut remote = repo.find_remote("origin")?;

        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);

        let mut push_options = git2::PushOptions::new();
        push_options.remote_callbacks(self.credentials.create_callbacks());

        remote.push(&[&refspec], Some(&mut push_options))?;

        tracing::info!(context = %self.context.name, branch = %branch_name, "Push completed");
        Ok(())
    }

    /// Create a new branch
    ///
    /// # Arguments
    /// * `name` - Branch name
    /// * `from` - Optional commit to branch from (defaults to HEAD)
    pub fn create_branch(&self, name: &str, from: Option<git2::Oid>) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| AllBeadsError::Git("Repository not cloned yet".to_string()))?;

        let commit = if let Some(oid) = from {
            repo.find_commit(oid)?
        } else {
            repo.head()?.peel_to_commit()?
        };

        repo.branch(name, &commit, false)?;

        tracing::info!(branch = %name, "Created branch");
        Ok(())
    }

    /// Checkout a branch
    ///
    /// # Arguments
    /// * `name` - Branch name to checkout
    pub fn checkout_branch(&self, name: &str) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| AllBeadsError::Git("Repository not cloned yet".to_string()))?;

        let refname = format!("refs/heads/{}", name);
        let obj = repo.revparse_single(&refname)?;

        let mut checkout_builder = git2::build::CheckoutBuilder::new();
        checkout_builder.force();

        repo.checkout_tree(&obj, Some(&mut checkout_builder))?;
        repo.set_head(&refname)?;

        tracing::info!(branch = %name, "Checked out branch");
        Ok(())
    }

    /// Check if there are uncommitted changes
    pub fn has_changes(&self) -> Result<bool> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| AllBeadsError::Git("Repository not cloned yet".to_string()))?;

        let statuses = repo.statuses(None)?;
        Ok(!statuses.is_empty())
    }

    /// Get list of changed files
    pub fn changed_files(&self) -> Result<Vec<PathBuf>> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| AllBeadsError::Git("Repository not cloned yet".to_string()))?;

        let statuses = repo.statuses(None)?;
        let mut files = Vec::new();

        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                files.push(PathBuf::from(path));
            }
        }

        Ok(files)
    }

    /// Sync beads to remote (add, commit, push)
    ///
    /// Convenience method that stages .beads, commits with a message, and pushes.
    pub fn sync_beads(&self, message: &str) -> Result<()> {
        if !self.has_changes()? {
            tracing::debug!(context = %self.context.name, "No changes to sync");
            return Ok(());
        }

        self.add_beads()?;
        self.commit(message, "AllBeads Sheriff", "sheriff@allbeads.local")?;
        self.push(None)?;

        tracing::info!(context = %self.context.name, "Synced beads to remote");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BossContext;

    #[test]
    fn test_git_credentials_from_ssh_context() {
        let context = BossContext::new(
            "test",
            "git@github.com:user/repo.git",
            AuthStrategy::SshAgent,
        );

        let creds = GitCredentials::from_context(&context).unwrap();
        assert!(creds.ssh_key_path.is_none());
        assert!(creds.token.is_none());
    }

    #[test]
    fn test_git_credentials_from_token_context() {
        std::env::set_var("TEST_TOKEN", "test_token_value");

        let context = BossContext::new(
            "test",
            "https://github.com/user/repo.git",
            AuthStrategy::PersonalAccessToken,
        )
        .with_env_var("GITHUB_TOKEN", "$TEST_TOKEN");

        let creds = GitCredentials::from_context(&context).unwrap();
        assert!(creds.token.is_some());
        assert_eq!(creds.token.unwrap(), "test_token_value");

        std::env::remove_var("TEST_TOKEN");
    }

    #[test]
    fn test_repo_status_not_cloned() {
        let context = BossContext::new(
            "test",
            "https://github.com/user/repo.git",
            AuthStrategy::SshAgent,
        )
        .with_path("/tmp/nonexistent/allbeads/test");

        let repo = BossRepo::from_context(context).unwrap();
        assert_eq!(repo.status().unwrap(), RepoStatus::NotCloned);
    }

    #[test]
    fn test_beads_dir_path() {
        let context = BossContext::new(
            "test",
            "https://github.com/user/repo.git",
            AuthStrategy::SshAgent,
        )
        .with_path("/tmp/test/boss");

        let repo = BossRepo::from_context(context).unwrap();
        assert_eq!(repo.beads_dir(), PathBuf::from("/tmp/test/boss/.beads"));
        assert_eq!(
            repo.issues_jsonl_path(),
            PathBuf::from("/tmp/test/boss/.beads/issues.jsonl")
        );
    }
}
