//! Aiki Tasks view for TUI
//!
//! Displays AllBeads beads with their linked Aiki tasks.
//! Queries Aiki for task details in real-time.

use crate::graph::{Bead, BeadId, FederatedGraph};
use ratatui::widgets::ListState;
use std::collections::HashMap;
use std::process::Command;

/// Aiki task details parsed from XML
#[derive(Debug, Clone)]
pub struct AikiTask {
    pub id: String,
    pub title: String,
    pub status: String,
}

/// View showing beads with linked Aiki tasks
pub struct AikiView {
    /// List state for keyboard navigation
    pub list_state: ListState,
    /// Beads that have linked Aiki tasks (cached)
    pub beads_with_tasks: Vec<BeadId>,
    /// Aiki task details (id -> task)
    pub task_details: HashMap<String, AikiTask>,
    /// Whether Aiki is available
    pub aiki_available: bool,
    /// Last query time
    pub last_query: Option<std::time::Instant>,
}

impl AikiView {
    /// Create a new Aiki view
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            list_state,
            beads_with_tasks: Vec::new(),
            task_details: HashMap::new(),
            aiki_available: false,
            last_query: None,
        }
    }

    /// Refresh data from the graph
    pub fn refresh(&mut self, graph: &FederatedGraph) {
        // Find all beads with linked Aiki tasks
        self.beads_with_tasks = graph
            .beads
            .values()
            .filter(|b| !b.aiki_tasks.is_empty())
            .map(|b| b.id.clone())
            .collect();

        // Sort by priority then updated time
        self.beads_with_tasks.sort_by(|a, b| {
            let bead_a = graph.beads.get(a);
            let bead_b = graph.beads.get(b);
            match (bead_a, bead_b) {
                (Some(a), Some(b)) => a
                    .priority
                    .cmp(&b.priority)
                    .then(b.updated_at.cmp(&a.updated_at)),
                _ => std::cmp::Ordering::Equal,
            }
        });

        // Query Aiki for task details (cache for 30 seconds)
        let should_query = self.last_query.is_none_or(|t| t.elapsed().as_secs() > 30);
        if should_query {
            self.query_aiki_tasks();
            self.last_query = Some(std::time::Instant::now());
        }
    }

    /// Query Aiki for task details
    fn query_aiki_tasks(&mut self) {
        let output = Command::new("aiki")
            .args(["task", "list", "--format=xml"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                self.aiki_available = true;
                let stdout = String::from_utf8_lossy(&output.stdout);
                self.task_details = Self::parse_aiki_xml(&stdout);
            }
            _ => {
                self.aiki_available = false;
                self.task_details.clear();
            }
        }
    }

    /// Parse Aiki XML task list
    fn parse_aiki_xml(xml: &str) -> HashMap<String, AikiTask> {
        let mut tasks = HashMap::new();

        // Split by <task and process each task block
        for chunk in xml.split("<task") {
            if chunk.trim().is_empty() {
                continue;
            }

            // Extract id from id="..."
            let id = if let Some(start) = chunk.find("id=\"") {
                let rest = &chunk[start + 4..];
                rest.find('"').map(|end| &rest[..end])
            } else {
                None
            };

            // Extract title from <title>...</title>
            let title = if let Some(start) = chunk.find("<title>") {
                let rest = &chunk[start + 7..];
                rest.find("</title>").map(|end| &rest[..end])
            } else {
                None
            };

            // Extract status from <status>...</status>
            let status = if let Some(start) = chunk.find("<status>") {
                let rest = &chunk[start + 8..];
                rest.find("</status>").map(|end| &rest[..end])
            } else {
                None
            };

            if let (Some(id), Some(title), Some(status)) = (id, title, status) {
                tasks.insert(
                    id.to_string(),
                    AikiTask {
                        id: id.to_string(),
                        title: title.to_string(),
                        status: status.to_string(),
                    },
                );
            }
        }

        tasks
    }

    /// Get currently selected bead
    pub fn selected_bead<'a>(&self, graph: &'a FederatedGraph) -> Option<&'a Bead> {
        self.list_state
            .selected()
            .and_then(|idx| self.beads_with_tasks.get(idx))
            .and_then(|id| graph.beads.get(id))
    }

    /// Navigate to next item
    pub fn next(&mut self) {
        if self.beads_with_tasks.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.beads_with_tasks.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Navigate to previous item
    pub fn previous(&mut self) {
        if self.beads_with_tasks.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.beads_with_tasks.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }
}

impl Default for AikiView {
    fn default() -> Self {
        Self::new()
    }
}
