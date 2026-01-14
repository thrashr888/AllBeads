//! Contexts view for the TUI
//!
//! Displays all configured contexts with their onboarding status.

use crate::config::AllBeadsConfig;
use crate::onboarding::{OnboardingReport, OnboardingStage};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

/// Contexts view state
pub struct ContextsView {
    /// Onboarding report
    pub report: Option<OnboardingReport>,
    /// Selected context index
    pub list_state: ListState,
}

impl ContextsView {
    /// Create a new contexts view
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            report: None,
            list_state,
        }
    }

    /// Refresh the contexts data
    pub fn refresh(&mut self, config: &AllBeadsConfig) {
        if let Ok(report) = OnboardingReport::from_contexts(&config.contexts) {
            self.report = Some(report);
        }
    }

    /// Select next context
    pub fn next(&mut self) {
        if let Some(ref report) = self.report {
            let count = report.statuses.len();
            if count == 0 {
                return;
            }
            let current = self.list_state.selected().unwrap_or(0);
            let next = if current >= count.saturating_sub(1) {
                0
            } else {
                current + 1
            };
            self.list_state.select(Some(next));
        }
    }

    /// Select previous context
    pub fn previous(&mut self) {
        if let Some(ref report) = self.report {
            let count = report.statuses.len();
            if count == 0 {
                return;
            }
            let current = self.list_state.selected().unwrap_or(0);
            let prev = if current == 0 {
                count.saturating_sub(1)
            } else {
                current - 1
            };
            self.list_state.select(Some(prev));
        }
    }
}

impl Default for ContextsView {
    fn default() -> Self {
        Self::new()
    }
}

/// Draw the contexts view
pub fn draw(f: &mut Frame, contexts_view: &mut ContextsView, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(10),    // Contexts list
            Constraint::Length(10), // Selected context details
            Constraint::Length(3),  // Help
        ])
        .split(area);

    // Title with summary
    draw_title(f, contexts_view, chunks[0]);

    // Contexts list
    draw_contexts_list(f, contexts_view, chunks[1]);

    // Selected context details
    draw_context_details(f, contexts_view, chunks[2]);

    // Help text
    let help = Paragraph::new("↑↓: navigate | Tab: switch view | q: quit")
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(help, chunks[3]);
}

fn draw_title(f: &mut Frame, contexts_view: &ContextsView, area: Rect) {
    let title_text = if let Some(ref report) = contexts_view.report {
        format!(
            "Contexts - {}/{} Fully Onboarded ({}%)",
            report.stats.fully_onboarded,
            report.stats.total_contexts,
            if report.stats.total_contexts > 0 {
                (report.stats.fully_onboarded * 100) / report.stats.total_contexts
            } else {
                0
            }
        )
    } else {
        "Contexts - Loading...".to_string()
    };

    let title = Paragraph::new(title_text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, area);
}

fn draw_contexts_list(f: &mut Frame, contexts_view: &mut ContextsView, area: Rect) {
    let items: Vec<ListItem> = if let Some(ref report) = contexts_view.report {
        report
            .statuses
            .iter()
            .map(|status| {
                let emoji = status.stage.emoji();
                let progress = status.stage.progress();
                let bar_width = (progress / 10) as usize;
                let bar = "█".repeat(bar_width);
                let empty = "░".repeat(10 - bar_width);

                let color = match status.stage {
                    OnboardingStage::NotCloned => Color::DarkGray,
                    OnboardingStage::Cloned => Color::Gray,
                    OnboardingStage::BeadsInitialized => Color::Blue,
                    OnboardingStage::HasIssues => Color::Yellow,
                    OnboardingStage::HasSkills => Color::Magenta,
                    OnboardingStage::IntegrationConfigured => Color::Cyan,
                    OnboardingStage::FullyOnboarded => Color::Green,
                };

                let line = Line::from(vec![
                    Span::raw(format!("{} ", emoji)),
                    Span::styled(
                        format!("{:<20} ", status.context_name),
                        Style::default().fg(Color::White),
                    ),
                    Span::styled(bar, Style::default().fg(color)),
                    Span::styled(empty, Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!(" {}%", progress),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]);

                ListItem::new(line)
            })
            .collect()
    } else {
        vec![ListItem::new("Loading contexts...")]
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Repository Onboarding Status ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut contexts_view.list_state);
}

fn draw_context_details(f: &mut Frame, contexts_view: &ContextsView, area: Rect) {
    let details_lines: Vec<Line> = if let Some(ref report) = contexts_view.report {
        if let Some(selected_idx) = contexts_view.list_state.selected() {
            if let Some(status) = report.statuses.get(selected_idx) {
                let mut lines = vec![
                    Line::from(vec![
                        Span::styled("Name: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            status.context_name.clone(),
                            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("URL: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            status.url.clone(),
                            Style::default().fg(Color::Blue),
                        ),
                    ]),
                ];

                if let Some(ref path) = status.path {
                    lines.push(Line::from(vec![
                        Span::styled("Path: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            path.display().to_string(),
                            Style::default().fg(Color::Yellow),
                        ),
                    ]));
                }

                if let Some(count) = status.issue_count {
                    lines.push(Line::from(vec![
                        Span::styled("Issues: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            count.to_string(),
                            Style::default().fg(Color::Cyan),
                        ),
                    ]));
                }

                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Skills: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        if status.has_skills { "✓" } else { "✗" },
                        Style::default().fg(if status.has_skills { Color::Green } else { Color::Red }),
                    ),
                    Span::raw("  "),
                    Span::styled("Integration: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        if status.has_integration { "✓" } else { "✗" },
                        Style::default().fg(if status.has_integration { Color::Green } else { Color::Red }),
                    ),
                    Span::raw("  "),
                    Span::styled("CI/CD: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        if status.has_ci { "✓" } else { "✗" },
                        Style::default().fg(if status.has_ci { Color::Green } else { Color::Red }),
                    ),
                ]));

                lines
            } else {
                vec![Line::from("No context selected")]
            }
        } else {
            vec![Line::from("No context selected")]
        }
    } else {
        vec![Line::from("Loading...")]
    };

    let paragraph = Paragraph::new(details_lines).block(
        Block::default()
            .title(" Details ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    f.render_widget(paragraph, area);
}
