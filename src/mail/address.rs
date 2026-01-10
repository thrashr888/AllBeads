//! Agent Mail addressing
//!
//! Provides type-safe addressing for agent-to-agent and agent-to-human communication.
//!
//! # Address Format
//!
//! Addresses follow an email-like format: `name@domain`
//!
//! - `agent_name@project_id` - Specific agent in a project
//! - `human@localhost` - Human operator inbox
//! - `all@project_id` - Broadcast to all agents in a project
//! - `postmaster@project_id` - The postmaster service
//!
//! # Examples
//!
//! ```
//! use allbeads::mail::Address;
//!
//! // Parse an address
//! let addr: Address = "refactor_bot@legacy-repo".parse().unwrap();
//! assert_eq!(addr.name(), "refactor_bot");
//! assert_eq!(addr.domain(), "legacy-repo");
//!
//! // Create special addresses
//! let human = Address::human();
//! let broadcast = Address::broadcast("my-project");
//! ```

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// Error type for address parsing
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AddressError {
    #[error("invalid address format: expected 'name@domain', got '{0}'")]
    InvalidFormat(String),

    #[error("address name cannot be empty")]
    EmptyName,

    #[error("address domain cannot be empty")]
    EmptyDomain,

    #[error("address name contains invalid characters: '{0}'")]
    InvalidNameCharacters(String),

    #[error("address domain contains invalid characters: '{0}'")]
    InvalidDomainCharacters(String),
}

/// A validated agent mail address
///
/// Addresses consist of a name and domain separated by '@'.
/// Names and domains can contain alphanumeric characters, hyphens, and underscores.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Address {
    name: String,
    domain: String,
}

impl Address {
    /// Create a new address from parts
    ///
    /// # Errors
    /// Returns an error if the name or domain are empty or contain invalid characters.
    pub fn new(name: impl Into<String>, domain: impl Into<String>) -> Result<Self, AddressError> {
        let name = name.into();
        let domain = domain.into();

        Self::validate_name(&name)?;
        Self::validate_domain(&domain)?;

        Ok(Self { name, domain })
    }

    /// Create a human inbox address
    ///
    /// Returns `human@localhost`
    pub fn human() -> Self {
        Self {
            name: "human".to_string(),
            domain: "localhost".to_string(),
        }
    }

    /// Create a broadcast address for a project
    ///
    /// Returns `all@{project_id}`
    pub fn broadcast(project_id: impl Into<String>) -> Self {
        Self {
            name: "all".to_string(),
            domain: project_id.into(),
        }
    }

    /// Create a postmaster address for a project
    ///
    /// Returns `postmaster@{project_id}`
    pub fn postmaster(project_id: impl Into<String>) -> Self {
        Self {
            name: "postmaster".to_string(),
            domain: project_id.into(),
        }
    }

    /// Get the name part of the address
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the domain part of the address
    pub fn domain(&self) -> &str {
        &self.domain
    }

    /// Check if this is the human inbox address
    pub fn is_human(&self) -> bool {
        self.name == "human" && self.domain == "localhost"
    }

    /// Check if this is a broadcast address
    pub fn is_broadcast(&self) -> bool {
        self.name == "all"
    }

    /// Check if this is a postmaster address
    pub fn is_postmaster(&self) -> bool {
        self.name == "postmaster"
    }

    /// Check if this address is in the given project
    pub fn is_in_project(&self, project_id: &str) -> bool {
        self.domain == project_id
    }

    /// Validate the name part of an address
    fn validate_name(name: &str) -> Result<(), AddressError> {
        if name.is_empty() {
            return Err(AddressError::EmptyName);
        }

        // Allow alphanumeric, hyphen, underscore
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AddressError::InvalidNameCharacters(name.to_string()));
        }

        Ok(())
    }

    /// Validate the domain part of an address
    fn validate_domain(domain: &str) -> Result<(), AddressError> {
        if domain.is_empty() {
            return Err(AddressError::EmptyDomain);
        }

        // Allow alphanumeric, hyphen, underscore, dot
        if !domain
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(AddressError::InvalidDomainCharacters(domain.to_string()));
        }

        Ok(())
    }
}

impl FromStr for Address {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('@').collect();

        if parts.len() != 2 {
            return Err(AddressError::InvalidFormat(s.to_string()));
        }

        Self::new(parts[0], parts[1])
    }
}

impl TryFrom<String> for Address {
    type Error = AddressError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<Address> for String {
    fn from(addr: Address) -> Self {
        addr.to_string()
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.name, self.domain)
    }
}

/// A routing destination for messages
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingTarget {
    /// Route to a specific agent
    Agent(Address),

    /// Route to the human inbox
    Human,

    /// Broadcast to all agents in a project
    Broadcast { project_id: String },

    /// Route to the postmaster
    Postmaster { project_id: String },
}

impl RoutingTarget {
    /// Determine the routing target for an address
    pub fn from_address(addr: &Address) -> Self {
        if addr.is_human() {
            RoutingTarget::Human
        } else if addr.is_broadcast() {
            RoutingTarget::Broadcast {
                project_id: addr.domain().to_string(),
            }
        } else if addr.is_postmaster() {
            RoutingTarget::Postmaster {
                project_id: addr.domain().to_string(),
            }
        } else {
            RoutingTarget::Agent(addr.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_parsing() {
        let addr: Address = "refactor_bot@legacy-repo".parse().unwrap();
        assert_eq!(addr.name(), "refactor_bot");
        assert_eq!(addr.domain(), "legacy-repo");
    }

    #[test]
    fn test_address_display() {
        let addr = Address::new("agent", "project").unwrap();
        assert_eq!(addr.to_string(), "agent@project");
    }

    #[test]
    fn test_human_address() {
        let addr = Address::human();
        assert!(addr.is_human());
        assert_eq!(addr.to_string(), "human@localhost");
    }

    #[test]
    fn test_broadcast_address() {
        let addr = Address::broadcast("my-project");
        assert!(addr.is_broadcast());
        assert_eq!(addr.to_string(), "all@my-project");
        assert!(addr.is_in_project("my-project"));
    }

    #[test]
    fn test_postmaster_address() {
        let addr = Address::postmaster("my-project");
        assert!(addr.is_postmaster());
        assert_eq!(addr.to_string(), "postmaster@my-project");
    }

    #[test]
    fn test_invalid_format() {
        let result: Result<Address, _> = "no-at-sign".parse();
        assert!(matches!(result, Err(AddressError::InvalidFormat(_))));

        let result: Result<Address, _> = "too@many@signs".parse();
        assert!(matches!(result, Err(AddressError::InvalidFormat(_))));
    }

    #[test]
    fn test_empty_parts() {
        let result: Result<Address, _> = "@domain".parse();
        assert!(matches!(result, Err(AddressError::EmptyName)));

        let result: Result<Address, _> = "name@".parse();
        assert!(matches!(result, Err(AddressError::EmptyDomain)));
    }

    #[test]
    fn test_invalid_characters() {
        let result: Result<Address, _> = "name with spaces@domain".parse();
        assert!(matches!(result, Err(AddressError::InvalidNameCharacters(_))));

        let result: Result<Address, _> = "name@domain with spaces".parse();
        assert!(matches!(
            result,
            Err(AddressError::InvalidDomainCharacters(_))
        ));
    }

    #[test]
    fn test_routing_target() {
        let human = Address::human();
        assert_eq!(RoutingTarget::from_address(&human), RoutingTarget::Human);

        let broadcast = Address::broadcast("proj");
        assert_eq!(
            RoutingTarget::from_address(&broadcast),
            RoutingTarget::Broadcast {
                project_id: "proj".to_string()
            }
        );

        let agent: Address = "bot@proj".parse().unwrap();
        assert!(matches!(
            RoutingTarget::from_address(&agent),
            RoutingTarget::Agent(_)
        ));
    }

    #[test]
    fn test_serde_roundtrip() {
        let addr: Address = "agent@project".parse().unwrap();
        let json = serde_json::to_string(&addr).unwrap();
        assert_eq!(json, "\"agent@project\"");

        let parsed: Address = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, addr);
    }
}
