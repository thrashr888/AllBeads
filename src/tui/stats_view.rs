//! Stats view for the TUI
//!
//! Displays aggregate metrics and statistics about beads.

use crate::graph::{FederatedGraph, Priority, Status};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Paragraph},
    Frame,
};
use std::collections::HashMap;

/// Stats view state
pub struct StatsView {
    /// Total beads count
    pub total: usize,
    /// Beads by status (open, in_progress, blocked, closed)
    pub status_open: usize,
    pub status_in_progress: usize,
    pub status_blocked: usize,
    pub status_closed: usize,
    /// Beads by priority (P0-P4)
    pub priority_p0: usize,
    pub priority_p1: usize,
    pub priority_p2: usize,
    pub priority_p3: usize,
    pub priority_p4: usize,
    /// Beads by context
    pub by_context: HashMap<String, usize>,
    /// Ready beads (no blockers)
    pub ready_count: usize,
}

impl StatsView {
    /// Create a new stats view
    pub fn new() -> Self {
        Self {
            total: 0,
            status_open: 0,
            status_in_progress: 0,
            status_blocked: 0,
            status_closed: 0,
            priority_p0: 0,
            priority_p1: 0,
            priority_p2: 0,
            priority_p3: 0,
            priority_p4: 0,
            by_context: HashMap::new(),
            ready_count: 0,
        }
    }

    /// Analyze the graph and update stats
    pub fn analyze(&mut self, graph: &FederatedGraph) {
        self.total = graph.beads.len();
        self.status_open = 0;
        self.status_in_progress = 0;
        self.status_blocked = 0;
        self.status_closed = 0;
        self.priority_p0 = 0;
        self.priority_p1 = 0;
        self.priority_p2 = 0;
        self.priority_p3 = 0;
        self.priority_p4 = 0;
        self.by_context.clear();

        for bead in graph.beads.values() {
            // Count by status
            match bead.status {
                Status::Open => self.status_open += 1,
                Status::InProgress => self.status_in_progress += 1,
                Status::Blocked => self.status_blocked += 1,
                Status::Closed => self.status_closed += 1,
                Status::Deferred | Status::Tombstone => {} // Skip these
            }

            // Count by priority
            match bead.priority {
                Priority::P0 => self.priority_p0 += 1,
                Priority::P1 => self.priority_p1 += 1,
                Priority::P2 => self.priority_p2 += 1,
                Priority::P3 => self.priority_p3 += 1,
                Priority::P4 => self.priority_p4 += 1,
            }

            // Count by context (from @labels)
            for label in &bead.labels {
                if let Some(ctx) = label.strip_prefix('@') {
                    *self.by_context.entry(ctx.to_string()).or_insert(0) += 1;
                }
            }
        }

        // Count ready
        self.ready_count = graph.ready_beads().len();
    }
}

impl Default for StatsView {
    fn default() -> Self {
        Self::new()
    }
}

/// Draw the stats view
pub fn draw(f: &mut Frame, stats_view: &StatsView, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(8),  // Status section
            Constraint::Length(10), // Priority section
            Constraint::Min(8),     // Context section
            Constraint::Length(3),  // Help
        ])
        .split(area);

    // Title
    let title = Paragraph::new(format!(
        "Project Statistics - {} Total Beads",
        stats_view.total
    ))
    .style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, chunks[0]);

    // Status breakdown
    draw_status_section(f, stats_view, chunks[1]);

    // Priority breakdown
    draw_priority_section(f, stats_view, chunks[2]);

    // Context breakdown
    draw_context_section(f, stats_view, chunks[3]);

    // Help text
    let help = Paragraph::new("Tab: switch view | q: quit")
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(help, chunks[4]);
}

fn draw_status_section(f: &mut Frame, stats: &StatsView, area: Rect) {
    let data = [
        ("Open", stats.status_open as u64, Color::White),
        (
            "In Progress",
            stats.status_in_progress as u64,
            Color::Yellow,
        ),
        ("Blocked", stats.status_blocked as u64, Color::Red),
        ("Closed", stats.status_closed as u64, Color::Green),
    ];

    let bars: Vec<Bar> = data
        .iter()
        .map(|(label, value, color)| {
            Bar::default()
                .value(*value)
                .label(Line::from(format!("{}: {}", label, value)))
                .style(Style::default().fg(*color))
        })
        .collect();

    let bar_chart = BarChart::default()
        .block(
            Block::default()
                .title(" Status ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(12)
        .bar_gap(2)
        .bar_style(Style::default().fg(Color::Cyan))
        .value_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(bar_chart, area);
}

fn draw_priority_section(f: &mut Frame, stats: &StatsView, area: Rect) {
    let data = [
        ("P0", stats.priority_p0 as u64, Color::Red),
        ("P1", stats.priority_p1 as u64, Color::LightRed),
        ("P2", stats.priority_p2 as u64, Color::Yellow),
        ("P3", stats.priority_p3 as u64, Color::Gray),
        ("P4", stats.priority_p4 as u64, Color::DarkGray),
    ];

    let bars: Vec<Bar> = data
        .iter()
        .map(|(label, value, color)| {
            Bar::default()
                .value(*value)
                .label(Line::from(format!("{}: {}", label, value)))
                .style(Style::default().fg(*color))
        })
        .collect();

    let bar_chart = BarChart::default()
        .block(
            Block::default()
                .title(" Priority Distribution ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .data(BarGroup::default().bars(&bars))
        .bar_width(8)
        .bar_gap(1)
        .bar_style(Style::default().fg(Color::Cyan))
        .value_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(bar_chart, area);
}

fn draw_context_section(f: &mut Frame, stats: &StatsView, area: Rect) {
    // Sort contexts by count descending
    let mut contexts: Vec<_> = stats.by_context.iter().collect();
    contexts.sort_by(|a, b| b.1.cmp(a.1));

    // Create text lines with simple bar visualization
    let lines: Vec<Line> = contexts
        .iter()
        .take(10) // Show top 10 contexts
        .enumerate()
        .map(|(i, (name, count))| {
            let max_count = contexts.first().map(|(_, c)| **c).unwrap_or(1);
            let bar_width = if max_count > 0 {
                (*count * 30) / max_count
            } else {
                0
            };
            let bar = "█".repeat(bar_width.max(1));

            let color = match i {
                0 => Color::Cyan,
                1 => Color::Blue,
                2 => Color::Magenta,
                _ => Color::DarkGray,
            };

            Line::from(vec![
                Span::styled(format!("{:>12} ", name), Style::default().fg(Color::White)),
                Span::styled(bar, Style::default().fg(color)),
                Span::styled(format!(" {}", count), Style::default().fg(Color::Yellow)),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title(" Beads by Context ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(paragraph, area);
}
