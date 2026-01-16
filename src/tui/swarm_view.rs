//! Swarm view for the TUI
//!
//! Displays swarm molecule status from bd swarm.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::process::Command;

/// Swarm view state
pub struct SwarmView {
    /// List state for keyboard navigation
    list_state: ListState,
    /// Cached swarm molecules
    molecules: Vec<SwarmMolecule>,
    /// Status message
    status_message: String,
    /// Show detail view
    show_detail: bool,
}

/// A swarm molecule from bd swarm list
#[derive(Clone)]
pub struct SwarmMolecule {
    pub id: String,
    pub epic_id: String,
    pub status: String,
    pub coordinator: Option<String>,
    pub children_count: usize,
}

impl SwarmView {
    /// Create a new swarm view
    pub fn new() -> Self {
        let mut view = Self {
            list_state: ListState::default(),
            molecules: Vec::new(),
            status_message: String::new(),
            show_detail: false,
        };
        view.refresh();
        view
    }

    /// Refresh swarm data from bd
    pub fn refresh(&mut self) {
        // Try to get swarm list from bd
        match Command::new("bd")
            .args(["swarm", "list", "--json"])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    // Parse JSON output (simplified - would need proper parsing)
                    if stdout.trim().is_empty() || stdout.contains("[]") {
                        self.molecules.clear();
                        self.status_message = "No swarm molecules found. Create one with: bd swarm create <epic-id>".to_string();
                    } else {
                        self.status_message = "Swarm molecules loaded".to_string();
                        // For now, just show that we have data
                        // TODO: Parse the JSON properly
                    }
                } else {
                    self.status_message = "bd swarm not available".to_string();
                }
            }
            Err(_) => {
                self.status_message = "bd command not found".to_string();
            }
        }
    }

    /// Check if we have a manager (compatibility stub)
    pub fn has_manager(&self) -> bool {
        false
    }

    /// Get active agent count (compatibility stub)
    pub fn active_count(&self) -> usize {
        self.molecules.len()
    }

    /// Set manager (compatibility stub - does nothing)
    pub fn set_manager<T>(&mut self, _manager: T) {
        // No-op - we don't use a manager anymore
    }

    /// Move to next item
    pub fn next(&mut self) {
        if self.molecules.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.molecules.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Move to previous item
    pub fn previous(&mut self) {
        if self.molecules.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.molecules.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Toggle detail view
    pub fn toggle_detail(&mut self) {
        self.show_detail = !self.show_detail;
    }

    /// Close detail view
    pub fn close_detail(&mut self) {
        self.show_detail = false;
    }

    /// Pause selected (stub - not applicable for molecules)
    pub fn pause_selected(&mut self) {
        // No-op for swarm molecules
    }

    /// Resume selected (stub - not applicable for molecules)
    pub fn resume_selected(&mut self) {
        // No-op for swarm molecules
    }

    /// Kill selected (stub - not applicable for molecules)
    pub fn kill_selected(&mut self) {
        // No-op for swarm molecules
    }
}

impl Default for SwarmView {
    fn default() -> Self {
        Self::new()
    }
}

/// Draw the swarm view
pub fn draw(f: &mut Frame, swarm_view: &mut SwarmView, area: Rect) {
    let block = Block::default()
        .title("Swarm Molecules")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    if swarm_view.molecules.is_empty() {
        // Show empty state message
        let message = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No swarm molecules",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(""),
            Line::from(swarm_view.status_message.as_str()),
            Line::from(""),
            Line::from("Swarm molecules orchestrate parallel work on epics."),
            Line::from(""),
            Line::from(Span::styled(
                "Create one with: bd swarm create <epic-id>",
                Style::default().fg(Color::Green),
            )),
        ])
        .block(block)
        .wrap(Wrap { trim: true });

        f.render_widget(message, area);
    } else {
        // Show molecule list
        let items: Vec<ListItem> = swarm_view
            .molecules
            .iter()
            .map(|m| {
                let status_color = match m.status.as_str() {
                    "active" => Color::Green,
                    "completed" => Color::Blue,
                    "paused" => Color::Yellow,
                    _ => Color::Gray,
                };
                ListItem::new(Line::from(vec![
                    Span::styled(&m.id, Style::default().fg(Color::Cyan)),
                    Span::raw(" → "),
                    Span::styled(&m.epic_id, Style::default().fg(Color::White)),
                    Span::raw(" ["),
                    Span::styled(&m.status, Style::default().fg(status_color)),
                    Span::raw("]"),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray),
            )
            .highlight_symbol("→ ");

        f.render_stateful_widget(list, area, &mut swarm_view.list_state);
    }
}
