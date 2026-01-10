//! Agent representation and lifecycle states
//!
//! Defines the Agent struct that represents a running AI agent (Polecat)
//! and its associated state, metrics, and lifecycle.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Agent status indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Agent is starting up
    Starting,

    /// Agent is actively working
    Running,

    /// Agent is waiting (for CI, API, etc.)
    Waiting,

    /// Agent is paused by user
    Paused,

    /// Agent encountered an error
    Error,

    /// Agent completed successfully
    Completed,

    /// Agent was killed by user
    Killed,
}

impl AgentStatus {
    /// Get emoji indicator for status
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Starting => "🔵",
            Self::Running => "🟢",
            Self::Waiting => "🟡",
            Self::Paused => "⏸️",
            Self::Error => "🔴",
            Self::Completed => "✅",
            Self::Killed => "💀",
        }
    }

    /// Check if agent is actively running
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Starting | Self::Running | Self::Waiting | Self::Paused)
    }

    /// Check if agent has finished (successfully or not)
    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Completed | Self::Killed | Self::Error)
    }
}

/// Agent type/persona
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentPersona {
    /// General-purpose agent
    General,

    /// Security specialist
    SecuritySpecialist,

    /// Frontend expert
    FrontendExpert,

    /// Backend developer
    BackendDeveloper,

    /// DevOps/infrastructure specialist
    DevOps,

    /// Documentation writer
    TechWriter,

    /// Code refactoring specialist
    RefactorBot,

    /// Test writer
    TestWriter,

    /// Custom persona with name
    Custom(String),
}

impl Default for AgentPersona {
    fn default() -> Self {
        Self::General
    }
}

impl std::fmt::Display for AgentPersona {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::General => write!(f, "general"),
            Self::SecuritySpecialist => write!(f, "security-specialist"),
            Self::FrontendExpert => write!(f, "frontend-expert"),
            Self::BackendDeveloper => write!(f, "backend-developer"),
            Self::DevOps => write!(f, "devops"),
            Self::TechWriter => write!(f, "tech-writer"),
            Self::RefactorBot => write!(f, "refactor-bot"),
            Self::TestWriter => write!(f, "test-writer"),
            Self::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Cost tracking for an agent
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentCost {
    /// Total API cost in USD
    pub total_usd: f64,

    /// Input tokens used
    pub input_tokens: u64,

    /// Output tokens used
    pub output_tokens: u64,

    /// Number of API calls made
    pub api_calls: u32,
}

impl AgentCost {
    /// Create a new cost tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Add cost from an API call
    pub fn add_call(&mut self, input_tokens: u64, output_tokens: u64, cost_usd: f64) {
        self.input_tokens += input_tokens;
        self.output_tokens += output_tokens;
        self.total_usd += cost_usd;
        self.api_calls += 1;
    }

    /// Format cost as string
    pub fn format(&self) -> String {
        format!("${:.2}", self.total_usd)
    }
}

/// A running AI agent (Polecat)
#[derive(Debug, Clone)]
pub struct Agent {
    /// Unique identifier
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Process ID (if running as subprocess)
    pub pid: Option<u32>,

    /// Agent persona/type
    pub persona: AgentPersona,

    /// Current status
    pub status: AgentStatus,

    /// Status message (what the agent is currently doing)
    pub status_message: String,

    /// Context this agent belongs to (work, personal, etc.)
    pub context: String,

    /// Repository/rig the agent is working on
    pub rig: Option<String>,

    /// Bead ID the agent is working on
    pub bead_id: Option<String>,

    /// Files currently locked by this agent
    pub locked_files: HashSet<PathBuf>,

    /// Cost tracking
    pub cost: AgentCost,

    /// When the agent started
    #[allow(dead_code)]
    start_time: Instant,

    /// When the agent finished (if finished)
    end_time: Option<Instant>,
}

impl Agent {
    /// Create a new agent
    pub fn new(id: impl Into<String>, name: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            pid: None,
            persona: AgentPersona::default(),
            status: AgentStatus::Starting,
            status_message: "Initializing...".to_string(),
            context: context.into(),
            rig: None,
            bead_id: None,
            locked_files: HashSet::new(),
            cost: AgentCost::new(),
            start_time: Instant::now(),
            end_time: None,
        }
    }

    /// Set the process ID
    pub fn with_pid(mut self, pid: u32) -> Self {
        self.pid = Some(pid);
        self
    }

    /// Set the persona
    pub fn with_persona(mut self, persona: AgentPersona) -> Self {
        self.persona = persona;
        self
    }

    /// Set the rig
    pub fn with_rig(mut self, rig: impl Into<String>) -> Self {
        self.rig = Some(rig.into());
        self
    }

    /// Set the bead ID
    pub fn with_bead(mut self, bead_id: impl Into<String>) -> Self {
        self.bead_id = Some(bead_id.into());
        self
    }

    /// Update the status
    pub fn set_status(&mut self, status: AgentStatus, message: impl Into<String>) {
        self.status = status;
        self.status_message = message.into();

        if status.is_finished() {
            self.end_time = Some(Instant::now());
        }
    }

    /// Lock a file
    pub fn lock_file(&mut self, path: impl Into<PathBuf>) {
        self.locked_files.insert(path.into());
    }

    /// Unlock a file
    pub fn unlock_file(&mut self, path: &PathBuf) {
        self.locked_files.remove(path);
    }

    /// Unlock all files
    pub fn unlock_all_files(&mut self) {
        self.locked_files.clear();
    }

    /// Add API call cost
    pub fn add_cost(&mut self, input_tokens: u64, output_tokens: u64, cost_usd: f64) {
        self.cost.add_call(input_tokens, output_tokens, cost_usd);
    }

    /// Get runtime duration
    pub fn runtime(&self) -> Duration {
        if let Some(end) = self.end_time {
            end.duration_since(self.start_time)
        } else {
            self.start_time.elapsed()
        }
    }

    /// Format runtime as human-readable string
    pub fn format_runtime(&self) -> String {
        let duration = self.runtime();
        let secs = duration.as_secs();

        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {:02}m", secs / 3600, (secs % 3600) / 60)
        }
    }

    /// Get display line for TUI
    pub fn display_line(&self) -> String {
        let pid_str = self.pid.map(|p| format!("PID {}", p)).unwrap_or_default();
        let rig_str = self.rig.as_deref().unwrap_or("unknown");

        format!(
            "{} {} ({}) - {}",
            self.status.emoji(),
            self.name,
            pid_str,
            rig_str
        )
    }

    /// Get status detail line for TUI
    pub fn status_line(&self) -> String {
        let locks = if self.locked_files.is_empty() {
            "No locks".to_string()
        } else {
            let files: Vec<_> = self
                .locked_files
                .iter()
                .filter_map(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .collect();
            format!("Locked: [{}]", files.join(", "))
        };

        format!(
            "Runtime: {} | Cost: {} | {}",
            self.format_runtime(),
            self.cost.format(),
            locks
        )
    }
}

/// Agent spawn request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnRequest {
    /// Agent name
    pub name: String,

    /// Persona to use
    pub persona: AgentPersona,

    /// Context to work in
    pub context: String,

    /// Rig to work on (optional)
    pub rig: Option<String>,

    /// Bead ID to work on (optional)
    pub bead_id: Option<String>,

    /// Initial task description
    pub task: String,

    /// Budget limit in USD (optional)
    pub budget_limit: Option<f64>,

    /// Timeout in seconds (optional)
    pub timeout_secs: Option<u64>,
}

impl SpawnRequest {
    /// Create a new spawn request
    pub fn new(name: impl Into<String>, context: impl Into<String>, task: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            persona: AgentPersona::default(),
            context: context.into(),
            rig: None,
            bead_id: None,
            task: task.into(),
            budget_limit: None,
            timeout_secs: None,
        }
    }

    /// Set persona
    pub fn with_persona(mut self, persona: AgentPersona) -> Self {
        self.persona = persona;
        self
    }

    /// Set rig
    pub fn with_rig(mut self, rig: impl Into<String>) -> Self {
        self.rig = Some(rig.into());
        self
    }

    /// Set bead
    pub fn with_bead(mut self, bead_id: impl Into<String>) -> Self {
        self.bead_id = Some(bead_id.into());
        self
    }

    /// Set budget limit
    pub fn with_budget(mut self, budget_usd: f64) -> Self {
        self.budget_limit = Some(budget_usd);
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let agent = Agent::new("agent-1", "test_agent", "work");

        assert_eq!(agent.id, "agent-1");
        assert_eq!(agent.name, "test_agent");
        assert_eq!(agent.context, "work");
        assert_eq!(agent.status, AgentStatus::Starting);
        assert!(agent.locked_files.is_empty());
    }

    #[test]
    fn test_agent_builder() {
        let agent = Agent::new("agent-1", "refactor_bot", "work")
            .with_pid(12345)
            .with_persona(AgentPersona::RefactorBot)
            .with_rig("auth-service")
            .with_bead("ab-123");

        assert_eq!(agent.pid, Some(12345));
        assert_eq!(agent.persona, AgentPersona::RefactorBot);
        assert_eq!(agent.rig, Some("auth-service".to_string()));
        assert_eq!(agent.bead_id, Some("ab-123".to_string()));
    }

    #[test]
    fn test_agent_status() {
        let mut agent = Agent::new("agent-1", "test", "work");

        assert!(agent.status.is_active());
        assert!(!agent.status.is_finished());

        agent.set_status(AgentStatus::Running, "Working on task");
        assert!(agent.status.is_active());

        agent.set_status(AgentStatus::Completed, "Task complete");
        assert!(!agent.status.is_active());
        assert!(agent.status.is_finished());
    }

    #[test]
    fn test_agent_cost() {
        let mut cost = AgentCost::new();
        assert_eq!(cost.total_usd, 0.0);

        cost.add_call(1000, 500, 0.05);
        assert_eq!(cost.input_tokens, 1000);
        assert_eq!(cost.output_tokens, 500);
        assert_eq!(cost.api_calls, 1);
        assert_eq!(cost.format(), "$0.05");

        cost.add_call(2000, 1000, 0.10);
        assert_eq!(cost.input_tokens, 3000);
        assert!((cost.total_usd - 0.15).abs() < 0.001);
    }

    #[test]
    fn test_file_locking() {
        let mut agent = Agent::new("agent-1", "test", "work");

        agent.lock_file(PathBuf::from("src/main.rs"));
        agent.lock_file(PathBuf::from("src/lib.rs"));

        assert_eq!(agent.locked_files.len(), 2);
        assert!(agent.locked_files.contains(&PathBuf::from("src/main.rs")));

        agent.unlock_file(&PathBuf::from("src/main.rs"));
        assert_eq!(agent.locked_files.len(), 1);

        agent.unlock_all_files();
        assert!(agent.locked_files.is_empty());
    }

    #[test]
    fn test_spawn_request() {
        let request = SpawnRequest::new("my_agent", "work", "Refactor the auth module")
            .with_persona(AgentPersona::RefactorBot)
            .with_rig("auth-service")
            .with_budget(5.0)
            .with_timeout(3600);

        assert_eq!(request.name, "my_agent");
        assert_eq!(request.context, "work");
        assert_eq!(request.budget_limit, Some(5.0));
        assert_eq!(request.timeout_secs, Some(3600));
    }
}
