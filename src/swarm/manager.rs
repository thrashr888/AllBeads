//! Agent Manager for lifecycle operations
//!
//! Handles spawning, monitoring, and killing agents across all contexts.

use super::agent::{Agent, AgentCost, AgentStatus, SpawnRequest};
use crate::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

/// Type alias for event listener collection to reduce complexity
type EventListeners = Arc<RwLock<Vec<Box<dyn Fn(ManagerEvent) + Send + Sync>>>>;

/// Budget tracking for a context
#[derive(Debug, Clone, Default)]
pub struct ContextBudget {
    /// Maximum budget in USD
    pub limit: Option<f64>,

    /// Current spend in USD
    pub spent: f64,

    /// Number of agents spawned
    pub agents_spawned: u32,

    /// Number of agents completed
    pub agents_completed: u32,
}

impl ContextBudget {
    /// Create a new budget with optional limit
    pub fn new(limit: Option<f64>) -> Self {
        Self {
            limit,
            spent: 0.0,
            agents_spawned: 0,
            agents_completed: 0,
        }
    }

    /// Check if budget allows more spending
    pub fn has_budget(&self, amount: f64) -> bool {
        match self.limit {
            Some(limit) => self.spent + amount <= limit,
            None => true,
        }
    }

    /// Add spending
    pub fn spend(&mut self, amount: f64) {
        self.spent += amount;
    }

    /// Get remaining budget
    pub fn remaining(&self) -> Option<f64> {
        self.limit.map(|l| (l - self.spent).max(0.0))
    }

    /// Get usage percentage
    pub fn usage_percent(&self) -> Option<f64> {
        self.limit.map(|l| (self.spent / l * 100.0).min(100.0))
    }
}

/// Events emitted by the agent manager
#[derive(Debug, Clone)]
pub enum ManagerEvent {
    /// Agent spawned
    AgentSpawned(String),

    /// Agent status changed
    AgentStatusChanged {
        agent_id: String,
        old_status: AgentStatus,
        new_status: AgentStatus,
    },

    /// Agent completed
    AgentCompleted { agent_id: String, cost: AgentCost },

    /// Agent killed
    AgentKilled(String),

    /// Budget warning (approaching limit)
    BudgetWarning {
        context: String,
        spent: f64,
        limit: f64,
    },

    /// Budget exceeded
    BudgetExceeded {
        context: String,
        spent: f64,
        limit: f64,
    },

    /// Error occurred
    Error(String),
}

/// Agent Manager
///
/// Central manager for all agent lifecycle operations.
pub struct AgentManager {
    /// Active agents by ID
    agents: Arc<RwLock<HashMap<String, Agent>>>,

    /// Context budgets
    budgets: Arc<RwLock<HashMap<String, ContextBudget>>>,

    /// Next agent ID counter
    next_id: Arc<RwLock<u64>>,

    /// Event listeners (for TUI updates, etc.)
    event_listeners: EventListeners,
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentManager {
    /// Create a new agent manager
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            budgets: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
            event_listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Set budget for a context
    pub fn set_budget(&self, context: impl Into<String>, limit: f64) {
        let context = context.into();
        let mut budgets = self.budgets.write().unwrap();
        budgets.insert(context, ContextBudget::new(Some(limit)));
    }

    /// Get budget for a context
    pub fn get_budget(&self, context: &str) -> Option<ContextBudget> {
        let budgets = self.budgets.read().unwrap();
        budgets.get(context).cloned()
    }

    /// Add an event listener
    pub fn add_listener<F>(&self, listener: F)
    where
        F: Fn(ManagerEvent) + Send + Sync + 'static,
    {
        let mut listeners = self.event_listeners.write().unwrap();
        listeners.push(Box::new(listener));
    }

    /// Emit an event to all listeners
    fn emit(&self, event: ManagerEvent) {
        let listeners = self.event_listeners.read().unwrap();
        for listener in listeners.iter() {
            listener(event.clone());
        }
    }

    /// Generate a unique agent ID
    fn generate_id(&self) -> String {
        let mut counter = self.next_id.write().unwrap();
        let id = format!("agent-{}", *counter);
        *counter += 1;
        id
    }

    /// Spawn a new agent
    ///
    /// Returns the agent ID if successful.
    pub fn spawn(&self, request: SpawnRequest) -> Result<String> {
        // Check budget
        if let Some(budget_limit) = request.budget_limit {
            let budgets = self.budgets.read().unwrap();
            if let Some(budget) = budgets.get(&request.context) {
                if !budget.has_budget(budget_limit) {
                    return Err(crate::AllBeadsError::Swarm(format!(
                        "Context '{}' budget exceeded",
                        request.context
                    )));
                }
            }
        }

        let id = self.generate_id();

        let mut agent =
            Agent::new(&id, &request.name, &request.context).with_persona(request.persona);

        if let Some(ref rig) = request.rig {
            agent = agent.with_rig(rig);
        }

        if let Some(ref bead_id) = request.bead_id {
            agent = agent.with_bead(bead_id);
        }

        agent.set_status(AgentStatus::Starting, &request.task);

        // Add to agents map
        {
            let mut agents = self.agents.write().unwrap();
            agents.insert(id.clone(), agent);
        }

        // Update budget
        {
            let mut budgets = self.budgets.write().unwrap();
            let budget = budgets
                .entry(request.context.clone())
                .or_insert_with(|| ContextBudget::new(None));
            budget.agents_spawned += 1;
        }

        info!("Spawned agent '{}' ({})", request.name, id);
        self.emit(ManagerEvent::AgentSpawned(id.clone()));

        Ok(id)
    }

    /// Get an agent by ID
    pub fn get(&self, agent_id: &str) -> Option<Agent> {
        let agents = self.agents.read().unwrap();
        agents.get(agent_id).cloned()
    }

    /// Get all agents
    pub fn list(&self) -> Vec<Agent> {
        let agents = self.agents.read().unwrap();
        agents.values().cloned().collect()
    }

    /// Get agents by context
    pub fn list_by_context(&self, context: &str) -> Vec<Agent> {
        let agents = self.agents.read().unwrap();
        agents
            .values()
            .filter(|a| a.context == context)
            .cloned()
            .collect()
    }

    /// Get active agents
    pub fn list_active(&self) -> Vec<Agent> {
        let agents = self.agents.read().unwrap();
        agents
            .values()
            .filter(|a| a.status.is_active())
            .cloned()
            .collect()
    }

    /// Update agent status
    pub fn update_status(
        &self,
        agent_id: &str,
        status: AgentStatus,
        message: impl Into<String>,
    ) -> Result<()> {
        let old_status;

        {
            let mut agents = self.agents.write().unwrap();
            let agent = agents.get_mut(agent_id).ok_or_else(|| {
                crate::AllBeadsError::Swarm(format!("Agent '{}' not found", agent_id))
            })?;

            old_status = agent.status;
            agent.set_status(status, message);

            // If completed, update budget stats
            if status == AgentStatus::Completed {
                let mut budgets = self.budgets.write().unwrap();
                if let Some(budget) = budgets.get_mut(&agent.context) {
                    budget.agents_completed += 1;
                }
            }
        }

        debug!(
            "Agent {} status: {:?} -> {:?}",
            agent_id, old_status, status
        );

        self.emit(ManagerEvent::AgentStatusChanged {
            agent_id: agent_id.to_string(),
            old_status,
            new_status: status,
        });

        Ok(())
    }

    /// Add cost to an agent
    pub fn add_cost(
        &self,
        agent_id: &str,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
    ) -> Result<()> {
        let context;
        let budget_warning;

        {
            let mut agents = self.agents.write().unwrap();
            let agent = agents.get_mut(agent_id).ok_or_else(|| {
                crate::AllBeadsError::Swarm(format!("Agent '{}' not found", agent_id))
            })?;

            agent.add_cost(input_tokens, output_tokens, cost_usd);
            context = agent.context.clone();
        }

        // Update context budget
        {
            let mut budgets = self.budgets.write().unwrap();
            if let Some(budget) = budgets.get_mut(&context) {
                budget.spend(cost_usd);

                // Check for budget warnings
                if let Some(limit) = budget.limit {
                    let usage = budget.spent / limit;
                    if usage >= 1.0 {
                        budget_warning = Some((budget.spent, limit, true));
                    } else if usage >= 0.9 {
                        budget_warning = Some((budget.spent, limit, false));
                    } else {
                        budget_warning = None;
                    }
                } else {
                    budget_warning = None;
                }
            } else {
                budget_warning = None;
            }
        }

        // Emit budget events
        if let Some((spent, limit, exceeded)) = budget_warning {
            if exceeded {
                warn!(
                    "Budget exceeded for context '{}': ${:.2} / ${:.2}",
                    context, spent, limit
                );
                self.emit(ManagerEvent::BudgetExceeded {
                    context,
                    spent,
                    limit,
                });
            } else {
                warn!(
                    "Budget warning for context '{}': ${:.2} / ${:.2}",
                    context, spent, limit
                );
                self.emit(ManagerEvent::BudgetWarning {
                    context,
                    spent,
                    limit,
                });
            }
        }

        Ok(())
    }

    /// Kill an agent
    pub fn kill(&self, agent_id: &str) -> Result<()> {
        let agent_cost;

        {
            let mut agents = self.agents.write().unwrap();
            let agent = agents.get_mut(agent_id).ok_or_else(|| {
                crate::AllBeadsError::Swarm(format!("Agent '{}' not found", agent_id))
            })?;

            if agent.status.is_finished() {
                return Err(crate::AllBeadsError::Swarm(format!(
                    "Agent '{}' already finished",
                    agent_id
                )));
            }

            agent.set_status(AgentStatus::Killed, "Killed by user");
            agent.unlock_all_files();
            agent_cost = agent.cost.clone();

            // If there's a PID, we would kill the process here
            if let Some(pid) = agent.pid {
                // In a real implementation, we would:
                // - Send SIGTERM to the process
                // - Wait for graceful shutdown
                // - Send SIGKILL if needed
                debug!("Would kill process {} for agent {}", pid, agent_id);
            }
        }

        info!("Killed agent '{}'", agent_id);

        self.emit(ManagerEvent::AgentKilled(agent_id.to_string()));
        self.emit(ManagerEvent::AgentCompleted {
            agent_id: agent_id.to_string(),
            cost: agent_cost,
        });

        Ok(())
    }

    /// Pause an agent
    pub fn pause(&self, agent_id: &str) -> Result<()> {
        self.update_status(agent_id, AgentStatus::Paused, "Paused by user")
    }

    /// Resume a paused agent
    pub fn resume(&self, agent_id: &str) -> Result<()> {
        let agents = self.agents.read().unwrap();
        let agent = agents.get(agent_id).ok_or_else(|| {
            crate::AllBeadsError::Swarm(format!("Agent '{}' not found", agent_id))
        })?;

        if agent.status != AgentStatus::Paused {
            return Err(crate::AllBeadsError::Swarm(format!(
                "Agent '{}' is not paused",
                agent_id
            )));
        }

        drop(agents);
        self.update_status(agent_id, AgentStatus::Running, "Resumed by user")
    }

    /// Pause all agents in a context
    pub fn pause_all(&self, context: &str) -> Result<u32> {
        let agent_ids: Vec<String> = {
            let agents = self.agents.read().unwrap();
            agents
                .values()
                .filter(|a| a.context == context && a.status.is_active())
                .map(|a| a.id.clone())
                .collect()
        };

        let mut count = 0;
        for id in agent_ids {
            if self.pause(&id).is_ok() {
                count += 1;
            }
        }

        info!("Paused {} agents in context '{}'", count, context);
        Ok(count)
    }

    /// Resume all paused agents in a context
    pub fn resume_all(&self, context: &str) -> Result<u32> {
        let agent_ids: Vec<String> = {
            let agents = self.agents.read().unwrap();
            agents
                .values()
                .filter(|a| a.context == context && a.status == AgentStatus::Paused)
                .map(|a| a.id.clone())
                .collect()
        };

        let mut count = 0;
        for id in agent_ids {
            if self.resume(&id).is_ok() {
                count += 1;
            }
        }

        info!("Resumed {} agents in context '{}'", count, context);
        Ok(count)
    }

    /// Kill all agents in a context
    pub fn kill_all(&self, context: &str) -> Result<u32> {
        let agent_ids: Vec<String> = {
            let agents = self.agents.read().unwrap();
            agents
                .values()
                .filter(|a| a.context == context && a.status.is_active())
                .map(|a| a.id.clone())
                .collect()
        };

        let mut count = 0;
        for id in agent_ids {
            if self.kill(&id).is_ok() {
                count += 1;
            }
        }

        info!("Killed {} agents in context '{}'", count, context);
        Ok(count)
    }

    /// Get total cost across all agents
    pub fn total_cost(&self) -> f64 {
        let agents = self.agents.read().unwrap();
        agents.values().map(|a| a.cost.total_usd).sum()
    }

    /// Get cost by context
    pub fn cost_by_context(&self) -> HashMap<String, f64> {
        let agents = self.agents.read().unwrap();
        let mut costs: HashMap<String, f64> = HashMap::new();

        for agent in agents.values() {
            *costs.entry(agent.context.clone()).or_insert(0.0) += agent.cost.total_usd;
        }

        costs
    }

    /// Get summary statistics
    pub fn stats(&self) -> ManagerStats {
        let agents = self.agents.read().unwrap();

        let total_agents = agents.len();
        let active_agents = agents.values().filter(|a| a.status.is_active()).count();
        let completed_agents = agents
            .values()
            .filter(|a| a.status == AgentStatus::Completed)
            .count();
        let errored_agents = agents
            .values()
            .filter(|a| a.status == AgentStatus::Error)
            .count();
        let total_cost: f64 = agents.values().map(|a| a.cost.total_usd).sum();

        // Calculate total budget from all contexts
        let budgets = self.budgets.read().unwrap();
        let total_budget: f64 = budgets.values().filter_map(|b| b.limit).sum();

        ManagerStats {
            total_agents,
            active_agents,
            completed_agents,
            errored_agents,
            total_cost,
            total_budget,
        }
    }

    /// Clean up finished agents older than the given duration
    pub fn cleanup_finished(&self, max_age: std::time::Duration) -> u32 {
        let mut agents = self.agents.write().unwrap();
        let before = agents.len();

        agents.retain(|_, agent| {
            if agent.status.is_finished() {
                agent.runtime() < max_age
            } else {
                true
            }
        });

        let removed = before - agents.len();
        if removed > 0 {
            debug!("Cleaned up {} finished agents", removed);
        }
        removed as u32
    }
}

/// Statistics about the agent manager
#[derive(Debug, Clone, Default)]
pub struct ManagerStats {
    pub total_agents: usize,
    pub active_agents: usize,
    pub completed_agents: usize,
    pub errored_agents: usize,
    pub total_cost: f64,
    pub total_budget: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_agent() {
        let manager = AgentManager::new();

        let request = SpawnRequest::new("test_agent", "work", "Test task");
        let id = manager.spawn(request).unwrap();

        assert!(id.starts_with("agent-"));

        let agent = manager.get(&id).unwrap();
        assert_eq!(agent.name, "test_agent");
        assert_eq!(agent.context, "work");
        assert_eq!(agent.status, AgentStatus::Starting);
    }

    #[test]
    fn test_agent_lifecycle() {
        let manager = AgentManager::new();

        let request = SpawnRequest::new("lifecycle_agent", "work", "Test lifecycle");
        let id = manager.spawn(request).unwrap();

        // Update status to running
        manager
            .update_status(&id, AgentStatus::Running, "Working...")
            .unwrap();

        let agent = manager.get(&id).unwrap();
        assert_eq!(agent.status, AgentStatus::Running);

        // Pause
        manager.pause(&id).unwrap();
        let agent = manager.get(&id).unwrap();
        assert_eq!(agent.status, AgentStatus::Paused);

        // Resume
        manager.resume(&id).unwrap();
        let agent = manager.get(&id).unwrap();
        assert_eq!(agent.status, AgentStatus::Running);

        // Kill
        manager.kill(&id).unwrap();
        let agent = manager.get(&id).unwrap();
        assert_eq!(agent.status, AgentStatus::Killed);
    }

    #[test]
    fn test_budget_tracking() {
        let manager = AgentManager::new();
        manager.set_budget("work", 10.0);

        let request = SpawnRequest::new("budget_agent", "work", "Test budget");
        let id = manager.spawn(request).unwrap();

        // Add some cost
        manager.add_cost(&id, 1000, 500, 2.50).unwrap();

        let agent = manager.get(&id).unwrap();
        assert_eq!(agent.cost.total_usd, 2.50);

        let budget = manager.get_budget("work").unwrap();
        assert_eq!(budget.spent, 2.50);
        assert_eq!(budget.remaining(), Some(7.50));
    }

    #[test]
    fn test_list_by_context() {
        let manager = AgentManager::new();

        manager
            .spawn(SpawnRequest::new("work1", "work", "Task 1"))
            .unwrap();
        manager
            .spawn(SpawnRequest::new("work2", "work", "Task 2"))
            .unwrap();
        manager
            .spawn(SpawnRequest::new("personal1", "personal", "Task 3"))
            .unwrap();

        let work_agents = manager.list_by_context("work");
        assert_eq!(work_agents.len(), 2);

        let personal_agents = manager.list_by_context("personal");
        assert_eq!(personal_agents.len(), 1);
    }

    #[test]
    fn test_manager_stats() {
        let manager = AgentManager::new();

        let id1 = manager
            .spawn(SpawnRequest::new("agent1", "work", "Task 1"))
            .unwrap();
        let id2 = manager
            .spawn(SpawnRequest::new("agent2", "work", "Task 2"))
            .unwrap();

        manager
            .update_status(&id1, AgentStatus::Running, "Working")
            .unwrap();
        manager
            .update_status(&id2, AgentStatus::Completed, "Done")
            .unwrap();

        let stats = manager.stats();
        assert_eq!(stats.total_agents, 2);
        assert_eq!(stats.active_agents, 1);
        assert_eq!(stats.completed_agents, 1);
    }
}
