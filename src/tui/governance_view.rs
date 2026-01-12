//! Governance view for the TUI
//!
//! Displays policy status, check results, and audit information.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

/// A policy check result
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub timestamp: String,
}

/// A policy definition
#[derive(Debug, Clone)]
pub struct Policy {
    pub name: String,
    pub enabled: bool,
    pub description: String,
    pub last_run: Option<String>,
}

/// Governance view state
pub struct GovernanceView {
    /// Active policies
    policies: Vec<Policy>,
    /// Recent check results
    check_results: Vec<CheckResult>,
    /// List state for selection
    list_state: ListState,
    /// Show detail view
    show_detail: bool,
    /// Current section (0 = policies, 1 = results)
    current_section: usize,
}

impl GovernanceView {
    /// Create a new governance view
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            policies: Vec::new(),
            check_results: Vec::new(),
            list_state,
            show_detail: false,
            current_section: 0,
        }
    }

    /// Load sample/placeholder data for now
    /// This will be replaced with actual Sheriff governance data
    pub fn load_placeholder_data(&mut self) {
        self.policies = vec![
            Policy {
                name: "require-description".to_string(),
                enabled: true,
                description: "All beads must have a description".to_string(),
                last_run: Some("2024-01-12 10:30".to_string()),
            },
            Policy {
                name: "max-in-progress".to_string(),
                enabled: true,
                description: "Limit concurrent in-progress beads per assignee".to_string(),
                last_run: Some("2024-01-12 10:30".to_string()),
            },
            Policy {
                name: "require-labels".to_string(),
                enabled: false,
                description: "All beads must have at least one label".to_string(),
                last_run: None,
            },
            Policy {
                name: "dependency-cycle-check".to_string(),
                enabled: true,
                description: "Detect and prevent circular dependencies".to_string(),
                last_run: Some("2024-01-12 10:30".to_string()),
            },
        ];

        self.check_results = vec![
            CheckResult {
                name: "require-description".to_string(),
                passed: true,
                message: "All 15 beads have descriptions".to_string(),
                timestamp: "2024-01-12 10:30".to_string(),
            },
            CheckResult {
                name: "max-in-progress".to_string(),
                passed: false,
                message: "user@example.com has 5 in-progress (max: 3)".to_string(),
                timestamp: "2024-01-12 10:30".to_string(),
            },
            CheckResult {
                name: "dependency-cycle-check".to_string(),
                passed: true,
                message: "No circular dependencies detected".to_string(),
                timestamp: "2024-01-12 10:30".to_string(),
            },
        ];
    }

    /// Get policy count
    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    /// Get enabled policy count
    pub fn enabled_policy_count(&self) -> usize {
        self.policies.iter().filter(|p| p.enabled).count()
    }

    /// Get passing check count
    pub fn passing_check_count(&self) -> usize {
        self.check_results.iter().filter(|r| r.passed).count()
    }

    /// Get failing check count
    pub fn failing_check_count(&self) -> usize {
        self.check_results.iter().filter(|r| !r.passed).count()
    }

    /// Move selection down
    pub fn next(&mut self) {
        let items = if self.current_section == 0 {
            &self.policies
        } else {
            // Use check_results length for section 1
            let len = self.check_results.len();
            if len == 0 {
                return;
            }
            let current = self.list_state.selected().unwrap_or(0);
            let next = if current >= len.saturating_sub(1) {
                0
            } else {
                current + 1
            };
            self.list_state.select(Some(next));
            return;
        };

        if items.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let next = if current >= items.len().saturating_sub(1) {
            0
        } else {
            current + 1
        };
        self.list_state.select(Some(next));
    }

    /// Move selection up
    pub fn previous(&mut self) {
        let len = if self.current_section == 0 {
            self.policies.len()
        } else {
            self.check_results.len()
        };

        if len == 0 {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let prev = if current == 0 {
            len.saturating_sub(1)
        } else {
            current - 1
        };
        self.list_state.select(Some(prev));
    }

    /// Switch between sections
    pub fn next_section(&mut self) {
        self.current_section = (self.current_section + 1) % 2;
        self.list_state.select(Some(0));
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

    /// Get selected policy (if in policies section)
    pub fn selected_policy(&self) -> Option<&Policy> {
        if self.current_section == 0 {
            self.list_state
                .selected()
                .and_then(|i| self.policies.get(i))
        } else {
            None
        }
    }

    /// Get selected check result (if in results section)
    pub fn selected_result(&self) -> Option<&CheckResult> {
        if self.current_section == 1 {
            self.list_state
                .selected()
                .and_then(|i| self.check_results.get(i))
        } else {
            None
        }
    }
}

impl Default for GovernanceView {
    fn default() -> Self {
        Self::new()
    }
}

/// Draw the governance view
pub fn draw(f: &mut Frame, view: &mut GovernanceView, area: Rect) {
    if view.show_detail {
        draw_detail_view(f, view, area);
    } else {
        draw_main_view(f, view, area);
    }
}

fn draw_main_view(f: &mut Frame, view: &mut GovernanceView, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Help
        ])
        .split(area);

    // Title with summary
    let passing = view.passing_check_count();
    let failing = view.failing_check_count();
    let enabled = view.enabled_policy_count();
    let total = view.policy_count();

    let title_text = format!(
        "Governance - {} policies ({} enabled) | Checks: {} passing, {} failing",
        total, enabled, passing, failing
    );

    let title = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, chunks[0]);

    // Split content into two panels
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // Policies panel
    draw_policies_panel(f, view, content_chunks[0]);

    // Check results panel
    draw_results_panel(f, view, content_chunks[1]);

    // Help text
    let help = Paragraph::new(
        "j/k: navigate | Tab: switch section | Enter: details | Tab(global): switch view | q: quit",
    )
    .style(Style::default().fg(Color::DarkGray))
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(help, chunks[2]);
}

fn draw_policies_panel(f: &mut Frame, view: &mut GovernanceView, area: Rect) {
    let is_selected = view.current_section == 0;
    let border_style = if is_selected {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = view
        .policies
        .iter()
        .map(|policy| {
            let status_icon = if policy.enabled { "●" } else { "○" };
            let status_color = if policy.enabled {
                Color::Green
            } else {
                Color::DarkGray
            };

            let line = Line::from(vec![
                Span::styled(format!("{} ", status_icon), Style::default().fg(status_color)),
                Span::styled(
                    policy.name.clone(),
                    Style::default().fg(Color::White),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Policies ")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    if is_selected {
        f.render_stateful_widget(list, area, &mut view.list_state);
    } else {
        f.render_widget(list, area);
    }
}

fn draw_results_panel(f: &mut Frame, view: &mut GovernanceView, area: Rect) {
    let is_selected = view.current_section == 1;
    let border_style = if is_selected {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = view
        .check_results
        .iter()
        .map(|result| {
            let status_icon = if result.passed { "✓" } else { "✗" };
            let status_color = if result.passed {
                Color::Green
            } else {
                Color::Red
            };

            let line = Line::from(vec![
                Span::styled(format!("{} ", status_icon), Style::default().fg(status_color)),
                Span::styled(result.name.clone(), Style::default().fg(Color::White)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Check Results ")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    if is_selected {
        f.render_stateful_widget(list, area, &mut view.list_state);
    } else {
        f.render_widget(list, area);
    }
}

fn draw_detail_view(f: &mut Frame, view: &GovernanceView, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Help
        ])
        .split(area);

    let title = Paragraph::new("Governance Detail")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, chunks[0]);

    // Show detail based on current section
    if view.current_section == 0 {
        if let Some(policy) = view.selected_policy() {
            let content = vec![
                Line::from(vec![
                    Span::styled("Name: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(policy.name.clone(), Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        if policy.enabled { "Enabled" } else { "Disabled" }.to_string(),
                        Style::default().fg(if policy.enabled {
                            Color::Green
                        } else {
                            Color::DarkGray
                        }),
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Description: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(policy.description.clone(), Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Last Run: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        policy.last_run.clone().unwrap_or_else(|| "Never".to_string()),
                        Style::default().fg(Color::White),
                    ),
                ]),
            ];

            let paragraph = Paragraph::new(content).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
            f.render_widget(paragraph, chunks[1]);
        }
    } else if let Some(result) = view.selected_result() {
        let content = vec![
            Line::from(vec![
                Span::styled("Check: ", Style::default().fg(Color::DarkGray)),
                Span::styled(result.name.clone(), Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("Result: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    if result.passed { "PASSED" } else { "FAILED" }.to_string(),
                    Style::default().fg(if result.passed {
                        Color::Green
                    } else {
                        Color::Red
                    }),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Message: ", Style::default().fg(Color::DarkGray)),
                Span::styled(result.message.clone(), Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Timestamp: ", Style::default().fg(Color::DarkGray)),
                Span::styled(result.timestamp.clone(), Style::default().fg(Color::White)),
            ]),
        ];

        let paragraph = Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(paragraph, chunks[1]);
    } else {
        let empty = Paragraph::new("No item selected").block(
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
