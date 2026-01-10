//! Graph View - Dependency visualization for the TUI
//!
//! Renders cross-repository dependencies using ASCII/Unicode box drawing.

use crate::graph::{BeadId, FederatedGraph, Priority, Status};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::collections::HashSet;

/// State for the graph view
pub struct GraphView {
    /// List state for navigation
    pub list_state: ListState,
    /// Cached dependency chains for visualization
    chains: Vec<DependencyChain>,
    /// Whether to show detail view
    pub show_detail: bool,
    /// Filter: show all or just problematic chains
    pub filter_mode: FilterMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    /// Show all dependency chains
    All,
    /// Show only chains with blockers
    Blocked,
    /// Show only cross-context dependencies
    CrossContext,
}

/// A chain of dependencies
#[derive(Debug, Clone)]
pub struct DependencyChain {
    /// Root bead (the one being blocked)
    pub root: BeadId,
    /// Beads that block this one (direct blockers)
    pub blockers: Vec<BeadId>,
    /// All beads in the dependency tree (including transitive)
    pub all_deps: HashSet<BeadId>,
    /// Contexts involved
    pub contexts: HashSet<String>,
    /// Whether this chain crosses context boundaries
    pub is_cross_context: bool,
    /// Is there a cycle detected?
    pub has_cycle: bool,
}

impl Default for GraphView {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphView {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            list_state,
            chains: Vec::new(),
            show_detail: false,
            filter_mode: FilterMode::All,
        }
    }

    /// Analyze the graph and build dependency chains
    pub fn analyze(&mut self, graph: &FederatedGraph) {
        self.chains = build_dependency_chains(graph);
    }

    /// Get filtered chains based on current filter mode
    pub fn filtered_chains(&self) -> Vec<&DependencyChain> {
        self.chains
            .iter()
            .filter(|chain| match self.filter_mode {
                FilterMode::All => true,
                FilterMode::Blocked => !chain.blockers.is_empty(),
                FilterMode::CrossContext => chain.is_cross_context,
            })
            .collect()
    }

    /// Get selected chain
    pub fn selected_chain(&self) -> Option<&DependencyChain> {
        let chains = self.filtered_chains();
        let index = self.list_state.selected().unwrap_or(0);
        chains.get(index).copied()
    }

    pub fn next(&mut self) {
        let chains = self.filtered_chains();
        if chains.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let next = if current >= chains.len().saturating_sub(1) {
            0
        } else {
            current + 1
        };
        self.list_state.select(Some(next));
    }

    pub fn previous(&mut self) {
        let chains = self.filtered_chains();
        if chains.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let prev = if current == 0 {
            chains.len().saturating_sub(1)
        } else {
            current - 1
        };
        self.list_state.select(Some(prev));
    }

    pub fn toggle_detail(&mut self) {
        self.show_detail = !self.show_detail;
    }

    pub fn close_detail(&mut self) {
        self.show_detail = false;
    }

    pub fn cycle_filter(&mut self) {
        self.filter_mode = match self.filter_mode {
            FilterMode::All => FilterMode::Blocked,
            FilterMode::Blocked => FilterMode::CrossContext,
            FilterMode::CrossContext => FilterMode::All,
        };
        // Reset selection when filter changes
        self.list_state.select(Some(0));
    }
}

/// Build dependency chains from the graph
fn build_dependency_chains(graph: &FederatedGraph) -> Vec<DependencyChain> {
    let mut chains = Vec::new();

    for bead in graph.beads.values() {
        // Only analyze non-closed beads with dependencies or blockers
        if bead.status == Status::Closed {
            continue;
        }

        if bead.dependencies.is_empty() && bead.blocks.is_empty() {
            continue;
        }

        let mut all_deps = HashSet::new();
        let mut contexts = HashSet::new();
        let mut has_cycle = false;

        // Get context from labels
        for label in &bead.labels {
            if label.starts_with('@') {
                contexts.insert(label.clone());
            }
        }

        // Collect all transitive dependencies
        let mut to_visit: Vec<BeadId> = bead.dependencies.clone();
        let mut visited = HashSet::new();
        visited.insert(bead.id.clone());

        while let Some(dep_id) = to_visit.pop() {
            if visited.contains(&dep_id) {
                has_cycle = true;
                continue;
            }
            visited.insert(dep_id.clone());
            all_deps.insert(dep_id.clone());

            if let Some(dep_bead) = graph.beads.get(&dep_id) {
                // Add context from dependency
                for label in &dep_bead.labels {
                    if label.starts_with('@') {
                        contexts.insert(label.clone());
                    }
                }
                // Add transitive dependencies
                for trans_dep in &dep_bead.dependencies {
                    if !visited.contains(trans_dep) {
                        to_visit.push(trans_dep.clone());
                    }
                }
            }
        }

        let is_cross_context = contexts.len() > 1;

        chains.push(DependencyChain {
            root: bead.id.clone(),
            blockers: bead.dependencies.clone(),
            all_deps,
            contexts,
            is_cross_context,
            has_cycle,
        });
    }

    // Sort by: cross-context first, then by number of blockers (desc), then by root id
    chains.sort_by(|a, b| {
        b.is_cross_context
            .cmp(&a.is_cross_context)
            .then_with(|| b.blockers.len().cmp(&a.blockers.len()))
            .then_with(|| a.root.as_str().cmp(b.root.as_str()))
    });

    chains
}

/// Draw the graph view
pub fn draw(f: &mut Frame, view: &mut GraphView, graph: &FederatedGraph, area: Rect) {
    if view.show_detail {
        draw_detail(f, view, graph, area);
    } else {
        draw_list(f, view, graph, area);
    }
}

fn draw_list(f: &mut Frame, view: &mut GraphView, graph: &FederatedGraph, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Stats bar
            Constraint::Min(0),    // Chain list
            Constraint::Length(3), // Help
        ])
        .split(area);

    // Stats bar
    let total_chains = view.chains.len();
    let blocked_chains = view.chains.iter().filter(|c| !c.blockers.is_empty()).count();
    let cross_context = view.chains.iter().filter(|c| c.is_cross_context).count();
    let cycles = view.chains.iter().filter(|c| c.has_cycle).count();

    let filter_str = match view.filter_mode {
        FilterMode::All => "All",
        FilterMode::Blocked => "Blocked Only",
        FilterMode::CrossContext => "Cross-Context",
    };

    let stats_text = format!(
        "Total: {} | Blocked: {} | Cross-Context: {} | Cycles: {} | Filter: {}",
        total_chains, blocked_chains, cross_context, cycles, filter_str
    );
    let stats = Paragraph::new(stats_text)
        .block(Block::default().borders(Borders::ALL).title("Dependency Graph"))
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(stats, chunks[0]);

    // Get selected index before borrowing filtered chains
    let selected_idx = view.list_state.selected();

    // Build items from chains directly to avoid borrow issues
    let filtered_indices: Vec<usize> = view
        .chains
        .iter()
        .enumerate()
        .filter(|(_, chain)| match view.filter_mode {
            FilterMode::All => true,
            FilterMode::Blocked => !chain.blockers.is_empty(),
            FilterMode::CrossContext => chain.is_cross_context,
        })
        .map(|(i, _)| i)
        .collect();

    let filtered_len = filtered_indices.len();
    let items: Vec<ListItem> = filtered_indices
        .iter()
        .enumerate()
        .map(|(display_idx, &chain_idx)| {
            let chain = &view.chains[chain_idx];
            let is_selected = Some(display_idx) == selected_idx;
            create_chain_item(chain, graph, is_selected)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Chains ({})", filtered_len)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, chunks[1], &mut view.list_state);

    // Help
    let help_text = vec![Line::from(vec![
        Span::styled("j/k: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Navigate  "),
        Span::styled("Enter: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Details  "),
        Span::styled("f: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Cycle Filter  "),
        Span::styled("Tab: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Switch View  "),
        Span::styled("q: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Quit"),
    ])];
    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help, chunks[2]);
}

fn create_chain_item<'a>(
    chain: &'a DependencyChain,
    graph: &'a FederatedGraph,
    is_selected: bool,
) -> ListItem<'a> {
    // Get root bead info
    let (root_title, root_priority, _root_status) = graph
        .beads
        .get(&chain.root)
        .map(|b| (b.title.as_str(), b.priority, b.status))
        .unwrap_or(("Unknown", Priority::P4, Status::Open));

    // Build status indicator
    let status_icon = if chain.has_cycle {
        "⟳ "  // Cycle detected
    } else if chain.is_cross_context {
        "⬡ "  // Cross-context
    } else if !chain.blockers.is_empty() {
        "⊘ "  // Blocked
    } else {
        "○ "  // Normal
    };

    let status_color = if chain.has_cycle {
        Color::Red
    } else if chain.is_cross_context {
        Color::Magenta
    } else if !chain.blockers.is_empty() {
        Color::Yellow
    } else {
        Color::Green
    };

    let priority_color = match root_priority {
        Priority::P0 => Color::Red,
        Priority::P1 => Color::LightRed,
        Priority::P2 => Color::Yellow,
        Priority::P3 => Color::LightBlue,
        Priority::P4 => Color::Gray,
    };

    // Context tags
    let contexts: Vec<&str> = chain.contexts.iter().map(|s| s.as_str()).collect();
    let context_str = contexts.join(", ");

    // Truncate title
    let max_len = 40;
    let title = if root_title.len() > max_len {
        format!("{}...", &root_title[..max_len])
    } else {
        root_title.to_string()
    };

    let blocker_str = if chain.blockers.is_empty() {
        String::new()
    } else {
        format!(" [{} blockers]", chain.blockers.len())
    };

    let mut spans = vec![
        Span::styled(status_icon, Style::default().fg(status_color)),
        Span::styled(
            format!("[{:?}] ", root_priority),
            Style::default().fg(priority_color),
        ),
        Span::raw(chain.root.as_str().to_string()),
        Span::raw(": "),
        Span::raw(title),
        Span::styled(blocker_str, Style::default().fg(Color::Yellow)),
    ];

    if !context_str.is_empty() {
        spans.push(Span::styled(
            format!(" ({})", context_str),
            Style::default().fg(Color::Cyan),
        ));
    }

    let style = if is_selected {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    ListItem::new(Line::from(spans)).style(style)
}

fn draw_detail(f: &mut Frame, view: &mut GraphView, graph: &FederatedGraph, area: Rect) {
    let Some(chain) = view.selected_chain() else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Content (graph + info)
            Constraint::Length(3), // Help
        ])
        .split(area);

    // Title
    let root_title = graph
        .beads
        .get(&chain.root)
        .map(|b| b.title.as_str())
        .unwrap_or("Unknown");
    let title = format!("Dependency Chain: {} - {}", chain.root.as_str(), root_title);
    let title_widget = Paragraph::new(title)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title_widget, chunks[0]);

    // Content: split into graph and info
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[1]);

    // ASCII dependency graph
    let graph_lines = render_ascii_graph(chain, graph);
    let graph_widget = Paragraph::new(graph_lines)
        .block(Block::default().borders(Borders::ALL).title("Graph"))
        .wrap(Wrap { trim: false });
    f.render_widget(graph_widget, content_chunks[0]);

    // Info panel
    let mut info_lines = Vec::new();

    // Root info
    if let Some(root_bead) = graph.beads.get(&chain.root) {
        info_lines.push(Line::from(vec![
            Span::styled("Root: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(chain.root.as_str()),
        ]));
        info_lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{:?}", root_bead.status)),
        ]));
        info_lines.push(Line::from(vec![
            Span::styled("Priority: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{:?}", root_bead.priority)),
        ]));
    }

    info_lines.push(Line::raw(""));

    // Blockers
    info_lines.push(Line::from(Span::styled(
        format!("Direct Blockers ({}):", chain.blockers.len()),
        Style::default().add_modifier(Modifier::BOLD),
    )));
    for blocker in &chain.blockers {
        let blocker_title = graph
            .beads
            .get(blocker)
            .map(|b| b.title.as_str())
            .unwrap_or("Unknown");
        let blocker_status = graph
            .beads
            .get(blocker)
            .map(|b| b.status)
            .unwrap_or(Status::Open);
        let status_icon = if blocker_status == Status::Closed {
            "✓"
        } else {
            "○"
        };
        info_lines.push(Line::from(format!(
            "  {} {} - {}",
            status_icon,
            blocker.as_str(),
            blocker_title
        )));
    }

    info_lines.push(Line::raw(""));

    // Contexts
    info_lines.push(Line::from(Span::styled(
        "Contexts:",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    for ctx in &chain.contexts {
        info_lines.push(Line::from(format!("  {}", ctx)));
    }

    if chain.is_cross_context {
        info_lines.push(Line::raw(""));
        info_lines.push(Line::from(Span::styled(
            "⚠ Cross-context dependency chain",
            Style::default().fg(Color::Magenta),
        )));
    }

    if chain.has_cycle {
        info_lines.push(Line::raw(""));
        info_lines.push(Line::from(Span::styled(
            "⚠ Cycle detected in dependencies!",
            Style::default().fg(Color::Red),
        )));
    }

    let info_widget = Paragraph::new(info_lines)
        .block(Block::default().borders(Borders::ALL).title("Info"))
        .wrap(Wrap { trim: true });
    f.render_widget(info_widget, content_chunks[1]);

    // Help
    let help_text = vec![Line::from(vec![
        Span::styled("Esc/Enter: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Back  "),
        Span::styled("q: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Quit"),
    ])];
    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help, chunks[2]);
}

/// Render an ASCII representation of the dependency chain
fn render_ascii_graph<'a>(chain: &'a DependencyChain, graph: &'a FederatedGraph) -> Vec<Line<'a>> {
    let mut lines = Vec::new();

    // Get root bead
    let root_bead = graph.beads.get(&chain.root);
    let root_title = root_bead.map(|b| b.title.as_str()).unwrap_or("Unknown");
    let root_ctx = root_bead
        .and_then(|b| b.labels.iter().find(|l| l.starts_with('@')).cloned())
        .unwrap_or_default();

    // Root node
    let max_title = 30;
    let truncated_root = if root_title.len() > max_title {
        format!("{}...", &root_title[..max_title])
    } else {
        root_title.to_string()
    };

    lines.push(Line::from(vec![
        Span::styled(
            format!("┌─ {} ─────────────────────────┐", chain.root.as_str()),
            Style::default().fg(Color::Cyan),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("│ "),
        Span::raw(truncated_root.clone()),
        Span::raw(" ".repeat(35_usize.saturating_sub(truncated_root.len()))),
        Span::raw("│"),
    ]));
    if !root_ctx.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("│ "),
            Span::styled(root_ctx.clone(), Style::default().fg(Color::Cyan)),
            Span::raw(" ".repeat(35_usize.saturating_sub(root_ctx.len()))),
            Span::raw("│"),
        ]));
    }
    lines.push(Line::from(vec![Span::styled(
        "└────────────────────────────────────┘",
        Style::default().fg(Color::Cyan),
    )]));

    // Dependencies
    if !chain.blockers.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("        "),
            Span::styled("│", Style::default().fg(Color::Yellow)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("        "),
            Span::styled("▼ depends on", Style::default().fg(Color::Yellow)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("        "),
            Span::styled("│", Style::default().fg(Color::Yellow)),
        ]));

        for (i, blocker) in chain.blockers.iter().enumerate() {
            let blocker_bead = graph.beads.get(blocker);
            let blocker_title = blocker_bead.map(|b| b.title.as_str()).unwrap_or("Unknown");
            let blocker_status = blocker_bead.map(|b| b.status).unwrap_or(Status::Open);
            let blocker_ctx = blocker_bead
                .and_then(|b| b.labels.iter().find(|l| l.starts_with('@')).cloned())
                .unwrap_or_default();

            let status_color = if blocker_status == Status::Closed {
                Color::Green
            } else {
                Color::Yellow
            };
            let status_icon = if blocker_status == Status::Closed {
                "✓"
            } else {
                "○"
            };

            let truncated = if blocker_title.len() > max_title {
                format!("{}...", &blocker_title[..max_title])
            } else {
                blocker_title.to_string()
            };

            let connector = if i < chain.blockers.len() - 1 {
                "├──"
            } else {
                "└──"
            };

            lines.push(Line::from(vec![
                Span::raw("        "),
                Span::styled(connector, Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!(" {} ", status_icon),
                    Style::default().fg(status_color),
                ),
                Span::styled(
                    format!("{}: ", blocker.as_str()),
                    Style::default().fg(Color::White),
                ),
                Span::raw(truncated),
            ]));

            if !blocker_ctx.is_empty() && blocker_ctx != root_ctx {
                let continuation = if i < chain.blockers.len() - 1 {
                    "│"
                } else {
                    " "
                };
                lines.push(Line::from(vec![
                    Span::raw("        "),
                    Span::styled(continuation, Style::default().fg(Color::Yellow)),
                    Span::raw("       "),
                    Span::styled(blocker_ctx, Style::default().fg(Color::Magenta)),
                ]));
            }
        }
    }

    lines
}
