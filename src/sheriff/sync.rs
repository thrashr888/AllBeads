//! Shadow Bead synchronization
//!
//! Handles syncing beads between Rig repositories and the Boss repo,
//! creating Shadow Beads for Epic-level items.

use crate::graph::{Bead, BeadId, IssueType, RigId, ShadowBead};
use crate::storage::BeadsRepo;
use crate::Result;
use std::collections::HashMap;
use std::path::Path;

/// Synchronization result for a single Rig
#[derive(Debug, Default, Clone)]
pub struct SyncResult {
    /// Shadow beads created
    pub created: Vec<BeadId>,

    /// Shadow beads updated
    pub updated: Vec<BeadId>,

    /// Shadow beads deleted (native bead removed)
    pub deleted: Vec<BeadId>,

    /// Errors encountered during sync
    pub errors: Vec<String>,
}

impl SyncResult {
    /// Check if sync had any changes
    pub fn has_changes(&self) -> bool {
        !self.created.is_empty() || !self.updated.is_empty() || !self.deleted.is_empty()
    }

    /// Total number of changes
    pub fn change_count(&self) -> usize {
        self.created.len() + self.updated.len() + self.deleted.len()
    }
}

/// Shadow Bead synchronizer
///
/// Syncs beads from a Rig repository to Shadow Beads in the Boss repo.
pub struct ShadowSync {
    /// Rig ID for namespacing
    rig_id: RigId,

    /// Context name (e.g., "work", "personal")
    context: String,

    /// Prefix for shadow bead IDs
    shadow_prefix: String,

    /// Existing shadow beads indexed by native bead URI
    shadow_index: HashMap<String, ShadowBead>,
}

impl ShadowSync {
    /// Create a new sync instance
    pub fn new(rig_id: impl Into<RigId>, context: impl Into<String>) -> Self {
        let rig_id = rig_id.into();
        let shadow_prefix = format!("shadow-{}", rig_id.as_str());

        Self {
            rig_id,
            context: context.into(),
            shadow_prefix,
            shadow_index: HashMap::new(),
        }
    }

    /// Set existing shadow beads for diffing
    pub fn with_shadows(mut self, shadows: Vec<ShadowBead>) -> Self {
        for shadow in shadows {
            self.shadow_index
                .insert(shadow.pointer.as_str().to_string(), shadow);
        }
        self
    }

    /// Sync beads from a Rig, returning Shadow Beads to create/update
    pub fn sync(&self, native_beads: &[Bead]) -> SyncResult {
        let mut result = SyncResult::default();

        // Track which shadows we've seen (for deletion detection)
        let mut seen_pointers: std::collections::HashSet<String> = std::collections::HashSet::new();

        for bead in native_beads {
            // Only sync Epic-level beads
            if !self.should_sync(bead) {
                continue;
            }

            let pointer_uri = format!("bead://{}/{}", self.rig_id.as_str(), bead.id.as_str());
            seen_pointers.insert(pointer_uri.clone());

            if let Some(existing) = self.shadow_index.get(&pointer_uri) {
                // Check if update needed
                if self.needs_update(existing, bead) {
                    result.updated.push(bead.id.clone());
                }
            } else {
                // New shadow needed
                result.created.push(bead.id.clone());
            }
        }

        // Check for deleted beads
        for pointer_uri in self.shadow_index.keys() {
            if !seen_pointers.contains(pointer_uri) {
                if let Some(shadow) = self.shadow_index.get(pointer_uri) {
                    result.deleted.push(shadow.id.clone());
                }
            }
        }

        result
    }

    /// Create a Shadow Bead from a native Bead
    pub fn create_shadow(&self, bead: &Bead) -> ShadowBead {
        let shadow_id = BeadId::new(format!(
            "{}-{}",
            self.shadow_prefix,
            &bead.id.as_str()[..std::cmp::min(6, bead.id.as_str().len())]
        ));

        let mut shadow = ShadowBead::new(
            shadow_id,
            &self.rig_id,
            &bead.id,
            &bead.title,
            &self.context,
        );

        shadow.status = bead.status;
        shadow.labels = bead.labels.clone();

        shadow
    }

    /// Update a Shadow Bead from a native Bead
    pub fn update_shadow(&self, shadow: &mut ShadowBead, bead: &Bead) {
        shadow.summary = bead.title.clone();
        shadow.status = bead.status;
        shadow.labels = bead.labels.clone();
        shadow.update_sync_time();
    }

    /// Check if a bead should be synced (Epic-level only by default)
    pub fn should_sync(&self, bead: &Bead) -> bool {
        use crate::graph::Priority;
        // Sync Epics and high-priority items (P0 or P1)
        bead.issue_type == IssueType::Epic
            || bead.priority == Priority::P0
            || bead.priority == Priority::P1
    }

    /// Check if a shadow needs updating
    fn needs_update(&self, shadow: &ShadowBead, bead: &Bead) -> bool {
        shadow.summary != bead.title
            || shadow.status != bead.status
            || shadow.labels != bead.labels
    }
}

/// Sync a Rig's beads to Shadow Beads
///
/// # Arguments
/// * `rig_path` - Path to the Rig repository
/// * `rig_id` - Rig identifier
/// * `context` - Context name
/// * `existing_shadows` - Existing Shadow Beads (for diffing)
pub fn sync_rig_to_shadows(
    rig_path: &Path,
    rig_id: &str,
    context: &str,
    existing_shadows: Vec<ShadowBead>,
) -> Result<(SyncResult, Vec<ShadowBead>)> {
    // Load beads from the Rig
    let beads_repo = BeadsRepo::with_workdir(rig_path);
    let native_beads = beads_repo.list_all()?;

    // Set up sync
    let sync = ShadowSync::new(rig_id, context).with_shadows(existing_shadows.clone());

    // Calculate diff
    let result = sync.sync(&native_beads);

    // Build new shadow list
    let mut shadows: Vec<ShadowBead> = Vec::new();
    let mut shadow_map: HashMap<String, ShadowBead> = existing_shadows
        .into_iter()
        .map(|s| (s.pointer.as_str().to_string(), s))
        .collect();

    for bead in native_beads.iter().filter(|b| sync.should_sync(b)) {
        let pointer_uri = format!("bead://{}/{}", rig_id, bead.id.as_str());

        let shadow = if let Some(mut existing) = shadow_map.remove(&pointer_uri) {
            sync.update_shadow(&mut existing, bead);
            existing
        } else {
            sync.create_shadow(bead)
        };

        shadows.push(shadow);
    }

    Ok((result, shadows))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Priority;

    fn make_bead(id: &str, title: &str, issue_type: IssueType, priority: Priority) -> Bead {
        let mut bead = Bead::new(id, title, "test");
        bead.issue_type = issue_type;
        bead.priority = priority;
        bead
    }

    #[test]
    fn test_sync_creates_shadows_for_epics() {
        let beads = vec![
            make_bead("auth-001", "Refactor auth", IssueType::Epic, Priority::P2),
            make_bead("auth-002", "Fix bug", IssueType::Bug, Priority::P3),
            make_bead("auth-003", "Add feature", IssueType::Feature, Priority::P0),
        ];

        let sync = ShadowSync::new("auth-service", "work");
        let result = sync.sync(&beads);

        // Epic and P0 feature should be synced
        assert_eq!(result.created.len(), 2);
        assert!(result.created.iter().any(|id| id.as_str() == "auth-001"));
        assert!(result.created.iter().any(|id| id.as_str() == "auth-003"));
    }

    #[test]
    fn test_sync_detects_updates() {
        let rig_id = RigId::new("auth-service");
        let native_id = BeadId::new("auth-001");

        // Existing shadow with old title
        let shadow = ShadowBead::new(
            "shadow-auth-001",
            &rig_id,
            &native_id,
            "Old Title",
            "work",
        );

        // Native bead with new title
        let beads = vec![make_bead(
            "auth-001",
            "New Title",
            IssueType::Epic,
            Priority::P2,
        )];

        let sync = ShadowSync::new("auth-service", "work").with_shadows(vec![shadow]);
        let result = sync.sync(&beads);

        assert_eq!(result.updated.len(), 1);
        assert_eq!(result.updated[0].as_str(), "auth-001");
    }

    #[test]
    fn test_sync_detects_deletions() {
        let rig_id = RigId::new("auth-service");
        let native_id = BeadId::new("auth-999");

        // Shadow for a bead that no longer exists
        let shadow = ShadowBead::new(
            "shadow-auth-999",
            &rig_id,
            &native_id,
            "Deleted Bead",
            "work",
        );

        // Empty beads list
        let beads: Vec<Bead> = vec![];

        let sync = ShadowSync::new("auth-service", "work").with_shadows(vec![shadow]);
        let result = sync.sync(&beads);

        assert_eq!(result.deleted.len(), 1);
        assert_eq!(result.deleted[0].as_str(), "shadow-auth-999");
    }

    #[test]
    fn test_create_shadow() {
        let bead = make_bead("auth-001", "Refactor auth", IssueType::Epic, Priority::P2);
        let sync = ShadowSync::new("auth-service", "work");

        let shadow = sync.create_shadow(&bead);

        assert!(shadow.id.as_str().starts_with("shadow-auth-service-"));
        assert_eq!(shadow.summary, "Refactor auth");
        assert_eq!(shadow.context, "work");
    }
}
