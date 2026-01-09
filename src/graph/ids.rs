//! Type-safe ID wrappers for beads and rigs

use std::fmt;

/// Type-safe wrapper for bead IDs
///
/// Prevents mixing up bead IDs with rig IDs at compile time.
/// Format: prefix-hash (e.g., "ab-ldr", "work-5fm")
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BeadId(String);

impl BeadId {
    /// Create a new BeadId from a string
    ///
    /// # Arguments
    /// * `id` - The bead ID string (e.g., "ab-ldr")
    ///
    /// # Returns
    /// A new BeadId instance
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the underlying string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extract the prefix from the ID
    ///
    /// # Returns
    /// The prefix portion before the hyphen, if present
    pub fn prefix(&self) -> Option<&str> {
        self.0.split('-').next()
    }

    /// Extract the hash from the ID
    ///
    /// # Returns
    /// The hash portion after the hyphen, if present
    pub fn hash(&self) -> Option<&str> {
        self.0.split('-').nth(1)
    }
}

impl fmt::Display for BeadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for BeadId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for BeadId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Type-safe wrapper for rig IDs
///
/// Prevents mixing up rig IDs with bead IDs at compile time.
/// Format: typically a path or name (e.g., "auth-service", "personal-blog")
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RigId(String);

impl RigId {
    /// Create a new RigId from a string
    ///
    /// # Arguments
    /// * `id` - The rig ID string (e.g., "auth-service")
    ///
    /// # Returns
    /// A new RigId instance
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the underlying string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RigId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for RigId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for RigId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_id_creation() {
        let id = BeadId::new("ab-ldr");
        assert_eq!(id.as_str(), "ab-ldr");
        assert_eq!(id.prefix(), Some("ab"));
        assert_eq!(id.hash(), Some("ldr"));
    }

    #[test]
    fn test_bead_id_display() {
        let id = BeadId::new("work-5fm");
        assert_eq!(format!("{}", id), "work-5fm");
    }

    #[test]
    fn test_rig_id_creation() {
        let id = RigId::new("auth-service");
        assert_eq!(id.as_str(), "auth-service");
    }

    #[test]
    fn test_type_safety() {
        let bead_id = BeadId::new("ab-ldr");
        let rig_id = RigId::new("auth-service");

        // This won't compile if you try to pass a BeadId where RigId is expected
        // (and vice versa), which is exactly what we want!
        fn takes_bead(_id: &BeadId) {}
        fn takes_rig(_id: &RigId) {}

        takes_bead(&bead_id);
        takes_rig(&rig_id);
    }
}
