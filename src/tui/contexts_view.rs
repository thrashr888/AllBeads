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

/// Sort mode for contexts list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    /// Sort by onboarding stage (default)
    ByStage,
    /// Sort alphabetically by name
    ByName,
    /// Sort by issue count (descending)
    ByIssueCount,
    /// Sort by progress percentage (ascending)
    ByProgress,
}

impl SortMode {
    /// Get display name for this sort mode
    pub fn name(&self) -> &'static str {
        match self {
            SortMode::ByStage => "Stage",
            SortMode::ByName => "Name",
            SortMode::ByIssueCount => "Issues",
            SortMode::ByProgress => "Progress",
        }
    }

    /// Get next sort mode (cycle through)
    pub fn next(&self) -> SortMode {
        match self {
            SortMode::ByStage => SortMode::ByName,
            SortMode::ByName => SortMode::ByIssueCount,
            SortMode::ByIssueCount => SortMode::ByProgress,
            SortMode::ByProgress => SortMode::ByStage,
        }
    }
}

/// Contexts view state
pub struct ContextsView {
    /// Onboarding report
    pub report: Option<OnboardingReport>,
    /// Selected context index
    pub list_state: ListState,
    /// Current sort mode
    pub sort_mode: SortMode,
    /// Whether to show detail view
    pub show_detail: bool,
    /// Whether a refresh is needed (deferred loading)
    pub needs_refresh: bool,
    /// Current organization filter (None = show all)
    pub org_filter: Option<String>,
    /// Available organizations (extracted from contexts)
    pub available_orgs: Vec<String>,
    /// Full config (cached for filtering)
    pub config: Option<AllBeadsConfig>,
}

impl ContextsView {
    /// Create a new contexts view
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            report: None,
            list_state,
            sort_mode: SortMode::ByName,
            show_detail: false,
            needs_refresh: false,
            org_filter: None,
            available_orgs: Vec::new(),
            config: None,
        }
    }

    /// Request a refresh (deferred loading)
    pub fn request_refresh(&mut self) {
        self.needs_refresh = true;
    }

    /// Refresh the contexts data
    pub fn refresh(&mut self, config: &AllBeadsConfig) {
        // Cache config for filtering
        self.config = Some(config.clone());

        // Extract available organizations
        self.available_orgs = Self::extract_organizations(&config.contexts);

        // Filter contexts by organization if filter is active
        let filtered_contexts: Vec<_> = if let Some(ref org) = self.org_filter {
            config
                .contexts
                .iter()
                .filter(|ctx| {
                    ctx.organization()
                        .as_ref()
                        .map(|o| o == org)
                        .unwrap_or(false)
                })
                .cloned()
                .collect()
        } else {
            config.contexts.clone()
        };

        if let Ok(mut report) = OnboardingReport::from_contexts(&filtered_contexts) {
            // Apply current sort mode
            Self::apply_sort(self.sort_mode, &mut report);
            self.report = Some(report);
            self.needs_refresh = false;
        }
    }

    /// Apply current sort mode to the report
    fn apply_sort(sort_mode: SortMode, report: &mut OnboardingReport) {
        match sort_mode {
            SortMode::ByStage => {
                // Sort by stage (least to most advanced) then by name
                report.statuses.sort_by(|a, b| {
                    a.stage
                        .cmp(&b.stage)
                        .then_with(|| a.context_name.cmp(&b.context_name))
                });
            }
            SortMode::ByName => {
                // Sort alphabetically by name
                report
                    .statuses
                    .sort_by(|a, b| a.context_name.cmp(&b.context_name));
            }
            SortMode::ByIssueCount => {
                // Sort by issue count (descending, with None at end)
                report
                    .statuses
                    .sort_by(|a, b| match (a.issue_count, b.issue_count) {
                        (Some(a_count), Some(b_count)) => b_count.cmp(&a_count),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.context_name.cmp(&b.context_name),
                    });
            }
            SortMode::ByProgress => {
                // Sort by progress percentage (ascending)
                report.statuses.sort_by(|a, b| {
                    a.stage
                        .progress()
                        .cmp(&b.stage.progress())
                        .then_with(|| a.context_name.cmp(&b.context_name))
                });
            }
        }
    }

    /// Cycle to next sort mode
    pub fn cycle_sort(&mut self) {
        self.sort_mode = self.sort_mode.next();
        // Re-sort if we have data
        if let Some(ref mut report) = self.report {
            Self::apply_sort(self.sort_mode, report);
        }
        // Reset selection to top
        self.list_state.select(Some(0));
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

    /// Toggle detail view
    pub fn toggle_detail(&mut self) {
        self.show_detail = !self.show_detail;
    }

    /// Close detail view
    pub fn close_detail(&mut self) {
        self.show_detail = false;
    }

    /// Extract unique organizations from contexts
    fn extract_organizations(contexts: &[crate::config::BossContext]) -> Vec<String> {
        let mut orgs: Vec<String> = contexts
            .iter()
            .filter_map(|ctx| ctx.organization())
            .collect();
        orgs.sort();
        orgs.dedup();
        orgs
    }

    /// Cycle to next organization filter
    /// Cycles through: All → Org1 → Org2 → ... → All
    pub fn cycle_org_filter(&mut self) {
        if self.available_orgs.is_empty() {
            // No orgs available, stay at All
            return;
        }

        self.org_filter = match &self.org_filter {
            None => {
                // Currently showing All, switch to first org
                self.available_orgs.first().cloned()
            }
            Some(current_org) => {
                // Find current org index and move to next
                if let Some(idx) = self.available_orgs.iter().position(|o| o == current_org) {
                    if idx + 1 < self.available_orgs.len() {
                        // Move to next org
                        self.available_orgs.get(idx + 1).cloned()
                    } else {
                        // Wrap back to All
                        None
                    }
                } else {
                    // Current org not found, reset to All
                    None
                }
            }
        };

        // Reapply filter by refreshing from cached config
        if let Some(ref config) = self.config.clone() {
            self.refresh(config);
        }

        // Reset selection to top
        self.list_state.select(Some(0));
    }
}

impl Default for ContextsView {
    fn default() -> Self {
        Self::new()
    }
}

/// Draw the contexts view
pub fn draw(f: &mut Frame, contexts_view: &mut ContextsView, area: Rect) {
    if contexts_view.show_detail {
        // Detail view layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(1),  // Column header
                Constraint::Min(10),    // Contexts list
                Constraint::Length(10), // Selected context details
                Constraint::Length(3),  // Help
            ])
            .split(area);

        // Title with summary
        draw_title(f, contexts_view, chunks[0]);

        // Column header
        draw_column_header(f, chunks[1]);

        // Contexts list
        draw_contexts_list(f, contexts_view, chunks[2]);

        // Selected context details
        draw_context_details(f, contexts_view, chunks[3]);

        // Help text
        let help = Paragraph::new("↑↓: navigate | s: sort | o: org filter | r: refresh | Enter: details | Esc: close | Tab: switch | q: quit")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::TOP));
        f.render_widget(help, chunks[4]);
    } else {
        // List view layout (no details)
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(1), // Column header
                Constraint::Min(10),   // Contexts list
                Constraint::Length(3), // Help
            ])
            .split(area);

        // Title with summary
        draw_title(f, contexts_view, chunks[0]);

        // Column header
        draw_column_header(f, chunks[1]);

        // Contexts list
        draw_contexts_list(f, contexts_view, chunks[2]);

        // Help text
        let help = Paragraph::new(
            "↑↓: navigate | s: sort | o: org filter | r: refresh | Enter: details | Tab: switch | q: quit",
        )
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP));
        f.render_widget(help, chunks[3]);
    }
}

fn draw_column_header(f: &mut Frame, area: Rect) {
    // Column headers: Issues, B=Beads, S=Skills, I=Integration, C=CI/CD, H=Hooks, Agent Tooling
    let header = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<18} ", "Name"),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(format!("{:>5} ", "#"), Style::default().fg(Color::DarkGray)),
        Span::styled(
            "B",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            "S",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            "I",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            "C",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            "H",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("MCP", Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::styled("Cur", Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::styled("Cop", Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::styled("Agt", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("Progress", Style::default().fg(Color::DarkGray)),
    ]);

    let paragraph = Paragraph::new(header);
    f.render_widget(paragraph, area);
}

fn draw_title(f: &mut Frame, contexts_view: &ContextsView, area: Rect) {
    let (left_text, right_text) = if let Some(ref report) = contexts_view.report {
        let left = format!(
            "Contexts - {}/{} Fully Onboarded ({}%)",
            report.stats.fully_onboarded,
            report.stats.total_contexts,
            if report.stats.total_contexts > 0 {
                (report.stats.fully_onboarded * 100) / report.stats.total_contexts
            } else {
                0
            }
        );

        // Show org filter and sort mode
        let org_text = if let Some(ref org) = contexts_view.org_filter {
            format!("Org: {} | ", org)
        } else {
            "".to_string()
        };
        let right = format!("{}Sort: {}", org_text, contexts_view.sort_mode.name());
        (left, right)
    } else {
        ("Contexts - Loading...".to_string(), String::new())
    };

    // Calculate spacing to right-align the sort info
    let available_width = area.width.saturating_sub(2) as usize; // Subtract borders
    let left_len = left_text.len();
    let right_len = right_text.len();
    let spacing = if left_len + right_len + 3 < available_width {
        " ".repeat(available_width.saturating_sub(left_len + right_len))
    } else {
        " ".to_string()
    };

    let title_line = Line::from(vec![
        Span::styled(
            left_text,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(spacing),
        Span::styled(right_text, Style::default().fg(Color::DarkGray)),
    ]);

    let title = Paragraph::new(title_line).block(Block::default().borders(Borders::BOTTOM));
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

                // Status indicators (checkmarks only, header shows labels)
                let beads_char = if status.has_beads_usage() {
                    "✓"
                } else {
                    "✗"
                };
                let skills_char = if status.has_skills { "✓" } else { "✗" };
                let integration_char = if status.has_integration { "✓" } else { "✗" };
                let ci_char = if status.has_ci { "✓" } else { "✗" };
                let hooks_char = if status.has_hooks { "✓" } else { "✗" };

                // Truncate name to fit column width
                let display_name = if status.context_name.len() > 16 {
                    format!("{}...", &status.context_name[..13])
                } else {
                    status.context_name.clone()
                };

                // Issue count display
                let issue_str = match status.issue_count {
                    Some(count) => format!("{:>5}", count),
                    None => "    -".to_string(),
                };

                // Agent tooling indicators
                let mcp_count = status.agent_tooling.mcp_servers.len();
                let mcp_str = if mcp_count > 0 {
                    format!("{:>3}", mcp_count)
                } else {
                    "  -".to_string()
                };
                let cursor_char = if status.agent_tooling.has_cursor_rules {
                    "✓"
                } else {
                    "-"
                };
                let copilot_char = if status.agent_tooling.has_copilot_rules {
                    "✓"
                } else {
                    "-"
                };
                let agents_char = if status.agent_tooling.has_agents_md {
                    "✓"
                } else {
                    "-"
                };

                let line = Line::from(vec![
                    Span::raw(format!("{} ", emoji)),
                    Span::styled(
                        format!("{:<16} ", display_name),
                        Style::default().fg(Color::White),
                    ),
                    // Issue count
                    Span::styled(
                        format!("{} ", issue_str),
                        Style::default().fg(if status.issue_count.is_some() {
                            Color::Cyan
                        } else {
                            Color::DarkGray
                        }),
                    ),
                    // Health check status columns (no labels, see header)
                    Span::styled(
                        beads_char,
                        Style::default().fg(if status.has_beads_usage() {
                            Color::Green
                        } else {
                            Color::Red
                        }),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        skills_char,
                        Style::default().fg(if status.has_skills {
                            Color::Green
                        } else {
                            Color::Red
                        }),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        integration_char,
                        Style::default().fg(if status.has_integration {
                            Color::Green
                        } else {
                            Color::Red
                        }),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        ci_char,
                        Style::default().fg(if status.has_ci {
                            Color::Green
                        } else {
                            Color::Red
                        }),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        hooks_char,
                        Style::default().fg(if status.has_hooks {
                            Color::Green
                        } else {
                            Color::Red
                        }),
                    ),
                    Span::raw("  "),
                    // Agent tooling columns
                    Span::styled(
                        mcp_str,
                        Style::default().fg(if mcp_count > 0 {
                            Color::Magenta
                        } else {
                            Color::DarkGray
                        }),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        cursor_char,
                        Style::default().fg(if status.agent_tooling.has_cursor_rules {
                            Color::Green
                        } else {
                            Color::DarkGray
                        }),
                    ),
                    Span::raw("   "),
                    Span::styled(
                        copilot_char,
                        Style::default().fg(if status.agent_tooling.has_copilot_rules {
                            Color::Green
                        } else {
                            Color::DarkGray
                        }),
                    ),
                    Span::raw("   "),
                    Span::styled(
                        agents_char,
                        Style::default().fg(if status.agent_tooling.has_agents_md {
                            Color::Green
                        } else {
                            Color::DarkGray
                        }),
                    ),
                    Span::raw("  "),
                    // Progress bar
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
        vec![
            ListItem::new(""),
            ListItem::new(Line::from(vec![Span::styled(
                "Loading contexts...",
                Style::default().fg(Color::Yellow),
            )])),
            ListItem::new(""),
            ListItem::new(Line::from(vec![Span::styled(
                "(This may take a few seconds on first load)",
                Style::default().fg(Color::DarkGray),
            )])),
        ]
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
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(vec![
                        Span::styled("URL: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(status.url.clone(), Style::default().fg(Color::Blue)),
                    ]),
                ];

                if let Some(ref org) = status.organization {
                    lines.push(Line::from(vec![
                        Span::styled("Organization: ", Style::default().fg(Color::DarkGray)),
                        Span::styled(org.clone(), Style::default().fg(Color::Magenta)),
                    ]));
                }

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
                        Span::styled(count.to_string(), Style::default().fg(Color::Cyan)),
                    ]));
                }

                lines.push(Line::from(""));

                // Health Checks row
                lines.push(Line::from(vec![
                    Span::styled(
                        "Health Checks: ",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("Beads: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        if status.has_beads_usage() {
                            "✓"
                        } else {
                            "✗"
                        },
                        Style::default().fg(if status.has_beads_usage() {
                            Color::Green
                        } else {
                            Color::Red
                        }),
                    ),
                    Span::raw("  "),
                    Span::styled("Skills: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        if status.has_skills { "✓" } else { "✗" },
                        Style::default().fg(if status.has_skills {
                            Color::Green
                        } else {
                            Color::Red
                        }),
                    ),
                    Span::raw("  "),
                    Span::styled("Integration: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        if status.has_integration { "✓" } else { "✗" },
                        Style::default().fg(if status.has_integration {
                            Color::Green
                        } else {
                            Color::Red
                        }),
                    ),
                    Span::raw("  "),
                    Span::styled("CI/CD: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        if status.has_ci { "✓" } else { "✗" },
                        Style::default().fg(if status.has_ci {
                            Color::Green
                        } else {
                            Color::Red
                        }),
                    ),
                    Span::raw("  "),
                    Span::styled("Hooks: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        if status.has_hooks { "✓" } else { "✗" },
                        Style::default().fg(if status.has_hooks {
                            Color::Green
                        } else {
                            Color::Red
                        }),
                    ),
                ]));

                // Agent Tooling row
                let mcp_count = status.agent_tooling.mcp_servers.len();
                let mcp_text = if mcp_count > 0 {
                    if mcp_count <= 3 {
                        status.agent_tooling.mcp_servers.join(", ")
                    } else {
                        format!(
                            "{}, +{} more",
                            status.agent_tooling.mcp_servers[..2].join(", "),
                            mcp_count - 2
                        )
                    }
                } else {
                    "none".to_string()
                };

                lines.push(Line::from(vec![
                    Span::styled(
                        "Agent Tooling: ",
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled("MCP: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        mcp_text,
                        Style::default().fg(if mcp_count > 0 {
                            Color::Magenta
                        } else {
                            Color::DarkGray
                        }),
                    ),
                    Span::raw("  "),
                    Span::styled("Cursor: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        if status.agent_tooling.has_cursor_rules {
                            "✓"
                        } else {
                            "✗"
                        },
                        Style::default().fg(if status.agent_tooling.has_cursor_rules {
                            Color::Green
                        } else {
                            Color::DarkGray
                        }),
                    ),
                    Span::raw("  "),
                    Span::styled("Copilot: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        if status.agent_tooling.has_copilot_rules {
                            "✓"
                        } else {
                            "✗"
                        },
                        Style::default().fg(if status.agent_tooling.has_copilot_rules {
                            Color::Green
                        } else {
                            Color::DarkGray
                        }),
                    ),
                    Span::raw("  "),
                    Span::styled("AGENTS.md: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        if status.agent_tooling.has_agents_md {
                            "✓"
                        } else {
                            "✗"
                        },
                        Style::default().fg(if status.agent_tooling.has_agents_md {
                            Color::Green
                        } else {
                            Color::DarkGray
                        }),
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
