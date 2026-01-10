//! TUI application state

use crate::graph::{Bead, FederatedGraph, Status};
use ratatui::widgets::ListState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Open,
    InProgress,
    Closed,
}

impl Column {
    pub fn to_status(self) -> Status {
        match self {
            Column::Open => Status::Open,
            Column::InProgress => Status::InProgress,
            Column::Closed => Status::Closed,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Column::Open => "Open",
            Column::InProgress => "In Progress",
            Column::Closed => "Closed",
        }
    }

    pub fn all() -> [Column; 3] {
        [Column::Open, Column::InProgress, Column::Closed]
    }
}

pub struct App {
    pub graph: FederatedGraph,
    pub current_column: Column,
    pub list_state: ListState,
    pub show_detail: bool,
}

impl App {
    pub fn new(graph: FederatedGraph) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            graph,
            current_column: Column::Open,
            list_state,
            show_detail: false,
        }
    }

    /// Get the currently selected index
    pub fn selected_index(&self) -> usize {
        self.list_state.selected().unwrap_or(0)
    }

    /// Get beads for the current column
    pub fn current_beads(&self) -> Vec<&Bead> {
        let status = self.current_column.to_status();
        let mut beads: Vec<_> = self
            .graph
            .beads
            .values()
            .filter(|b| b.status == status)
            .collect();

        // Sort by priority then title
        beads.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.title.cmp(&b.title))
        });

        beads
    }

    /// Get the currently selected bead
    pub fn selected_bead(&self) -> Option<&Bead> {
        let beads = self.current_beads();
        let index = self.list_state.selected().unwrap_or(0);
        beads.get(index).copied()
    }

    pub fn next(&mut self) {
        let beads = self.current_beads();
        if beads.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let next = if current >= beads.len().saturating_sub(1) {
            0 // Wrap to beginning
        } else {
            current + 1
        };
        self.list_state.select(Some(next));
    }

    pub fn previous(&mut self) {
        let beads = self.current_beads();
        if beads.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let prev = if current == 0 {
            beads.len().saturating_sub(1) // Wrap to end
        } else {
            current - 1
        };
        self.list_state.select(Some(prev));
    }

    pub fn next_column(&mut self) {
        self.current_column = match self.current_column {
            Column::Open => Column::InProgress,
            Column::InProgress => Column::Closed,
            Column::Closed => Column::Closed,
        };
        // Reset selection for new column
        self.list_state.select(Some(0));
        *self.list_state.offset_mut() = 0;
    }

    pub fn previous_column(&mut self) {
        self.current_column = match self.current_column {
            Column::Open => Column::Open,
            Column::InProgress => Column::Open,
            Column::Closed => Column::InProgress,
        };
        // Reset selection for new column
        self.list_state.select(Some(0));
        *self.list_state.offset_mut() = 0;
    }

    pub fn toggle_detail(&mut self) {
        self.show_detail = !self.show_detail;
    }

    pub fn close_detail(&mut self) {
        self.show_detail = false;
    }
}
