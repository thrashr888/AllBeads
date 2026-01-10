//! TUI application state

use crate::graph::{Bead, FederatedGraph, Status};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Open,
    InProgress,
    Closed,
}

impl Column {
    pub fn to_status(&self) -> Status {
        match self {
            Column::Open => Status::Open,
            Column::InProgress => Status::InProgress,
            Column::Closed => Status::Closed,
        }
    }

    pub fn title(&self) -> &'static str {
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
    pub selected_index: usize,
    pub show_detail: bool,
}

impl App {
    pub fn new(graph: FederatedGraph) -> Self {
        Self {
            graph,
            current_column: Column::Open,
            selected_index: 0,
            show_detail: false,
        }
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
        beads.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.title.cmp(&b.title)));

        beads
    }

    /// Get the currently selected bead
    pub fn selected_bead(&self) -> Option<&Bead> {
        let beads = self.current_beads();
        beads.get(self.selected_index).copied()
    }

    pub fn next(&mut self) {
        let beads = self.current_beads();
        if !beads.is_empty() {
            self.selected_index = (self.selected_index + 1) % beads.len();
        }
    }

    pub fn previous(&mut self) {
        let beads = self.current_beads();
        if !beads.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = beads.len().saturating_sub(1);
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub fn next_column(&mut self) {
        self.current_column = match self.current_column {
            Column::Open => Column::InProgress,
            Column::InProgress => Column::Closed,
            Column::Closed => Column::Closed,
        };
        self.selected_index = 0;
    }

    pub fn previous_column(&mut self) {
        self.current_column = match self.current_column {
            Column::Open => Column::Open,
            Column::InProgress => Column::Open,
            Column::Closed => Column::InProgress,
        };
        self.selected_index = 0;
    }

    pub fn toggle_detail(&mut self) {
        self.show_detail = !self.show_detail;
    }

    pub fn close_detail(&mut self) {
        self.show_detail = false;
    }
}
