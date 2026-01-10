//! Swarm agent view for the TUI
//!
//! Displays running agents with status, cost, and actions.

use crate::swarm::{AgentManager, AgentStatus, ManagerStats};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Row, Table, Wrap},
    Frame,
};
use std::sync::Arc;

/// Swarm view state
pub struct SwarmView {
    /// Agent manager reference
    manager: Option<Arc<AgentManager>>,
    /// Cached list of agents for display
    agents: Vec<AgentSnapshot>,
    /// Selected agent index
    list_state: ListState,
    /// Currently viewing agent details
    show_detail: bool,
    /// Stats cache
    stats: ManagerStats,
}

/// Snapshot of agent state for display
#[derive(Clone)]
pub struct AgentSnapshot {
    pub id: String,
    pub name: String,
    pub persona: String,
    pub status: AgentStatus,
    pub status_message: String,
    pub context: String,
    pub rig: Option<String>,
    pub bead_id: Option<String>,
    pub locked_files: Vec<String>,
    pub cost_usd: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub api_calls: u32,
    pub runtime: String,
}

impl SwarmView {
    /// Create a new swarm view
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            manager: None,
            agents: Vec::new(),
            list_state,
            show_detail: false,
            stats: ManagerStats::default(),
        }
    }

    /// Set the agent manager
    pub fn set_manager(&mut self, manager: Arc<AgentManager>) {
        self.manager = Some(manager);
        self.refresh();
    }

    /// Check if swarm is available
    pub fn has_manager(&self) -> bool {
        self.manager.is_some()
    }

    /// Refresh agent list from manager
    pub fn refresh(&mut self) {
        if let Some(ref manager) = self.manager {
            self.stats = manager.stats();

            // Get all agents and convert to snapshots
            self.agents = manager
                .list()
                .iter()
                .map(|a| AgentSnapshot {
                    id: a.id.clone(),
                    name: a.name.clone(),
                    persona: a.persona.to_string(),
                    status: a.status,
                    status_message: a.status_message.clone(),
                    context: a.context.clone(),
                    rig: a.rig.clone(),
                    bead_id: a.bead_id.clone(),
                    locked_files: a
                        .locked_files
                        .iter()
                        .filter_map(|p| p.file_name())
                        .map(|n| n.to_string_lossy().to_string())
                        .collect(),
                    cost_usd: a.cost.total_usd,
                    input_tokens: a.cost.input_tokens,
                    output_tokens: a.cost.output_tokens,
                    api_calls: a.cost.api_calls,
                    runtime: a.format_runtime(),
                })
                .collect();

            // Sort by status (active first) then by name
            self.agents.sort_by(|a, b| {
                let a_active = a.status.is_active();
                let b_active = b.status.is_active();
                b_active.cmp(&a_active).then_with(|| a.name.cmp(&b.name))
            });

            // Reset selection if out of bounds
            if self.agents.is_empty() {
                self.list_state.select(None);
            } else if self.list_state.selected().unwrap_or(0) >= self.agents.len() {
                self.list_state.select(Some(self.agents.len() - 1));
            }
        }
    }

    /// Get active agent count
    pub fn active_count(&self) -> usize {
        self.stats.active_agents
    }

    /// Get total cost
    pub fn total_cost(&self) -> f64 {
        self.stats.total_cost
    }

    /// Move selection down
    pub fn next(&mut self) {
        if self.agents.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let next = if current >= self.agents.len().saturating_sub(1) {
            0
        } else {
            current + 1
        };
        self.list_state.select(Some(next));
    }

    /// Move selection up
    pub fn previous(&mut self) {
        if self.agents.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let prev = if current == 0 {
            self.agents.len().saturating_sub(1)
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

    /// Get selected agent
    pub fn selected_agent(&self) -> Option<&AgentSnapshot> {
        self.list_state
            .selected()
            .and_then(|i| self.agents.get(i))
    }

    /// Get selected agent ID
    pub fn selected_agent_id(&self) -> Option<&str> {
        self.selected_agent().map(|a| a.id.as_str())
    }

    /// Kill selected agent
    pub fn kill_selected(&mut self) {
        if let (Some(ref manager), Some(agent_id)) =
            (&self.manager, self.selected_agent_id().map(|s| s.to_string()))
        {
            let _ = manager.kill(&agent_id);
            self.refresh();
        }
    }

    /// Pause selected agent
    pub fn pause_selected(&mut self) {
        if let (Some(ref manager), Some(agent_id)) =
            (&self.manager, self.selected_agent_id().map(|s| s.to_string()))
        {
            let _ = manager.pause(&agent_id);
            self.refresh();
        }
    }

    /// Resume selected agent
    pub fn resume_selected(&mut self) {
        if let (Some(ref manager), Some(agent_id)) =
            (&self.manager, self.selected_agent_id().map(|s| s.to_string()))
        {
            let _ = manager.resume(&agent_id);
            self.refresh();
        }
    }

    /// Get the stats
    pub fn stats(&self) -> &ManagerStats {
        &self.stats
    }
}

impl Default for SwarmView {
    fn default() -> Self {
        Self::new()
    }
}

/// Draw the swarm view
pub fn draw(f: &mut Frame, swarm_view: &mut SwarmView, area: Rect) {
    if swarm_view.show_detail {
        draw_detail_view(f, swarm_view, area);
    } else {
        draw_list_view(f, swarm_view, area);
    }
}

fn draw_list_view(f: &mut Frame, swarm_view: &mut SwarmView, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title with stats
            Constraint::Length(3), // Budget gauge
            Constraint::Min(0),    // Agent list
            Constraint::Length(3), // Help
        ])
        .split(area);

    // Title with stats
    let stats = swarm_view.stats();
    let title_text = format!(
        "Agent Swarm - {} active, {} total | Cost: ${:.2}",
        stats.active_agents, stats.total_agents, stats.total_cost
    );
    let title = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Budget gauge (if there's a budget set)
    let budget_ratio = if stats.total_budget > 0.0 {
        (stats.total_cost / stats.total_budget).min(1.0)
    } else {
        0.0
    };
    let budget_color = if budget_ratio > 0.9 {
        Color::Red
    } else if budget_ratio > 0.7 {
        Color::Yellow
    } else {
        Color::Green
    };
    let budget_label = if stats.total_budget > 0.0 {
        format!(
            "Budget: ${:.2} / ${:.2} ({:.0}%)",
            stats.total_cost,
            stats.total_budget,
            budget_ratio * 100.0
        )
    } else {
        format!("Budget: ${:.2} (no limit set)", stats.total_cost)
    };
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Cost"))
        .gauge_style(Style::default().fg(budget_color))
        .percent((budget_ratio * 100.0) as u16)
        .label(budget_label);
    f.render_widget(gauge, chunks[1]);

    // Agent list
    let items: Vec<ListItem> = swarm_view
        .agents
        .iter()
        .map(|agent| create_agent_list_item(agent))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!("Agents ({})", swarm_view.agents.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, chunks[2], &mut swarm_view.list_state);

    // Help
    let help_text = vec![Line::from(vec![
        Span::raw("j/k or ↑/↓ (navigate)  "),
        Span::styled("Enter: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Details  "),
        Span::styled("p: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Pause  "),
        Span::styled("r: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Resume  "),
        Span::styled("x: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Kill  "),
        Span::styled("Tab: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Switch View  "),
        Span::styled("q: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Quit"),
    ])];
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help, chunks[3]);
}

fn create_agent_list_item(agent: &AgentSnapshot) -> ListItem<'static> {
    let status_color = match agent.status {
        AgentStatus::Starting => Color::Blue,
        AgentStatus::Running => Color::Green,
        AgentStatus::Waiting => Color::Yellow,
        AgentStatus::Paused => Color::Gray,
        AgentStatus::Error => Color::Red,
        AgentStatus::Completed => Color::Cyan,
        AgentStatus::Killed => Color::DarkGray,
    };

    let status_emoji = agent.status.emoji();

    let rig_str = agent
        .rig
        .as_ref()
        .map(|r| format!(" [{}]", r))
        .unwrap_or_default();

    let spans = vec![
        Span::styled(
            format!("{} ", status_emoji),
            Style::default().fg(status_color),
        ),
        Span::styled(
            format!("{} ", agent.name),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("({}) ", agent.persona),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(rig_str, Style::default().fg(Color::Yellow)),
        Span::raw("  "),
        Span::styled(
            format!("${:.2}", agent.cost_usd),
            Style::default().fg(Color::Green),
        ),
        Span::raw("  "),
        Span::styled(agent.runtime.clone(), Style::default().fg(Color::DarkGray)),
    ];

    ListItem::new(Line::from(spans))
}

fn draw_detail_view(f: &mut Frame, swarm_view: &mut SwarmView, area: Rect) {
    if let Some(agent) = swarm_view.selected_agent().cloned() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Help
            ])
            .split(area);

        // Title
        let status_emoji = agent.status.emoji();
        let title = Paragraph::new(format!(
            "{} {} - {}",
            status_emoji, agent.name, agent.persona
        ))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Content - split into info and cost sections
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(chunks[1]);

        // Left side - agent info
        let mut text = Vec::new();

        text.push(Line::from(vec![
            Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&agent.id),
        ]));

        text.push(Line::from(vec![
            Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{:?}", agent.status),
                Style::default().fg(status_color(agent.status)),
            ),
        ]));

        text.push(Line::from(vec![
            Span::styled("Message: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&agent.status_message),
        ]));

        text.push(Line::from(vec![
            Span::styled("Context: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&agent.context),
        ]));

        if let Some(ref rig) = agent.rig {
            text.push(Line::from(vec![
                Span::styled("Rig: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(rig),
            ]));
        }

        if let Some(ref bead_id) = agent.bead_id {
            text.push(Line::from(vec![
                Span::styled("Bead: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(bead_id),
            ]));
        }

        text.push(Line::from(vec![
            Span::styled("Runtime: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&agent.runtime),
        ]));

        if !agent.locked_files.is_empty() {
            text.push(Line::raw(""));
            text.push(Line::from(Span::styled(
                "Locked Files:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for file in &agent.locked_files {
                text.push(Line::raw(format!("  - {}", file)));
            }
        }

        let info = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Agent Info"))
            .wrap(Wrap { trim: true });
        f.render_widget(info, content_chunks[0]);

        // Right side - cost breakdown
        let cost_str = format!("${:.4}", agent.cost_usd);
        let input_str = agent.input_tokens.to_string();
        let output_str = agent.output_tokens.to_string();
        let calls_str = agent.api_calls.to_string();

        let cost_rows = vec![
            Row::new(vec!["Cost", cost_str.as_str()]),
            Row::new(vec!["Input Tokens", input_str.as_str()]),
            Row::new(vec!["Output Tokens", output_str.as_str()]),
            Row::new(vec!["API Calls", calls_str.as_str()]),
        ];

        let cost_table = Table::new(
            cost_rows,
            [Constraint::Percentage(50), Constraint::Percentage(50)],
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Cost Breakdown"),
        )
        .style(Style::default().fg(Color::White))
        .header(
            Row::new(vec!["Metric", "Value"])
                .style(Style::default().add_modifier(Modifier::BOLD))
                .bottom_margin(1),
        );
        f.render_widget(cost_table, content_chunks[1]);

        // Help - show available actions based on status
        let action_hint = match agent.status {
            AgentStatus::Running | AgentStatus::Waiting => "p: Pause  x: Kill",
            AgentStatus::Paused => "r: Resume  x: Kill",
            _ => "",
        };

        let help_text = vec![Line::from(vec![
            Span::styled("Esc/Enter: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("Back to List  "),
            Span::raw(action_hint),
            Span::raw("  "),
            Span::styled("q: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("Quit"),
        ])];
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(help, chunks[2]);
    }
}

fn status_color(status: AgentStatus) -> Color {
    match status {
        AgentStatus::Starting => Color::Blue,
        AgentStatus::Running => Color::Green,
        AgentStatus::Waiting => Color::Yellow,
        AgentStatus::Paused => Color::Gray,
        AgentStatus::Error => Color::Red,
        AgentStatus::Completed => Color::Cyan,
        AgentStatus::Killed => Color::DarkGray,
    }
}
