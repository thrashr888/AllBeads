//! TUI application state

use super::graph_view::GraphView;
use super::mail_view::MailView;
use super::swarm_view::SwarmView;
use crate::graph::{Bead, FederatedGraph, Status};
use crate::mail::{Address, Postmaster};
use crate::swarm::AgentManager;
use ratatui::widgets::ListState;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Active tab in the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Kanban,
    Mail,
    Graph,
    Swarm,
}

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
    pub current_tab: Tab,
    pub mail_view: MailView,
    pub graph_view: GraphView,
    pub swarm_view: SwarmView,
    pub postmaster: Option<Arc<Mutex<Postmaster>>>,
    pub inbox_address: Address,
}

impl App {
    pub fn new(graph: FederatedGraph) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let mut graph_view = GraphView::new();
        graph_view.analyze(&graph);
        Self {
            graph,
            current_column: Column::Open,
            list_state,
            show_detail: false,
            current_tab: Tab::Kanban,
            mail_view: MailView::new(),
            graph_view,
            swarm_view: SwarmView::new(),
            postmaster: None,
            inbox_address: Address::human(),
        }
    }

    /// Create app with mail support
    pub fn with_mail(graph: FederatedGraph, mail_db_path: PathBuf, project_id: &str) -> Self {
        let mut app = Self::new(graph);
        if let Ok(postmaster) = Postmaster::with_project_id(mail_db_path, project_id) {
            app.postmaster = Some(Arc::new(Mutex::new(postmaster)));
            app.refresh_mail();
        }
        app
    }

    /// Refresh mail inbox
    pub fn refresh_mail(&mut self) {
        if let Some(ref postmaster) = self.postmaster {
            if let Ok(pm) = postmaster.lock() {
                self.mail_view.refresh(&pm, &self.inbox_address);
            }
        }
    }

    /// Check if mail is available
    pub fn has_mail(&self) -> bool {
        self.postmaster.is_some()
    }

    /// Get unread mail count
    pub fn unread_mail_count(&self) -> usize {
        self.mail_view.unread_count()
    }

    /// Check if swarm is available
    pub fn has_swarm(&self) -> bool {
        self.swarm_view.has_manager()
    }

    /// Get active agent count
    pub fn active_agent_count(&self) -> usize {
        self.swarm_view.active_count()
    }

    /// Set the agent manager for swarm view
    pub fn set_agent_manager(&mut self, manager: Arc<AgentManager>) {
        self.swarm_view.set_manager(manager);
    }

    /// Switch to next tab
    /// Tab order: Kanban -> Mail (if available) -> Graph -> Swarm (if available) -> Kanban
    pub fn next_tab(&mut self) {
        let has_mail = self.has_mail();
        let has_swarm = self.has_swarm();

        self.current_tab = match self.current_tab {
            Tab::Kanban => {
                if has_mail {
                    Tab::Mail
                } else {
                    Tab::Graph
                }
            }
            Tab::Mail => Tab::Graph,
            Tab::Graph => {
                if has_swarm {
                    Tab::Swarm
                } else {
                    Tab::Kanban
                }
            }
            Tab::Swarm => Tab::Kanban,
        };

        // Refresh data when switching to specific tabs
        match self.current_tab {
            Tab::Mail => self.refresh_mail(),
            Tab::Graph => self.graph_view.analyze(&self.graph),
            Tab::Swarm => self.swarm_view.refresh(),
            _ => {}
        }
    }

    /// Mark selected message as read
    pub fn mark_message_read(&mut self) {
        if let Some(ref postmaster) = self.postmaster {
            if let Some(msg_id) = self.mail_view.selected_message_id() {
                if let Ok(pm) = postmaster.lock() {
                    let _ = pm.mark_read(msg_id);
                }
            }
            self.refresh_mail();
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
