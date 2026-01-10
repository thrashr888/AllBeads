//! Agent Swarm Management
//!
//! Provides lifecycle management for AI agents (Polecats), including spawning,
//! monitoring, cost tracking, and budget management.
//!
//! # Overview
//!
//! The swarm module implements Phase 5 of AllBeads - the self-managing agent workforce.
//! It enables:
//!
//! - **Agent Lifecycle**: Spawn, pause, resume, and kill agents
//! - **Cost Tracking**: Track API costs per agent and per context
//! - **Budget Management**: Set limits and receive warnings when approaching limits
//! - **Real-time Monitoring**: View agent status, runtime, and file locks
//!
//! # Agent Types (Polecats)
//!
//! Agents are ephemeral worker processes that execute specific tasks:
//!
//! - `RefactorBot`: Code refactoring specialist
//! - `TestWriter`: Unit and integration test generator
//! - `SecuritySpecialist`: Security audit and fixes
//! - `FrontendExpert`: UI/UX implementation
//! - `BackendDeveloper`: API and service implementation
//! - `DevOps`: Infrastructure and CI/CD
//! - `TechWriter`: Documentation
//!
//! # Example
//!
//! ```ignore
//! use allbeads::swarm::{AgentManager, SpawnRequest, AgentPersona};
//!
//! // Create manager
//! let manager = AgentManager::new();
//!
//! // Set budget for work context
//! manager.set_budget("work", 50.0);
//!
//! // Spawn an agent
//! let request = SpawnRequest::new("refactor_bot", "work", "Refactor auth module")
//!     .with_persona(AgentPersona::RefactorBot)
//!     .with_rig("auth-service")
//!     .with_budget(5.0);
//!
//! let agent_id = manager.spawn(request)?;
//!
//! // Monitor status
//! let agent = manager.get(&agent_id)?;
//! println!("Status: {:?}", agent.status);
//!
//! // Add API cost
//! manager.add_cost(&agent_id, 1000, 500, 0.05)?;
//!
//! // Kill when done
//! manager.kill(&agent_id)?;
//! ```

mod agent;
mod manager;

pub use agent::{Agent, AgentCost, AgentPersona, AgentStatus, SpawnRequest};
pub use manager::{AgentManager, ContextBudget, ManagerEvent, ManagerStats};
