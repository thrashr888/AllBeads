//! Timeline view for the TUI
//!
//! Displays beads in a Gantt-style timeline visualization.

use crate::graph::{BeadId, FederatedGraph, Priority, Status};
use chrono::{DateTime, NaiveDate, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

/// A bead with its timeline position
#[derive(Debug, Clone)]
pub struct TimelineBead {
    pub id: BeadId,
    pub title: String,
    pub status: Status,
    pub priority: Priority,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub days_open: i64,
    pub dependencies: Vec<BeadId>,
}

/// Timeline view state
pub struct TimelineView {
    /// Beads sorted by creation date
    beads: Vec<TimelineBead>,
    /// List state for selection
    list_state: ListState,
    /// Show detail view
    show_detail: bool,
    /// Timeline range in days (from today going back)
    range_days: i64,
    /// Earliest date in the timeline
    earliest_date: Option<NaiveDate>,
    /// Latest date in the timeline
    latest_date: Option<NaiveDate>,
}

impl TimelineView {
    /// Create a new timeline view
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            beads: Vec::new(),
            list_state,
            show_detail: false,
            range_days: 30,
            earliest_date: None,
            latest_date: None,
        }
    }

    /// Analyze the graph and populate timeline data
    pub fn analyze(&mut self, graph: &FederatedGraph) {
        self.beads.clear();

        let now = Utc::now();

        for bead in graph.beads.values() {
            // Parse created_at timestamp
            let created_at = DateTime::parse_from_rfc3339(&bead.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or(now);

            let updated_at = DateTime::parse_from_rfc3339(&bead.updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or(now);

            let days_open = if bead.status == Status::Closed {
                (updated_at - created_at).num_days()
            } else {
                (now - created_at).num_days()
            };

            self.beads.push(TimelineBead {
                id: bead.id.clone(),
                title: bead.title.clone(),
                status: bead.status,
                priority: bead.priority,
                created_at,
                updated_at,
                days_open,
                dependencies: bead.dependencies.clone(),
            });
        }

        // Sort by created_at descending (most recent first)
        self.beads.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Calculate date range
        if let (Some(first), Some(last)) = (self.beads.last(), self.beads.first()) {
            self.earliest_date = Some(first.created_at.date_naive());
            self.latest_date = Some(last.created_at.date_naive());
        }

        // Reset selection if out of bounds
        if self.beads.is_empty() {
            self.list_state.select(None);
        } else if self.list_state.selected().unwrap_or(0) >= self.beads.len() {
            self.list_state.select(Some(self.beads.len() - 1));
        }
    }

    /// Move selection down
    pub fn next(&mut self) {
        if self.beads.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let next = if current >= self.beads.len().saturating_sub(1) {
            0
        } else {
            current + 1
        };
        self.list_state.select(Some(next));
    }

    /// Move selection up
    pub fn previous(&mut self) {
        if self.beads.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let prev = if current == 0 {
            self.beads.len().saturating_sub(1)
        } else {
            current - 1
        };
        self.list_state.select(Some(prev));
    }

    /// Toggle detail view
    pub fn toggle_detail(&mut self) {
        self.show_detail = !self.show_detail;
    }

    /// Close detail view
    pub fn close_detail(&mut self) {
        self.show_detail = false;
    }

    /// Check if showing detail
    pub fn is_showing_detail(&self) -> bool {
        self.show_detail
    }

    /// Get selected bead
    pub fn selected_bead(&self) -> Option<&TimelineBead> {
        self.list_state
            .selected()
            .and_then(|i| self.beads.get(i))
    }

    /// Increase timeline range
    pub fn zoom_out(&mut self) {
        self.range_days = (self.range_days + 15).min(365);
    }

    /// Decrease timeline range
    pub fn zoom_in(&mut self) {
        self.range_days = (self.range_days - 15).max(7);
    }
}

impl Default for TimelineView {
    fn default() -> Self {
        Self::new()
    }
}

/// Draw the timeline view
pub fn draw(f: &mut Frame, view: &mut TimelineView, area: Rect) {
    if view.show_detail {
        draw_detail_view(f, view, area);
    } else {
        draw_timeline_view(f, view, area);
    }
}

fn draw_timeline_view(f: &mut Frame, view: &mut TimelineView, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Timeline list
            Constraint::Length(3), // Help
        ])
        .split(area);

    // Title with date range
    let title_text = if let (Some(earliest), Some(latest)) = (view.earliest_date, view.latest_date) {
        format!(
            "Timeline ({} beads) - {} to {}",
            view.beads.len(),
            earliest.format("%Y-%m-%d"),
            latest.format("%Y-%m-%d")
        )
    } else {
        format!("Timeline ({} beads)", view.beads.len())
    };

    let title = Paragraph::new(title_text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, chunks[0]);

    // Timeline list
    let items: Vec<ListItem> = view
        .beads
        .iter()
        .map(|bead| create_timeline_item(bead))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, chunks[1], &mut view.list_state);

    // Help text
    let help = Paragraph::new("j/k: navigate | Enter: details | +/-: zoom | Tab: switch view | q: quit")
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(help, chunks[2]);
}

fn create_timeline_item(bead: &TimelineBead) -> ListItem<'static> {
    let status_color = match bead.status {
        Status::Open => Color::White,
        Status::InProgress => Color::Yellow,
        Status::Blocked => Color::Red,
        Status::Closed => Color::Green,
        Status::Deferred | Status::Tombstone => Color::DarkGray,
    };

    let priority_color = match bead.priority {
        Priority::P0 => Color::Red,
        Priority::P1 => Color::LightRed,
        Priority::P2 => Color::Yellow,
        Priority::P3 => Color::Gray,
        Priority::P4 => Color::DarkGray,
    };

    // Create a simple ASCII timeline bar
    let bar_width = (bead.days_open as usize).min(30);
    let bar = "█".repeat(bar_width.max(1));

    let date_str = bead.created_at.format("%m/%d").to_string();

    let line = Line::from(vec![
        Span::styled(
            format!("{:>5} ", date_str),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            format!("{:>3} ", format!("P{}", bead.priority as u8)),
            Style::default().fg(priority_color),
        ),
        Span::styled(bar, Style::default().fg(status_color)),
        Span::styled(
            format!(" {}d ", bead.days_open),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            truncate_string(&bead.title, 40),
            Style::default().fg(Color::White),
        ),
    ]);

    ListItem::new(line)
}

fn draw_detail_view(f: &mut Frame, view: &TimelineView, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Help
        ])
        .split(area);

    let title = Paragraph::new("Bead Timeline Detail")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, chunks[0]);

    if let Some(bead) = view.selected_bead() {
        let status_str = match bead.status {
            Status::Open => "Open",
            Status::InProgress => "In Progress",
            Status::Blocked => "Blocked",
            Status::Closed => "Closed",
            Status::Deferred => "Deferred",
            Status::Tombstone => "Tombstone",
        };

        let dep_str = if bead.dependencies.is_empty() {
            "None".to_string()
        } else {
            bead.dependencies
                .iter()
                .map(|d| d.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };

        let content = vec![
            Line::from(vec![
                Span::styled("ID: ", Style::default().fg(Color::DarkGray)),
                Span::styled(bead.id.as_str().to_string(), Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("Title: ", Style::default().fg(Color::DarkGray)),
                Span::styled(bead.title.clone(), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
                Span::styled(status_str.to_string(), Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled("Priority: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("P{}", bead.priority as u8), Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Created: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    bead.created_at.format("%Y-%m-%d %H:%M").to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("Updated: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    bead.updated_at.format("%Y-%m-%d %H:%M").to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("Days Open: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{}", bead.days_open), Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Dependencies: ", Style::default().fg(Color::DarkGray)),
                Span::styled(dep_str, Style::default().fg(Color::White)),
            ]),
        ];

        let paragraph = Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(paragraph, chunks[1]);
    } else {
        let empty = Paragraph::new("No bead selected").block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(empty, chunks[1]);
    }

    let help = Paragraph::new("Esc: back | Tab: switch view | q: quit")
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(help, chunks[2]);
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
