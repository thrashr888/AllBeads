//! TUI rendering

use super::app::{App, Column, Tab};
use super::governance_view;
use super::graph_view;
use super::mail_view;
use super::stats_view;
use super::swarm_view;
use super::timeline_view;
use crate::graph::{Bead, Priority};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App) {
    match app.current_tab {
        Tab::Kanban => {
            if app.show_detail {
                draw_detail_view(f, app);
            } else {
                draw_kanban_view(f, app);
            }
        }
        Tab::Mail => {
            draw_mail_tab(f, app);
        }
        Tab::Graph => {
            draw_graph_tab(f, app);
        }
        Tab::Stats => {
            draw_stats_tab(f, app);
        }
        Tab::Timeline => {
            draw_timeline_tab(f, app);
        }
        Tab::Governance => {
            draw_governance_tab(f, app);
        }
        Tab::Swarm => {
            draw_swarm_tab(f, app);
        }
    }
}

fn draw_mail_tab(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Content
        ])
        .split(f.area());

    draw_tab_bar(f, app, chunks[0]);
    mail_view::draw(f, &mut app.mail_view, chunks[1]);
}

fn draw_graph_tab(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Content
        ])
        .split(f.area());

    draw_tab_bar(f, app, chunks[0]);
    graph_view::draw(f, &mut app.graph_view, &app.graph, chunks[1]);
}

fn draw_swarm_tab(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Content
        ])
        .split(f.area());

    draw_tab_bar(f, app, chunks[0]);
    swarm_view::draw(f, &mut app.swarm_view, chunks[1]);
}

fn draw_stats_tab(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Content
        ])
        .split(f.area());

    draw_tab_bar(f, app, chunks[0]);
    stats_view::draw(f, &app.stats_view, chunks[1]);
}

fn draw_timeline_tab(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Content
        ])
        .split(f.area());

    draw_tab_bar(f, app, chunks[0]);
    timeline_view::draw(f, &mut app.timeline_view, chunks[1]);
}

fn draw_governance_tab(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Content
        ])
        .split(f.area());

    draw_tab_bar(f, app, chunks[0]);
    governance_view::draw(f, &mut app.governance_view, chunks[1]);
}

fn draw_tab_bar(f: &mut Frame, app: &App, area: Rect) {
    // Create owned strings for tab titles
    // Tab order: Kanban, Mail (if available), Graph, Timeline, Governance, Stats, Swarm (if available)
    let mut tab_titles: Vec<String> = vec!["Kanban".to_string()];
    let mut graph_index = 1;
    let mut timeline_index = 2;
    let mut governance_index = 3;
    let mut stats_index = 4;
    let mut swarm_index = 5;

    // Add mail tab if available
    if app.has_mail() {
        let unread = app.unread_mail_count();
        if unread > 0 {
            tab_titles.push(format!("Mail ({})", unread));
        } else {
            tab_titles.push("Mail".to_string());
        }
        graph_index = 2;
        timeline_index = 3;
        governance_index = 4;
        stats_index = 5;
        swarm_index = 6;
    }

    // Graph tab is always present
    tab_titles.push("Graph".to_string());

    // Timeline tab is always present
    tab_titles.push("Timeline".to_string());

    // Governance tab is always present
    tab_titles.push("Governance".to_string());

    // Stats tab is always present
    tab_titles.push("Stats".to_string());

    // Add swarm tab if available
    if app.has_swarm() {
        let active = app.active_agent_count();
        if active > 0 {
            tab_titles.push(format!("Swarm ({})", active));
        } else {
            tab_titles.push("Swarm".to_string());
        }
    }

    // Calculate selected index
    let tab_index = match app.current_tab {
        Tab::Kanban => 0,
        Tab::Mail => 1,
        Tab::Graph => graph_index,
        Tab::Timeline => timeline_index,
        Tab::Governance => governance_index,
        Tab::Stats => stats_index,
        Tab::Swarm => swarm_index,
    };

    let tabs = Tabs::new(
        tab_titles
            .iter()
            .map(|s| Line::from(s.as_str()))
            .collect::<Vec<_>>(),
    )
    .block(Block::default().borders(Borders::ALL).title("AllBeads"))
    .select(tab_index)
    .style(Style::default().fg(Color::White))
    .highlight_style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(tabs, area);
}

fn draw_kanban_view(f: &mut Frame, app: &mut App) {
    // If mail is available, show tab bar; otherwise show title
    let has_mail = app.has_mail();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title or Tab bar
            Constraint::Min(0),    // Kanban board
            Constraint::Length(3), // Help (needs 3 for borders + 1 line of text)
        ])
        .split(f.area());

    // Title or Tab bar
    if has_mail {
        draw_tab_bar(f, app, chunks[0]);
    } else {
        let title = Paragraph::new("AllBeads - Multi-Context Task Aggregator")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);
    }

    // Kanban board
    let board_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(chunks[1]);

    let columns = Column::all();
    draw_column(f, app, columns[0], board_chunks[0]);
    draw_column(f, app, columns[1], board_chunks[1]);
    draw_column(f, app, columns[2], board_chunks[2]);

    // Help
    let mut help_spans = vec![
        Span::raw("j/k or ↑/↓ (up/down)  h/l or ←/→ (switch column)  "),
        Span::styled("Enter: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("View Details  "),
    ];
    if has_mail {
        help_spans.push(Span::styled(
            "Tab: ",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        help_spans.push(Span::raw("Switch View  "));
    }
    help_spans.push(Span::styled(
        "q: ",
        Style::default().add_modifier(Modifier::BOLD),
    ));
    help_spans.push(Span::raw("Quit  "));
    help_spans.push(Span::styled(
        "[READ-ONLY]",
        Style::default().fg(Color::Yellow),
    ));

    let help_text = vec![Line::from(help_spans)];
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help, chunks[2]);
}

fn draw_column(f: &mut Frame, app: &mut App, column: Column, area: Rect) {
    let is_selected = app.current_column == column;
    let border_style = if is_selected {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    let status = column.to_status();
    let beads: Vec<_> = app
        .graph
        .beads
        .values()
        .filter(|b| b.status == status)
        .collect();

    let mut sorted_beads = beads;
    sorted_beads.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.title.cmp(&b.title))
    });

    let items: Vec<ListItem> = sorted_beads
        .iter()
        .enumerate()
        .map(|(i, bead)| {
            // Only highlight in the selected column, using list_state selection
            let is_current = is_selected && Some(i) == app.list_state.selected();
            create_bead_list_item(bead, is_current)
        })
        .collect();

    let title = format!("{} ({})", column.title(), sorted_beads.len());
    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    // Only use stateful rendering for the selected column
    if is_selected {
        f.render_stateful_widget(list, area, &mut app.list_state);
    } else {
        f.render_widget(list, area);
    }
}

fn create_bead_list_item(bead: &Bead, is_selected: bool) -> ListItem<'_> {
    let priority_color = match bead.priority {
        Priority::P0 => Color::Red,
        Priority::P1 => Color::LightRed,
        Priority::P2 => Color::Yellow,
        Priority::P3 => Color::LightBlue,
        Priority::P4 => Color::Gray,
    };

    let priority_str = format!("[{:?}] ", bead.priority);

    // Extract context tags
    let context_tags: Vec<_> = bead.labels.iter().filter(|l| l.starts_with('@')).collect();

    let context_str = if !context_tags.is_empty() {
        let tags: Vec<&str> = context_tags.iter().map(|s| s.as_str()).collect();
        format!(" ({})", tags.join(", "))
    } else {
        String::new()
    };

    // Truncate title if too long
    let max_title_len = 50;
    let title = if bead.title.len() > max_title_len {
        format!("{}...", &bead.title[..max_title_len])
    } else {
        bead.title.clone()
    };

    let mut spans = vec![
        Span::styled(priority_str, Style::default().fg(priority_color)),
        Span::raw(bead.id.as_str().to_string()),
        Span::raw(": "),
        Span::raw(title),
    ];

    if !context_str.is_empty() {
        spans.push(Span::styled(context_str, Style::default().fg(Color::Cyan)));
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

fn draw_detail_view(f: &mut Frame, app: &mut App) {
    if let Some(bead) = app.selected_bead() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Help (needs 3 for borders + 1 line of text)
            ])
            .split(f.area());

        // Title
        let title = format!("{}: {}", bead.id.as_str(), bead.title);
        let title_widget = Paragraph::new(title)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title_widget, chunks[0]);

        // Content
        let mut text = Vec::new();

        text.push(Line::from(vec![
            Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{:?}", bead.status)),
        ]));

        text.push(Line::from(vec![
            Span::styled("Priority: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{:?}", bead.priority),
                Style::default().fg(priority_color(bead.priority)),
            ),
        ]));

        text.push(Line::from(vec![
            Span::styled("Type: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{:?}", bead.issue_type)),
        ]));

        text.push(Line::from(vec![
            Span::styled("Created: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{} by {}", bead.created_at, bead.created_by)),
        ]));

        text.push(Line::from(vec![
            Span::styled("Updated: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&bead.updated_at),
        ]));

        if let Some(ref assignee) = bead.assignee {
            text.push(Line::from(vec![
                Span::styled("Assignee: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(assignee),
            ]));
        }

        if !bead.labels.is_empty() {
            let labels = bead
                .labels
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            text.push(Line::from(vec![
                Span::styled("Labels: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(labels, Style::default().fg(Color::Cyan)),
            ]));
        }

        if !bead.dependencies.is_empty() {
            let deps = bead
                .dependencies
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            text.push(Line::from(vec![
                Span::styled(
                    "Depends on: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(deps),
            ]));
        }

        if !bead.blocks.is_empty() {
            let blocks = bead
                .blocks
                .iter()
                .map(|id| id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            text.push(Line::from(vec![
                Span::styled("Blocks: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(blocks),
            ]));
        }

        if let Some(ref description) = bead.description {
            text.push(Line::raw(""));
            text.push(Line::from(Span::styled(
                "Description:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            text.push(Line::raw(description.as_str()));
        }

        if let Some(ref notes) = bead.notes {
            text.push(Line::raw(""));
            text.push(Line::from(Span::styled(
                "Notes:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            text.push(Line::raw(notes.as_str()));
        }

        let content = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        f.render_widget(content, chunks[1]);

        // Help
        let help_text = vec![Line::from(vec![
            Span::styled("Esc/Enter: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("Back to Kanban  "),
            Span::styled("q: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("Quit  "),
            Span::styled("[READ-ONLY]", Style::default().fg(Color::Yellow)),
        ])];
        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL).title("Help"));
        f.render_widget(help, chunks[2]);
    }
}

fn priority_color(priority: Priority) -> Color {
    match priority {
        Priority::P0 => Color::Red,
        Priority::P1 => Color::LightRed,
        Priority::P2 => Color::Yellow,
        Priority::P3 => Color::LightBlue,
        Priority::P4 => Color::Gray,
    }
}
