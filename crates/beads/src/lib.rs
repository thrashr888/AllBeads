//! Beads (bd) issue tracker wrapper for Rust
//!
//! A type-safe interface to the bd CLI for git-native issue tracking.
//!
//! # Example
//!
//! ```no_run
//! use beads::Beads;
//!
//! let bd = Beads::new()?;
//!
//! // List issues
//! let issues = bd.list(None, None)?;
//!
//! // Show ready issues
//! let ready = bd.ready()?;
//!
//! // Create an issue
//! bd.create("Fix the bug", "bug", Some(2), None)?;
//!
//! // Get stats
//! let stats = bd.stats()?;
//! println!("Open: {}, Closed: {}", stats.open, stats.closed);
//! # Ok::<(), beads::Error>(())
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

/// Errors that can occur when interacting with beads
#[derive(Error, Debug)]
pub enum Error {
    #[error("bd is not installed or not in PATH")]
    NotInstalled,

    #[error("Not in a beads-enabled repository")]
    NotInRepo,

    #[error("Failed to execute bd command: {0}")]
    CommandFailed(String),

    #[error("Failed to parse output: {0}")]
    ParseError(String),

    #[error("Issue not found: {0}")]
    IssueNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result type for beads operations
pub type Result<T> = std::result::Result<T, Error>;

/// Issue status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed,
    Tombstone,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Open => write!(f, "open"),
            Status::InProgress => write!(f, "in_progress"),
            Status::Blocked => write!(f, "blocked"),
            Status::Deferred => write!(f, "deferred"),
            Status::Closed => write!(f, "closed"),
            Status::Tombstone => write!(f, "tombstone"),
        }
    }
}

/// Issue type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    Bug,
    Feature,
    Task,
    Epic,
    Chore,
    MergeRequest,
    Molecule,
    Gate,
}

impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueType::Bug => write!(f, "bug"),
            IssueType::Feature => write!(f, "feature"),
            IssueType::Task => write!(f, "task"),
            IssueType::Epic => write!(f, "epic"),
            IssueType::Chore => write!(f, "chore"),
            IssueType::MergeRequest => write!(f, "merge_request"),
            IssueType::Molecule => write!(f, "molecule"),
            IssueType::Gate => write!(f, "gate"),
        }
    }
}

/// A dependency reference (used in bd show --json output)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyRef {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub dependency_type: Option<String>,
}

/// A beads issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub status: String,
    #[serde(rename = "issue_type", alias = "type")]
    pub issue_type: String,
    #[serde(default)]
    pub priority: Option<u8>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    /// Rich dependency objects from bd show --json
    #[serde(default)]
    pub dependencies: Vec<DependencyRef>,
    /// Simple list of issue IDs this depends on (from bd list)
    #[serde(default, alias = "blocked_by")]
    pub depends_on: Vec<String>,
    /// Beads blocked by this (from bd show --json as objects, or bd list as strings)
    #[serde(default, alias = "dependents", deserialize_with = "deserialize_blocks")]
    pub blocks: Vec<DependencyRef>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl Issue {
    /// Get all blocker IDs (from either dependencies or depends_on)
    pub fn blocker_ids(&self) -> Vec<String> {
        if !self.dependencies.is_empty() {
            self.dependencies
                .iter()
                .filter(|d| d.dependency_type.as_deref() == Some("blocks"))
                .map(|d| d.id.clone())
                .collect()
        } else {
            self.depends_on.clone()
        }
    }
}

/// Custom deserializer for blocks field that handles both strings and objects
fn deserialize_blocks<'de, D>(deserializer: D) -> std::result::Result<Vec<DependencyRef>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, SeqAccess, Visitor};
    use std::fmt;

    struct BlocksVisitor;

    impl<'de> Visitor<'de> for BlocksVisitor {
        type Value = Vec<DependencyRef>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a sequence of strings or dependency objects")
        }

        fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Vec<DependencyRef>, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut deps = Vec::new();

            while let Some(value) = seq.next_element::<serde_json::Value>()? {
                match value {
                    serde_json::Value::String(s) => {
                        deps.push(DependencyRef {
                            id: s,
                            title: None,
                            status: None,
                            dependency_type: None,
                        });
                    }
                    serde_json::Value::Object(_) => {
                        let dep: DependencyRef = serde_json::from_value(value)
                            .map_err(de::Error::custom)?;
                        deps.push(dep);
                    }
                    _ => {
                        return Err(de::Error::custom(
                            "expected string or object in blocks array",
                        ));
                    }
                }
            }

            Ok(deps)
        }
    }

    deserializer.deserialize_seq(BlocksVisitor)
}

/// A comment on an issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    #[serde(default)]
    pub id: Option<String>,
    pub author: String,
    pub content: String,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Project statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Stats {
    #[serde(default)]
    pub total: usize,
    #[serde(default)]
    pub open: usize,
    #[serde(default)]
    pub in_progress: usize,
    #[serde(default)]
    pub closed: usize,
    #[serde(default)]
    pub blocked: usize,
    #[serde(default)]
    pub epics: usize,
}

/// Activity log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub timestamp: String,
    pub action: String,
    #[serde(default)]
    pub issue_id: Option<String>,
    #[serde(default)]
    pub details: Option<String>,
}

/// Output from a bd command
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    /// Get combined stdout and stderr output
    pub fn combined(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

/// Status info for display
#[derive(Debug, Clone, Default)]
pub struct StatusInfo {
    pub open: usize,
    pub in_progress: usize,
    pub blocked: usize,
    pub ready: usize,
}

/// Beads CLI wrapper
#[derive(Debug, Clone, Default)]
pub struct Beads {
    /// Working directory
    workdir: Option<PathBuf>,
    /// Global flags to pass to all bd commands
    global_flags: Vec<String>,
}

impl Beads {
    /// Create a new Beads instance
    pub fn new() -> Result<Self> {
        let bd = Self::default();
        if !bd.is_available() {
            return Err(Error::NotInstalled);
        }
        Ok(bd)
    }

    /// Create with a specific working directory
    pub fn with_workdir(path: impl Into<PathBuf>) -> Self {
        Self {
            workdir: Some(path.into()),
            global_flags: Vec::new(),
        }
    }

    /// Create with working directory and global flags
    pub fn with_workdir_and_flags(path: impl Into<PathBuf>, flags: Vec<String>) -> Self {
        Self {
            workdir: Some(path.into()),
            global_flags: flags,
        }
    }

    /// Set the working directory
    pub fn set_workdir(&mut self, path: impl Into<PathBuf>) {
        self.workdir = Some(path.into());
    }

    /// Set global flags to pass to all bd commands
    pub fn set_global_flags(&mut self, flags: Vec<String>) {
        self.global_flags = flags;
    }

    /// Add a global flag
    pub fn add_global_flag(&mut self, flag: String) {
        self.global_flags.push(flag);
    }

    /// Check if bd is available
    pub fn is_available(&self) -> bool {
        self.run_command(&["--version"]).is_ok()
    }

    /// Check if current directory has beads initialized
    pub fn is_repo(&self) -> bool {
        self.run_command(&["stats"]).is_ok()
    }

    // --- List operations ---

    /// List issues with optional status and type filters
    pub fn list(&self, status: Option<&str>, issue_type: Option<&str>) -> Result<Vec<Issue>> {
        let mut args = vec!["list"];

        if let Some(s) = status {
            args.push("--status");
            args.push(s);
        }

        if let Some(t) = issue_type {
            args.push("--type");
            args.push(t);
        }

        args.push("--json");

        let output = self.run_command(&args)?;
        serde_json::from_str(&output.stdout).map_err(Error::from)
    }

    /// List all open issues
    pub fn list_open(&self) -> Result<Vec<Issue>> {
        self.list(Some("open"), None)
    }

    /// List in-progress issues
    pub fn list_in_progress(&self) -> Result<Vec<Issue>> {
        self.list(Some("in_progress"), None)
    }

    /// List closed issues
    pub fn list_closed(&self) -> Result<Vec<Issue>> {
        self.list(Some("closed"), None)
    }

    /// List epics
    pub fn list_epics(&self) -> Result<Vec<Issue>> {
        self.list(None, Some("epic"))
    }

    /// List open epics
    pub fn list_open_epics(&self) -> Result<Vec<Issue>> {
        self.list(Some("open"), Some("epic"))
    }

    /// Get ready issues (no blockers)
    pub fn ready(&self) -> Result<Vec<Issue>> {
        let output = self.run_command(&["ready", "--json"])?;
        serde_json::from_str(&output.stdout).map_err(Error::from)
    }

    /// Get blocked issues
    pub fn blocked(&self) -> Result<Vec<Issue>> {
        let output = self.run_command(&["blocked", "--json"])?;
        serde_json::from_str(&output.stdout).map_err(Error::from)
    }

    // --- Issue details ---

    /// Show a specific issue
    pub fn show(&self, id: &str) -> Result<Issue> {
        let output = self.run_command(&["show", id, "--json"])?;
        // bd show returns an array with a single issue
        let issues: Vec<Issue> = serde_json::from_str(&output.stdout)?;
        issues
            .into_iter()
            .next()
            .ok_or_else(|| Error::IssueNotFound(id.to_string()))
    }

    /// Search for issues by query
    pub fn search(&self, query: &str) -> Result<Vec<Issue>> {
        let output = self.run_command(&["search", query, "--json"])?;
        serde_json::from_str(&output.stdout).map_err(Error::from)
    }

    // --- Issue creation ---

    /// Create a new issue
    pub fn create(
        &self,
        title: &str,
        issue_type: &str,
        priority: Option<u8>,
        parent: Option<&str>,
    ) -> Result<CommandOutput> {
        let mut args = vec!["create", "--title", title, "--type", issue_type];

        let priority_str;
        if let Some(p) = priority {
            priority_str = p.to_string();
            args.extend(["--priority", &priority_str]);
        }

        if let Some(parent_id) = parent {
            args.extend(["--parent", parent_id]);
        }

        self.run_command(&args)
    }

    /// Create an issue with full options
    #[allow(clippy::too_many_arguments)]
    pub fn create_full(
        &self,
        title: &str,
        issue_type: &str,
        priority: Option<u8>,
        description: Option<&str>,
        assignee: Option<&str>,
        parent: Option<&str>,
        labels: Option<&[&str]>,
    ) -> Result<CommandOutput> {
        let mut args = vec!["create", "--title", title, "--type", issue_type];

        let priority_str;
        if let Some(p) = priority {
            priority_str = p.to_string();
            args.extend(["--priority", &priority_str]);
        }

        if let Some(desc) = description {
            args.extend(["--description", desc]);
        }

        if let Some(user) = assignee {
            args.extend(["--assignee", user]);
        }

        if let Some(parent_id) = parent {
            args.extend(["--parent", parent_id]);
        }

        if let Some(label_list) = labels {
            for label in label_list {
                args.extend(["--label", label]);
            }
        }

        self.run_command(&args)
    }

    /// Create an epic
    pub fn create_epic(&self, title: &str, priority: Option<u8>) -> Result<CommandOutput> {
        self.create(title, "epic", priority, None)
    }

    /// Create a child issue under a parent
    pub fn create_child(
        &self,
        title: &str,
        issue_type: &str,
        parent_id: &str,
        priority: Option<u8>,
    ) -> Result<CommandOutput> {
        let output = self.create(title, issue_type, priority, Some(parent_id))?;

        // If successful, add dependency relationship
        if output.success {
            // Extract new issue ID from output
            if let Some(new_id) = self.extract_issue_id(&output.stdout) {
                let _ = self.dep_add(&new_id, parent_id);
            }
        }

        Ok(output)
    }

    // --- Issue updates ---

    /// Update an issue's status
    pub fn update_status(&self, id: &str, status: &str) -> Result<CommandOutput> {
        self.run_command(&["update", id, &format!("--status={}", status)])
    }

    /// Update an issue with various options
    pub fn update(
        &self,
        id: &str,
        status: Option<&str>,
        priority: Option<u8>,
        assignee: Option<&str>,
        title: Option<&str>,
    ) -> Result<CommandOutput> {
        let mut args = vec!["update".to_string(), id.to_string()];

        if let Some(s) = status {
            args.push(format!("--status={}", s));
        }

        if let Some(p) = priority {
            args.push(format!("--priority={}", p));
        }

        if let Some(a) = assignee {
            args.push(format!("--assignee={}", a));
        }

        if let Some(t) = title {
            args.push(format!("--title={}", t));
        }

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.run_command(&args_refs)
    }

    /// Close an issue
    pub fn close(&self, id: &str) -> Result<CommandOutput> {
        self.run_command(&["close", id])
    }

    /// Close an issue with a reason
    pub fn close_with_reason(&self, id: &str, reason: &str) -> Result<CommandOutput> {
        self.run_command(&["close", id, "--reason", reason])
    }

    /// Close multiple issues at once
    pub fn close_multiple(&self, ids: &[&str]) -> Result<CommandOutput> {
        let mut args = vec!["close"];
        args.extend(ids);
        self.run_command(&args)
    }

    /// Reopen a closed issue
    pub fn reopen(&self, id: &str) -> Result<CommandOutput> {
        self.run_command(&["reopen", id])
    }

    /// Reopen multiple closed issues
    pub fn reopen_multiple(&self, ids: &[&str]) -> Result<CommandOutput> {
        let mut args = vec!["reopen"];
        args.extend(ids);
        self.run_command(&args)
    }

    /// Delete an issue
    pub fn delete(&self, id: &str) -> Result<CommandOutput> {
        self.run_command(&["delete", id, "--force"])
    }

    /// Delete multiple issues
    pub fn delete_multiple(&self, ids: &[&str]) -> Result<CommandOutput> {
        let mut args = vec!["delete"];
        args.extend(ids);
        args.push("--force");
        self.run_command(&args)
    }

    /// Mark an issue as a duplicate of another
    pub fn duplicate(&self, issue: &str, duplicate_of: &str) -> Result<CommandOutput> {
        self.run_command(&["duplicate", issue, duplicate_of])
    }

    /// Quick create - create an issue and return just the ID
    pub fn quick_create(&self, title: &str) -> Result<String> {
        let output = self.run_command(&["q", title])?;
        // bd q outputs just the ID
        Ok(output.stdout.trim().to_string())
    }

    /// Quick create with type and priority
    pub fn quick_create_full(
        &self,
        title: &str,
        issue_type: Option<&str>,
        priority: Option<u8>,
    ) -> Result<String> {
        let mut args = vec!["q", title];

        let priority_str;
        if let Some(t) = issue_type {
            args.extend(["--type", t]);
        }
        if let Some(p) = priority {
            priority_str = p.to_string();
            args.extend(["--priority", &priority_str]);
        }

        let output = self.run_command(&args)?;
        Ok(output.stdout.trim().to_string())
    }

    // --- Dependencies ---

    /// Add a dependency (issue depends on depends_on)
    pub fn dep_add(&self, issue: &str, depends_on: &str) -> Result<CommandOutput> {
        self.run_command(&["dep", "add", issue, depends_on])
    }

    /// Remove a dependency
    pub fn dep_remove(&self, issue: &str, depends_on: &str) -> Result<CommandOutput> {
        self.run_command(&["dep", "remove", issue, depends_on])
    }

    // --- Comments ---

    /// Get comments for an issue
    pub fn comments(&self, issue_id: &str) -> Result<Vec<Comment>> {
        let output = self.run_command(&["comments", issue_id, "--json"])?;
        serde_json::from_str(&output.stdout).map_err(Error::from)
    }

    /// Add a comment to an issue
    pub fn comment_add(&self, issue_id: &str, content: &str) -> Result<CommandOutput> {
        self.run_command(&["comments", "add", issue_id, content])
    }

    // --- Labels ---

    /// Add a label to an issue
    pub fn label_add(&self, issue_id: &str, label: &str) -> Result<CommandOutput> {
        self.run_command(&["label", "add", issue_id, label])
    }

    /// Remove a label from an issue
    pub fn label_remove(&self, issue_id: &str, label: &str) -> Result<CommandOutput> {
        self.run_command(&["label", "remove", issue_id, label])
    }

    /// List all labels in the project
    pub fn label_list(&self) -> Result<CommandOutput> {
        self.run_command(&["label", "list"])
    }

    // --- Epic management ---

    /// List all epics
    pub fn epic_list(&self) -> Result<Vec<Issue>> {
        self.list(None, Some("epic"))
    }

    /// List open epics
    pub fn epic_list_open(&self) -> Result<Vec<Issue>> {
        self.list(Some("open"), Some("epic"))
    }

    /// Show epic details with children
    pub fn epic_show(&self, id: &str) -> Result<Issue> {
        self.show(id)
    }

    // --- Edit ---

    /// Edit an issue field in $EDITOR (returns immediately, editor opens interactively)
    /// Note: This spawns an interactive editor, use with caution in automated contexts
    pub fn edit(&self, id: &str, field: Option<&str>) -> Result<CommandOutput> {
        let mut args = vec!["edit", id];
        if let Some(f) = field {
            args.extend(["--field", f]);
        }
        self.run_command(&args)
    }

    // --- Stats and info ---

    /// Get project statistics
    pub fn stats(&self) -> Result<Stats> {
        let output = self.run_command(&["stats", "--json"])?;
        serde_json::from_str(&output.stdout).map_err(Error::from)
    }

    /// Get raw stats output
    pub fn stats_raw(&self) -> Result<CommandOutput> {
        self.run_command(&["stats"])
    }

    /// Get combined status info
    pub fn status_info(&self) -> Result<StatusInfo> {
        let stats = self.stats()?;
        let ready_count = self.ready().map(|r| r.len()).unwrap_or(0);

        Ok(StatusInfo {
            open: stats.open,
            in_progress: stats.in_progress,
            blocked: stats.blocked,
            ready: ready_count,
        })
    }

    // --- Activity ---

    /// Get global activity log
    pub fn activity(&self, limit: Option<usize>) -> Result<Vec<Activity>> {
        let limit_str;
        let mut args = vec!["activity", "--json"];

        if let Some(l) = limit {
            limit_str = l.to_string();
            args.extend(["--limit", &limit_str]);
        }

        let output = self.run_command(&args)?;
        serde_json::from_str(&output.stdout).map_err(Error::from)
    }

    /// Get activity for a specific issue
    pub fn activity_for_issue(
        &self,
        issue_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<Activity>> {
        let limit_str;
        let mut args = vec!["activity", "--mol", issue_id, "--json"];

        if let Some(l) = limit {
            limit_str = l.to_string();
            args.extend(["--limit", &limit_str]);
        }

        let output = self.run_command(&args)?;
        serde_json::from_str(&output.stdout).map_err(Error::from)
    }

    // --- Sync and admin ---

    /// Sync with remote
    pub fn sync(&self) -> Result<CommandOutput> {
        self.run_command(&["sync"])
    }

    /// Check sync status
    pub fn sync_status(&self) -> Result<CommandOutput> {
        self.run_command(&["sync", "--status"])
    }

    /// Initialize beads in current directory
    pub fn init(&self) -> Result<CommandOutput> {
        self.run_command(&["init"])
    }

    /// Run doctor checks
    pub fn doctor(&self) -> Result<CommandOutput> {
        self.run_command(&["doctor"])
    }

    /// Get human-readable help
    pub fn human(&self) -> Result<CommandOutput> {
        self.run_command(&["human"])
    }

    // --- Raw command execution ---

    /// Run an arbitrary bd command
    pub fn run(&self, args: &[&str]) -> Result<CommandOutput> {
        self.run_command(args)
    }

    // --- Private helpers ---

    fn run_command(&self, args: &[&str]) -> Result<CommandOutput> {
        let mut cmd = Command::new("bd");

        // Add global flags first (they apply to all commands)
        for flag in &self.global_flags {
            cmd.arg(flag);
        }

        // Then add command-specific args
        cmd.args(args);

        if let Some(ref dir) = self.workdir {
            cmd.current_dir(dir);
        }

        let output = cmd.output()?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() && !stderr.is_empty() {
            // Check for specific error conditions
            if stderr.contains("not initialized") || stderr.contains("No .beads") {
                return Err(Error::NotInRepo);
            }
            if stderr.contains("not found") || stderr.contains("Issue not found") {
                if let Some(id) = args.get(1) {
                    return Err(Error::IssueNotFound(id.to_string()));
                }
            }
            return Err(Error::CommandFailed(stderr));
        }

        Ok(CommandOutput {
            success: output.status.success(),
            stdout,
            stderr,
        })
    }

    /// Extract issue ID from command output
    fn extract_issue_id(&self, output: &str) -> Option<String> {
        // Look for patterns like "Created PROJ-1234" or "PROJ-1234:"
        for line in output.lines() {
            let words: Vec<&str> = line.split_whitespace().collect();
            for word in words {
                let word = word.trim_end_matches(':');
                if word.contains('-') && word.chars().any(|c| c.is_ascii_digit()) {
                    return Some(word.to_string());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beads_available() {
        // This test only passes if bd is installed
        if let Ok(bd) = Beads::new() {
            assert!(bd.is_available());
        }
    }

    #[test]
    fn test_with_workdir() {
        let bd = Beads::with_workdir("/tmp");
        assert_eq!(bd.workdir, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn test_command_output_combined() {
        let output = CommandOutput {
            success: true,
            stdout: "output".to_string(),
            stderr: "".to_string(),
        };
        assert_eq!(output.combined(), "output");

        let output_with_err = CommandOutput {
            success: false,
            stdout: "out".to_string(),
            stderr: "err".to_string(),
        };
        assert_eq!(output_with_err.combined(), "out\nerr");
    }

    #[test]
    fn test_status_display() {
        assert_eq!(Status::Open.to_string(), "open");
        assert_eq!(Status::InProgress.to_string(), "in_progress");
        assert_eq!(Status::Blocked.to_string(), "blocked");
        assert_eq!(Status::Deferred.to_string(), "deferred");
        assert_eq!(Status::Closed.to_string(), "closed");
    }

    #[test]
    fn test_issue_type_display() {
        assert_eq!(IssueType::Bug.to_string(), "bug");
        assert_eq!(IssueType::Feature.to_string(), "feature");
        assert_eq!(IssueType::Task.to_string(), "task");
        assert_eq!(IssueType::Epic.to_string(), "epic");
        assert_eq!(IssueType::Chore.to_string(), "chore");
        assert_eq!(IssueType::MergeRequest.to_string(), "merge_request");
        assert_eq!(IssueType::Molecule.to_string(), "molecule");
        assert_eq!(IssueType::Gate.to_string(), "gate");
    }

    #[test]
    fn test_stats_default() {
        let stats = Stats::default();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.open, 0);
        assert_eq!(stats.closed, 0);
    }

    #[test]
    fn test_status_info_default() {
        let info = StatusInfo::default();
        assert_eq!(info.open, 0);
        assert_eq!(info.in_progress, 0);
        assert_eq!(info.blocked, 0);
        assert_eq!(info.ready, 0);
    }

    #[test]
    fn test_issue_deserialize() {
        let json = r#"{
            "id": "PROJ-123",
            "title": "Test Issue",
            "status": "open",
            "type": "bug",
            "priority": 2
        }"#;
        let issue: Issue = serde_json::from_str(json).unwrap();
        assert_eq!(issue.id, "PROJ-123");
        assert_eq!(issue.title, "Test Issue");
        assert_eq!(issue.status, "open");
        assert_eq!(issue.issue_type, "bug");
        assert_eq!(issue.priority, Some(2));
    }

    #[test]
    fn test_comment_deserialize() {
        let json = r#"{
            "author": "user@example.com",
            "content": "This is a comment",
            "created_at": "2024-01-01"
        }"#;
        let comment: Comment = serde_json::from_str(json).unwrap();
        assert_eq!(comment.author, "user@example.com");
        assert_eq!(comment.content, "This is a comment");
    }

    #[test]
    fn test_activity_deserialize() {
        let json = r#"{
            "timestamp": "2024-01-01T12:00:00Z",
            "action": "created",
            "issue_id": "PROJ-123"
        }"#;
        let activity: Activity = serde_json::from_str(json).unwrap();
        assert_eq!(activity.action, "created");
        assert_eq!(activity.issue_id, Some("PROJ-123".to_string()));
    }

    #[test]
    fn test_extract_issue_id() {
        let bd = Beads::default();

        assert_eq!(
            bd.extract_issue_id("Created PROJ-123"),
            Some("PROJ-123".to_string())
        );
        assert_eq!(
            bd.extract_issue_id("Issue BEADS-456: Something"),
            Some("BEADS-456".to_string())
        );
        assert_eq!(bd.extract_issue_id("No issue id here"), None);
    }

    // Integration tests (require bd to be installed and in a repo)
    #[test]
    #[ignore]
    fn test_stats_in_repo() {
        if let Ok(bd) = Beads::new() {
            let result = bd.stats();
            assert!(result.is_ok());
        }
    }

    #[test]
    #[ignore]
    fn test_list_in_repo() {
        if let Ok(bd) = Beads::new() {
            let result = bd.list(None, None);
            assert!(result.is_ok());
        }
    }
}
