//! TUI rendering

use super::app::{App, Column};
use crate::graph::{Bead, Priority};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App) {
    if app.show_detail {
        draw_detail_view(f, app);
    } else {
        draw_kanban_view(f, app);
    }
}

fn draw_kanban_view(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Kanban board
            Constraint::Length(3), // Help (needs 3 for borders + 1 line of text)
        ])
        .split(f.area());

    // Title
    let title = Paragraph::new("AllBeads - Multi-Context Task Aggregator")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

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
    let help_text = vec![Line::from(vec![
        Span::raw("j/k or ↑/↓ (up/down)  h/l or ←/→ (switch column)  "),
        Span::styled("Enter: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("View Details  "),
        Span::styled("q: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Quit  "),
        Span::styled("[READ-ONLY]", Style::default().fg(Color::Yellow)),
    ])];
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
