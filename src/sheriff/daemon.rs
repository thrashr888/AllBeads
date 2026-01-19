//! Sheriff daemon implementation
//!
//! Background synchronization daemon that federates beads across repositories.
//! Runs as a tokio async event loop with configurable poll intervals.

use super::metrics;
use crate::governance::checker::CheckSummary;
use crate::governance::config::load_policies_for_context;
use crate::governance::rules::CheckResult;
use crate::governance::{Policy, PolicyChecker, PolicyStorage};
use crate::graph::{RigId, ShadowBead};
use crate::mail::Postmaster;
use crate::manifest::Manifest;
use crate::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, Mutex};

use super::{sync_rig_to_shadows, SyncResult};

/// Default poll interval (5 seconds)
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Sheriff daemon configuration
#[derive(Debug, Clone)]
pub struct SheriffConfig {
    /// Path to the Boss repository
    pub boss_repo_path: PathBuf,

    /// Path to manifest file
    pub manifest_path: Option<PathBuf>,

    /// Poll interval for checking Rig repositories
    pub poll_interval: Duration,

    /// Enable verbose logging
    pub verbose: bool,

    /// Path to SQLite database for mail
    pub db_path: PathBuf,

    /// Project ID for mail routing
    pub project_id: String,

    /// Enable mail polling (check inbox and process messages)
    pub mail_poll: bool,

    /// Mail poll interval
    pub mail_poll_interval: Duration,

    /// Event broadcast channel capacity (default 1000)
    pub event_channel_capacity: usize,
}

/// Default mail poll interval (60 seconds)
pub const DEFAULT_MAIL_POLL_INTERVAL: Duration = Duration::from_secs(60);

/// Default event channel capacity (1000 events)
pub const DEFAULT_EVENT_CHANNEL_CAPACITY: usize = 1000;

impl Default for SheriffConfig {
    fn default() -> Self {
        Self {
            boss_repo_path: PathBuf::from("."),
            manifest_path: None,
            poll_interval: DEFAULT_POLL_INTERVAL,
            verbose: false,
            db_path: PathBuf::from(".beads/mail.db"),
            project_id: "boss".to_string(),
            mail_poll: false,
            mail_poll_interval: DEFAULT_MAIL_POLL_INTERVAL,
            event_channel_capacity: DEFAULT_EVENT_CHANNEL_CAPACITY,
        }
    }
}

impl SheriffConfig {
    /// Create a new config with the boss repo path
    pub fn new(boss_repo_path: impl Into<PathBuf>) -> Self {
        Self {
            boss_repo_path: boss_repo_path.into(),
            ..Default::default()
        }
    }

    /// Set the manifest path
    pub fn with_manifest(mut self, path: impl Into<PathBuf>) -> Self {
        self.manifest_path = Some(path.into());
        self
    }

    /// Set the poll interval
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Set verbose logging
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Set the database path
    pub fn with_db_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.db_path = path.into();
        self
    }

    /// Set the project ID
    pub fn with_project_id(mut self, id: impl Into<String>) -> Self {
        self.project_id = id.into();
        self
    }

    /// Enable mail polling
    pub fn with_mail_poll(mut self, enabled: bool) -> Self {
        self.mail_poll = enabled;
        self
    }

    /// Set the mail poll interval
    pub fn with_mail_poll_interval(mut self, interval: Duration) -> Self {
        self.mail_poll_interval = interval;
        self
    }
}

/// Events emitted by the Sheriff daemon
#[derive(Debug, Clone)]
pub enum SheriffEvent {
    /// Daemon started
    Started,

    /// Daemon stopped
    Stopped,

    /// Poll cycle started
    PollStarted,

    /// Poll cycle completed
    PollCompleted {
        /// Number of rigs polled
        rigs_polled: usize,
        /// Total sync changes
        changes: usize,
    },

    /// Sync completed for a rig
    RigSynced {
        /// Rig identifier
        rig_id: RigId,
        /// Sync result
        result: SyncResult,
    },

    /// Error occurred
    Error {
        /// Error message
        message: String,
    },

    /// New shadow bead created
    ShadowCreated(ShadowBead),

    /// Shadow bead updated
    ShadowUpdated(ShadowBead),

    /// Shadow bead deleted
    ShadowDeleted(RigId, String),

    /// Policy check completed
    PolicyChecked {
        /// Summary of check results
        summary: CheckSummary,
        /// Individual check results
        results: Vec<CheckResult>,
    },

    /// Mail poll started
    MailPollStarted,

    /// Mail poll completed
    MailPollCompleted {
        /// Number of messages processed
        messages_processed: usize,
    },
}

/// Commands that can be sent to the Sheriff daemon
#[derive(Debug, Clone)]
pub enum SheriffCommand {
    /// Trigger an immediate sync
    SyncNow,

    /// Stop the daemon
    Shutdown,

    /// Reload the manifest
    ReloadManifest,

    /// Set poll interval
    SetPollInterval(Duration),

    /// Reload policies
    ReloadPolicies,

    /// Run policy checks immediately
    CheckPolicies,
}

/// Result of handling a command
enum CommandResult {
    /// Continue running the daemon
    Continue,
    /// Stop the daemon
    Stop,
}

/// Rig state tracked by the daemon
struct RigState {
    /// Rig identifier
    id: RigId,

    /// Path to the rig repository
    path: PathBuf,

    /// Context name for this rig
    context: String,

    /// Existing shadow beads for this rig
    shadows: Vec<ShadowBead>,

    /// Last sync result
    last_sync: Option<SyncResult>,
}

/// Sheriff daemon
///
/// The synchronization engine that federates beads across repositories.
pub struct Sheriff {
    /// Configuration
    config: SheriffConfig,

    /// Manifest (if loaded)
    manifest: Option<Manifest>,

    /// Rig states indexed by rig ID
    rigs: HashMap<String, RigState>,

    /// All shadow beads
    shadows: Vec<ShadowBead>,

    /// Postmaster for mail delivery
    postmaster: Option<Arc<Mutex<Postmaster>>>,

    /// Policy checker for governance
    policy_checker: PolicyChecker,

    /// Policy storage (for persistence)
    policy_storage: Option<PolicyStorage>,

    /// Event sender
    event_tx: broadcast::Sender<SheriffEvent>,

    /// Command receiver
    command_rx: Option<mpsc::Receiver<SheriffCommand>>,

    /// Command sender (for cloning)
    command_tx: mpsc::Sender<SheriffCommand>,

    /// Running flag
    running: bool,
}

impl Sheriff {
    /// Create a new Sheriff daemon
    pub fn new(config: SheriffConfig) -> Result<Self> {
        let (event_tx, _) = broadcast::channel(config.event_channel_capacity);
        let (command_tx, command_rx) = mpsc::channel(10);

        // Load policies from config file, or use defaults
        let policies = load_policies_for_context(&config.boss_repo_path);
        let policy_checker = if policies.is_empty() {
            PolicyChecker::with_defaults()
        } else {
            let mut checker = PolicyChecker::new();
            for policy in policies {
                checker.add_policy(policy);
            }
            checker
        };

        Ok(Self {
            config,
            manifest: None,
            rigs: HashMap::new(),
            shadows: Vec::new(),
            postmaster: None,
            policy_checker,
            policy_storage: None,
            event_tx,
            command_rx: Some(command_rx),
            command_tx,
            running: false,
        })
    }

    /// Set custom policies for the checker
    pub fn set_policies(&mut self, policies: Vec<Policy>) {
        self.policy_checker.set_policies(policies);
    }

    /// Get current policies
    pub fn policies(&self) -> &[Policy] {
        self.policy_checker.policies()
    }

    /// Get an event subscriber
    pub fn subscribe(&self) -> broadcast::Receiver<SheriffEvent> {
        self.event_tx.subscribe()
    }

    /// Get a command sender
    pub fn command_sender(&self) -> mpsc::Sender<SheriffCommand> {
        self.command_tx.clone()
    }

    /// Send an event, logging if dropped due to no receivers or channel full
    fn send_event(&self, event: SheriffEvent) {
        match self.event_tx.send(event) {
            Ok(receiver_count) => {
                // Warn if getting close to capacity (80% threshold)
                let capacity = self.config.event_channel_capacity;
                let len = self.event_tx.len();
                if len > capacity * 80 / 100 {
                    tracing::warn!(
                        current = len,
                        capacity = capacity,
                        threshold_pct = 80,
                        "Event channel nearing capacity"
                    );
                }
                // Debug log when no receivers
                if receiver_count == 0 {
                    tracing::debug!("Event sent but no receivers subscribed");
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Event dropped - channel may be full or no receivers"
                );
            }
        }
    }

    /// Initialize the daemon
    pub fn init(&mut self) -> Result<()> {
        // Load manifest if configured
        if let Some(ref manifest_path) = self.config.manifest_path {
            self.manifest = Some(Manifest::from_file(manifest_path)?);
            self.init_rigs_from_manifest()?;
        }

        // Initialize postmaster
        let db_path = self.config.boss_repo_path.join(&self.config.db_path);
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let postmaster = Postmaster::with_project_id(db_path, &self.config.project_id)?;
        self.postmaster = Some(Arc::new(Mutex::new(postmaster)));

        Ok(())
    }

    /// Initialize rigs from manifest
    fn init_rigs_from_manifest(&mut self) -> Result<()> {
        let manifest = self
            .manifest
            .as_ref()
            .ok_or_else(|| crate::AllBeadsError::Config("No manifest loaded".to_string()))?;

        for project in &manifest.projects {
            let rig_id = project.prefix().unwrap_or(&project.path).to_string();

            let path = self.config.boss_repo_path.join(&project.path);

            let state = RigState {
                id: RigId::new(&rig_id),
                path,
                context: self.config.project_id.clone(),
                shadows: Vec::new(),
                last_sync: None,
            };

            self.rigs.insert(rig_id, state);
        }

        Ok(())
    }

    /// Add a rig manually (without manifest)
    pub fn add_rig(
        &mut self,
        id: impl Into<String>,
        path: impl Into<PathBuf>,
        context: impl Into<String>,
    ) {
        let id = id.into();
        let state = RigState {
            id: RigId::new(&id),
            path: path.into(),
            context: context.into(),
            shadows: Vec::new(),
            last_sync: None,
        };
        self.rigs.insert(id, state);
    }

    /// Run the daemon event loop with graceful shutdown on SIGTERM/SIGINT
    pub async fn run(&mut self) -> Result<()> {
        self.running = true;
        metrics::set_health_status(true);
        self.send_event(SheriffEvent::Started);

        let mut interval = tokio::time::interval(self.config.poll_interval);
        let mut mail_interval = tokio::time::interval(self.config.mail_poll_interval);
        let mail_poll_enabled = self.config.mail_poll;

        let mut command_rx = self
            .command_rx
            .take()
            .ok_or_else(|| crate::AllBeadsError::Config("Daemon already running".to_string()))?;

        // Use platform-specific event loop
        #[cfg(unix)]
        {
            self.run_with_signals(
                &mut interval,
                &mut mail_interval,
                mail_poll_enabled,
                &mut command_rx,
            )
            .await?;
        }

        #[cfg(not(unix))]
        {
            self.run_without_signals(
                &mut interval,
                &mut mail_interval,
                mail_poll_enabled,
                &mut command_rx,
            )
            .await?;
        }

        // Cleanup before exit
        tracing::info!("Performing shutdown cleanup");
        self.cleanup().await;

        metrics::set_health_status(false);
        self.send_event(SheriffEvent::Stopped);
        Ok(())
    }

    /// Run event loop with Unix signal handling (SIGTERM/SIGINT)
    #[cfg(unix)]
    async fn run_with_signals(
        &mut self,
        interval: &mut tokio::time::Interval,
        mail_interval: &mut tokio::time::Interval,
        mail_poll_enabled: bool,
        command_rx: &mut mpsc::Receiver<SheriffCommand>,
    ) -> Result<()> {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm = signal(SignalKind::terminate()).map_err(|e| {
            crate::AllBeadsError::Other(format!("Failed to set up SIGTERM handler: {}", e))
        })?;
        let mut sigint = signal(SignalKind::interrupt()).map_err(|e| {
            crate::AllBeadsError::Other(format!("Failed to set up SIGINT handler: {}", e))
        })?;

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if self.running {
                        self.poll_cycle().await;
                    }
                }
                _ = mail_interval.tick(), if mail_poll_enabled => {
                    if self.running {
                        self.poll_mail().await;
                    }
                }
                Some(cmd) = command_rx.recv() => {
                    match self.handle_command_async(cmd, interval).await {
                        CommandResult::Continue => {}
                        CommandResult::Stop => break,
                    }
                }
                _ = sigterm.recv() => {
                    tracing::info!("Received SIGTERM, initiating graceful shutdown");
                    self.running = false;
                    break;
                }
                _ = sigint.recv() => {
                    tracing::info!("Received SIGINT, initiating graceful shutdown");
                    self.running = false;
                    break;
                }
            }
        }
        Ok(())
    }

    /// Run event loop without signal handling (non-Unix platforms)
    #[cfg(not(unix))]
    async fn run_without_signals(
        &mut self,
        interval: &mut tokio::time::Interval,
        mail_interval: &mut tokio::time::Interval,
        mail_poll_enabled: bool,
        command_rx: &mut mpsc::Receiver<SheriffCommand>,
    ) -> Result<()> {
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if self.running {
                        self.poll_cycle().await;
                    }
                }
                _ = mail_interval.tick(), if mail_poll_enabled => {
                    if self.running {
                        self.poll_mail().await;
                    }
                }
                Some(cmd) = command_rx.recv() => {
                    match self.handle_command_async(cmd, interval).await {
                        CommandResult::Continue => {}
                        CommandResult::Stop => break,
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle a command asynchronously
    async fn handle_command_async(
        &mut self,
        cmd: SheriffCommand,
        interval: &mut tokio::time::Interval,
    ) -> CommandResult {
        match cmd {
            SheriffCommand::SyncNow => {
                self.poll_cycle().await;
            }
            SheriffCommand::Shutdown => {
                tracing::info!("Received shutdown command");
                self.running = false;
                return CommandResult::Stop;
            }
            SheriffCommand::ReloadManifest => {
                if let Err(e) = self.reload_manifest() {
                    self.send_event(SheriffEvent::Error {
                        message: format!("Failed to reload manifest: {}", e),
                    });
                }
            }
            SheriffCommand::SetPollInterval(duration) => {
                *interval = tokio::time::interval(duration);
                self.config.poll_interval = duration;
            }
            SheriffCommand::ReloadPolicies => {
                self.reload_policies();
            }
            SheriffCommand::CheckPolicies => {
                self.run_policy_checks();
            }
        }
        CommandResult::Continue
    }

    /// Perform cleanup before shutdown
    async fn cleanup(&mut self) {
        // Flush any pending mail operations
        if let Some(ref postmaster) = self.postmaster {
            if let Ok(mut pm) = postmaster.try_lock() {
                pm.cleanup_expired_locks();
                tracing::debug!("Cleaned up expired locks");
            }
        }

        // Log final stats
        let total_shadows = self.shadows.len();
        let total_rigs = self.rigs.len();
        tracing::info!(
            shadows = total_shadows,
            rigs = total_rigs,
            "Sheriff daemon shutdown complete"
        );
    }

    /// Execute a single poll cycle
    async fn poll_cycle(&mut self) {
        self.send_event(SheriffEvent::PollStarted);

        let mut total_changes = 0;
        let rigs_count = self.rigs.len();

        // Collect rig IDs to avoid borrow issues
        let rig_ids: Vec<String> = self.rigs.keys().cloned().collect();

        for rig_id in rig_ids {
            let start = std::time::Instant::now();
            match self.sync_rig(&rig_id) {
                Ok(result) => {
                    // Record sync duration metric
                    let duration = start.elapsed().as_secs_f64();
                    metrics::record_sync_duration(&rig_id, duration);

                    total_changes += result.change_count();

                    // Get rig_id_typed from the state
                    if let Some(state) = self.rigs.get(&rig_id) {
                        // Record shadow count metric
                        metrics::set_shadows_count(&rig_id, state.shadows.len() as i64);

                        self.send_event(SheriffEvent::RigSynced {
                            rig_id: state.id.clone(),
                            result,
                        });
                    }
                }
                Err(e) => {
                    // Record error metric
                    metrics::record_api_error("sync_error", &rig_id);

                    self.send_event(SheriffEvent::Error {
                        message: format!("Failed to sync rig {}: {}", rig_id, e),
                    });
                }
            }
        }

        // Record sync cycle completion
        metrics::record_sync_cycle("success");

        self.send_event(SheriffEvent::PollCompleted {
            rigs_polled: rigs_count,
            changes: total_changes,
        });

        // Cleanup expired locks in postmaster
        if let Some(ref postmaster) = self.postmaster {
            if let Ok(mut pm) = postmaster.try_lock() {
                pm.cleanup_expired_locks();
            }
        }

        // Run policy checks
        self.run_policy_checks();
    }

    /// Poll mail inbox and process messages
    async fn poll_mail(&mut self) {
        use crate::mail::Address;

        self.send_event(SheriffEvent::MailPollStarted);

        let postmaster = match &self.postmaster {
            Some(pm) => pm.clone(),
            None => {
                self.send_event(SheriffEvent::Error {
                    message: "Postmaster not initialized for mail polling".to_string(),
                });
                return;
            }
        };

        // Create Sheriff address for this project
        let sheriff_address = match Address::new("sheriff", &self.config.project_id) {
            Ok(addr) => addr,
            Err(e) => {
                self.send_event(SheriffEvent::Error {
                    message: format!("Invalid sheriff address: {}", e),
                });
                return;
            }
        };

        // Get unread messages
        let messages = {
            let pm = postmaster.lock().await;
            match pm.unread(&sheriff_address) {
                Ok(msgs) => msgs,
                Err(e) => {
                    self.send_event(SheriffEvent::Error {
                        message: format!("Failed to fetch mail: {}", e),
                    });
                    return;
                }
            }
        };

        let mut processed_count = 0;

        for message in messages {
            // Process the message based on type
            // For now, we just acknowledge it by marking as read
            // Future: Handle specific message types (Notify, Request, etc.)

            if self.config.verbose {
                tracing::info!(
                    "Processing mail from {}: {:?}",
                    message.message.from,
                    message.message.message_type
                );
            }

            // Mark message as read
            {
                let pm = postmaster.lock().await;
                if let Err(e) = pm.mark_read(&message.message.id) {
                    self.send_event(SheriffEvent::Error {
                        message: format!("Failed to mark mail as read: {}", e),
                    });
                }
            }

            processed_count += 1;
        }

        self.send_event(SheriffEvent::MailPollCompleted {
            messages_processed: processed_count,
        });
    }

    /// Run policy checks against the current state
    fn run_policy_checks(&mut self) {
        // Create a temporary graph from shadows for policy checking
        let graph = self.create_graph_from_shadows();

        // Run policy checks
        let results = self.policy_checker.check_graph(&graph);
        let summary = PolicyChecker::summarize(&results);

        // Store results if storage is available
        if let Some(ref storage) = self.policy_storage {
            for result in &results {
                let _ = storage.save_result(result);
            }
        }

        // Emit policy check event
        let _ = self
            .event_tx
            .send(SheriffEvent::PolicyChecked { summary, results });
    }

    /// Create a FederatedGraph from shadow beads for policy checking
    fn create_graph_from_shadows(&self) -> crate::graph::FederatedGraph {
        use crate::graph::{Bead, FederatedGraph, IssueType, Priority};

        let mut graph = FederatedGraph::new();

        for shadow in &self.shadows {
            // Convert shadow bead to regular bead for policy checking
            let bead = Bead {
                id: shadow.id.clone(),
                title: shadow.summary.clone(),
                description: shadow.notes.clone(),
                status: shadow.status,
                priority: Priority::P2, // Default priority for shadows
                labels: shadow.labels.clone(),
                dependencies: shadow
                    .cross_repo_dependencies
                    .iter()
                    .filter_map(|uri| uri.bead_id())
                    .collect(),
                blocks: shadow
                    .cross_repo_blocks
                    .iter()
                    .filter_map(|uri| uri.bead_id())
                    .collect(),
                created_at: shadow.last_synced.clone(),
                updated_at: shadow.last_synced.clone(),
                created_by: "shadow".to_string(),
                assignee: None,
                issue_type: IssueType::Task,
                notes: None,
                aiki_tasks: Vec::new(),
                handoff: None,
            };

            graph.beads.insert(bead.id.clone(), bead);
        }

        graph
    }

    /// Reload policies from storage
    fn reload_policies(&mut self) {
        if let Some(ref storage) = self.policy_storage {
            if let Ok(policies) = storage.load_policies() {
                self.policy_checker.set_policies(policies);
            }
        }
    }

    /// Sync a single rig
    fn sync_rig(&mut self, rig_id: &str) -> Result<SyncResult> {
        // Get mutable access to take shadows and read path/context
        let state = self
            .rigs
            .get_mut(rig_id)
            .ok_or_else(|| crate::AllBeadsError::Config(format!("Rig not found: {}", rig_id)))?;

        // Check if rig path exists
        if !state.path.exists() {
            return Err(crate::AllBeadsError::Config(format!(
                "Rig path does not exist: {}",
                state.path.display()
            )));
        }

        // Take ownership of existing shadows instead of cloning (saves one full Vec clone)
        let existing_shadows = std::mem::take(&mut state.shadows);
        // Copy path and context for use after releasing borrow
        let rig_path = state.path.clone();
        let context = state.context.clone();

        // Sync rig to shadows (borrow of self.rigs released)
        let (result, new_shadows) =
            sync_rig_to_shadows(&rig_path, rig_id, &context, existing_shadows)?;

        // Update state - clone shadows for rig state, move into global list
        let shadows_for_rig = new_shadows.clone();
        if let Some(state) = self.rigs.get_mut(rig_id) {
            state.shadows = shadows_for_rig;
            // Clone result for storing (original returned to caller)
            state.last_sync = Some(result.clone());
        }

        // Update global shadows list (takes ownership of new_shadows)
        self.update_shadows(rig_id, new_shadows);

        Ok(result)
    }

    /// Update global shadows list after a rig sync
    fn update_shadows(&mut self, rig_id: &str, new_shadows: Vec<ShadowBead>) {
        // Pre-compute prefix once instead of allocating for each shadow (O(1) vs O(n) allocations)
        let prefix = format!("bead://{}/", rig_id);

        // Remove old shadows for this rig
        self.shadows
            .retain(|s| !s.pointer.as_str().contains(&prefix));

        // Add new shadows
        self.shadows.extend(new_shadows);
    }

    /// Reload manifest from file
    fn reload_manifest(&mut self) -> Result<()> {
        if let Some(ref manifest_path) = self.config.manifest_path {
            self.manifest = Some(Manifest::from_file(manifest_path)?);
            self.init_rigs_from_manifest()?;
        }
        Ok(())
    }

    /// Get all shadow beads
    pub fn shadows(&self) -> &[ShadowBead] {
        &self.shadows
    }

    /// Get the postmaster
    pub fn postmaster(&self) -> Option<Arc<Mutex<Postmaster>>> {
        self.postmaster.clone()
    }

    /// Check if daemon is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Get sync stats
    pub fn stats(&self) -> SheriffStats {
        let mut total_shadows = 0;
        let mut last_sync_changes = 0;

        for state in self.rigs.values() {
            total_shadows += state.shadows.len();
            if let Some(ref sync) = state.last_sync {
                last_sync_changes += sync.change_count();
            }
        }

        SheriffStats {
            rigs_count: self.rigs.len(),
            total_shadows,
            last_sync_changes,
            poll_interval: self.config.poll_interval,
        }
    }
}

/// Sheriff statistics
#[derive(Debug, Clone)]
pub struct SheriffStats {
    /// Number of rigs being tracked
    pub rigs_count: usize,

    /// Total shadow beads
    pub total_shadows: usize,

    /// Changes from last sync cycle
    pub last_sync_changes: usize,

    /// Current poll interval
    pub poll_interval: Duration,
}

/// Builder for Sheriff daemon
pub struct SheriffBuilder {
    config: SheriffConfig,
}

impl SheriffBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: SheriffConfig::default(),
        }
    }

    /// Set the boss repo path
    pub fn boss_repo(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.boss_repo_path = path.into();
        self
    }

    /// Set the manifest path
    pub fn manifest(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.manifest_path = Some(path.into());
        self
    }

    /// Set the poll interval
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.config.poll_interval = interval;
        self
    }

    /// Set verbose logging
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.config.verbose = verbose;
        self
    }

    /// Set the database path
    pub fn db_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.db_path = path.into();
        self
    }

    /// Set the project ID
    pub fn project_id(mut self, id: impl Into<String>) -> Self {
        self.config.project_id = id.into();
        self
    }

    /// Build the Sheriff
    pub fn build(self) -> Result<Sheriff> {
        Sheriff::new(self.config)
    }
}

impl Default for SheriffBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = SheriffConfig::new(".")
            .with_poll_interval(Duration::from_secs(10))
            .with_verbose(true)
            .with_project_id("test-project");

        assert_eq!(config.poll_interval, Duration::from_secs(10));
        assert!(config.verbose);
        assert_eq!(config.project_id, "test-project");
    }

    #[test]
    fn test_sheriff_builder() {
        let sheriff = SheriffBuilder::new()
            .boss_repo(".")
            .poll_interval(Duration::from_secs(10))
            .verbose(true)
            .project_id("test")
            .build()
            .unwrap();

        assert_eq!(sheriff.config.poll_interval, Duration::from_secs(10));
    }

    #[test]
    fn test_add_rig() {
        let mut sheriff = Sheriff::new(SheriffConfig::default()).unwrap();
        sheriff.add_rig("auth-service", "/path/to/auth", "work");

        assert!(sheriff.rigs.contains_key("auth-service"));
    }

    #[test]
    fn test_stats() {
        let sheriff = Sheriff::new(SheriffConfig::default()).unwrap();
        let stats = sheriff.stats();

        assert_eq!(stats.rigs_count, 0);
        assert_eq!(stats.total_shadows, 0);
    }
}
