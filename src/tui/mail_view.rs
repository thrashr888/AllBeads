//! Mail inbox view for the TUI
//!
//! Displays Agent Mail messages with actions.

use crate::mail::{MessageType, Postmaster, StoredMessage};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

/// Mail view state
pub struct MailView {
    /// Messages in the inbox
    messages: Vec<StoredMessage>,
    /// Selected message index
    list_state: ListState,
    /// Currently viewing message details
    show_detail: bool,
    /// Unread count
    unread_count: usize,
}

impl MailView {
    /// Create a new mail view
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            messages: Vec::new(),
            list_state,
            show_detail: false,
            unread_count: 0,
        }
    }

    /// Refresh messages from postmaster
    pub fn refresh(&mut self, postmaster: &Postmaster, inbox_address: &crate::mail::Address) {
        if let Ok(messages) = postmaster.inbox(inbox_address) {
            self.unread_count = messages
                .iter()
                .filter(|m| m.status == crate::mail::DeliveryStatus::Delivered)
                .count();
            self.messages = messages;

            // Reset selection if out of bounds
            if self.messages.is_empty() {
                self.list_state.select(None);
            } else if self.list_state.selected().unwrap_or(0) >= self.messages.len() {
                self.list_state.select(Some(self.messages.len() - 1));
            }
        }
    }

    /// Get unread message count
    pub fn unread_count(&self) -> usize {
        self.unread_count
    }

    /// Move selection down
    pub fn next(&mut self) {
        if self.messages.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let next = if current >= self.messages.len().saturating_sub(1) {
            0
        } else {
            current + 1
        };
        self.list_state.select(Some(next));
    }

    /// Move selection up
    pub fn previous(&mut self) {
        if self.messages.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let prev = if current == 0 {
            self.messages.len().saturating_sub(1)
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

    /// Get selected message
    pub fn selected_message(&self) -> Option<&StoredMessage> {
        self.list_state
            .selected()
            .and_then(|i| self.messages.get(i))
    }

    /// Get selected message ID
    pub fn selected_message_id(&self) -> Option<&crate::mail::MessageId> {
        self.selected_message().map(|m| &m.message.id)
    }
}

impl Default for MailView {
    fn default() -> Self {
        Self::new()
    }
}

/// Draw the mail view
pub fn draw(f: &mut Frame, mail_view: &mut MailView, area: Rect) {
    if mail_view.show_detail {
        draw_detail_view(f, mail_view, area);
    } else {
        draw_inbox_view(f, mail_view, area);
    }
}

fn draw_inbox_view(f: &mut Frame, mail_view: &mut MailView, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Messages
            Constraint::Length(3), // Help
        ])
        .split(area);

    // Title with unread count
    let unread = mail_view.unread_count;
    let title_text = if unread > 0 {
        format!("Agent Mail Inbox ({} unread)", unread)
    } else {
        "Agent Mail Inbox".to_string()
    };
    let title = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Messages list
    let items: Vec<ListItem> = mail_view
        .messages
        .iter()
        .map(|msg| create_message_list_item(msg))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!("Messages ({})", mail_view.messages.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, chunks[1], &mut mail_view.list_state);

    // Help
    let help_text = vec![Line::from(vec![
        Span::raw("j/k or ↑/↓ (navigate)  "),
        Span::styled("Enter: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("View Message  "),
        Span::styled("r: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Mark Read  "),
        Span::styled("Tab: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Switch View  "),
        Span::styled("q: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("Quit"),
    ])];
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help, chunks[2]);
}

fn create_message_list_item(msg: &StoredMessage) -> ListItem<'static> {
    let is_unread = msg.status == crate::mail::DeliveryStatus::Delivered;

    let type_indicator = match &msg.message.message_type {
        MessageType::Lock(_) => "[LOCK]",
        MessageType::Unlock(_) => "[UNLOCK]",
        MessageType::Notify(_) => "[NOTIFY]",
        MessageType::Request(_) => "[REQUEST]",
        MessageType::Broadcast(_) => "[BROADCAST]",
        MessageType::Heartbeat(_) => "[HEARTBEAT]",
        MessageType::Response(_) => "[RESPONSE]",
    };

    let type_color = match &msg.message.message_type {
        MessageType::Lock(_) | MessageType::Unlock(_) => Color::Red,
        MessageType::Request(_) => Color::Yellow,
        MessageType::Notify(_) => Color::Green,
        MessageType::Broadcast(_) => Color::Magenta,
        MessageType::Response(_) => Color::Blue,
        MessageType::Heartbeat(_) => Color::Gray,
    };

    // Get message summary
    let summary = match &msg.message.message_type {
        MessageType::Lock(l) => format!("Lock: {}", l.path),
        MessageType::Unlock(u) => format!("Unlock: {}", u.path),
        MessageType::Notify(n) => n.message.clone(),
        MessageType::Request(r) => r.message.clone(),
        MessageType::Broadcast(b) => b.message.clone(),
        MessageType::Heartbeat(h) => format!("Status: {:?}", h.status),
        MessageType::Response(r) => r
            .message
            .clone()
            .unwrap_or_else(|| format!("{:?}", r.status)),
    };

    let from = msg.message.from.to_string();
    let timestamp = msg.message.timestamp.format("%H:%M").to_string();

    let mut spans = vec![
        Span::styled(
            if is_unread { "* " } else { "  " },
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(
            format!("{} ", type_indicator),
            Style::default().fg(type_color),
        ),
        Span::styled(
            format!("{} ", timestamp),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(format!("{}: ", from), Style::default().fg(Color::Cyan)),
    ];

    // Truncate summary if too long
    let max_summary_len = 50;
    let summary_display = if summary.len() > max_summary_len {
        format!("{}...", &summary[..max_summary_len])
    } else {
        summary
    };

    let style = if is_unread {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    spans.push(Span::styled(summary_display, style));

    ListItem::new(Line::from(spans))
}

fn draw_detail_view(f: &mut Frame, mail_view: &mut MailView, area: Rect) {
    if let Some(msg) = mail_view.selected_message() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Help
            ])
            .split(area);

        // Title
        let type_name = match &msg.message.message_type {
            MessageType::Lock(_) => "Lock Request",
            MessageType::Unlock(_) => "Unlock Request",
            MessageType::Notify(_) => "Notification",
            MessageType::Request(_) => "Request",
            MessageType::Broadcast(_) => "Broadcast",
            MessageType::Heartbeat(_) => "Heartbeat",
            MessageType::Response(_) => "Response",
        };
        let title = Paragraph::new(format!("{} from {}", type_name, msg.message.from))
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Content
        let mut text = Vec::new();

        text.push(Line::from(vec![
            Span::styled("From: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(msg.message.from.to_string()),
        ]));

        text.push(Line::from(vec![
            Span::styled("To: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(msg.message.to.to_string()),
        ]));

        text.push(Line::from(vec![
            Span::styled("Time: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(msg.message.timestamp.to_rfc3339()),
        ]));

        text.push(Line::from(vec![
            Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{:?}", msg.status)),
        ]));

        text.push(Line::raw(""));
        text.push(Line::from(Span::styled(
            "Message:",
            Style::default().add_modifier(Modifier::BOLD),
        )));

        // Message content based on type
        match &msg.message.message_type {
            MessageType::Lock(l) => {
                text.push(Line::raw(format!("Path: {}", l.path)));
                text.push(Line::raw(format!("TTL: {} seconds", l.ttl.as_secs())));
                if let Some(ref reason) = l.reason {
                    text.push(Line::raw(format!("Reason: {}", reason)));
                }
            }
            MessageType::Unlock(u) => {
                text.push(Line::raw(format!("Path: {}", u.path)));
            }
            MessageType::Notify(n) => {
                text.push(Line::raw(&n.message));
                text.push(Line::raw(format!("Severity: {:?}", n.severity)));
            }
            MessageType::Request(r) => {
                text.push(Line::raw(&r.message));
                if !r.options.is_empty() {
                    text.push(Line::raw(""));
                    text.push(Line::from(Span::styled(
                        "Options:",
                        Style::default().add_modifier(Modifier::BOLD),
                    )));
                    for opt in &r.options {
                        text.push(Line::raw(format!("  - {}", opt)));
                    }
                }
            }
            MessageType::Broadcast(b) => {
                text.push(Line::raw(&b.message));
                text.push(Line::raw(format!("Category: {:?}", b.category)));
            }
            MessageType::Heartbeat(h) => {
                text.push(Line::raw(format!("Status: {:?}", h.status)));
                if let Some(ref task) = h.task {
                    text.push(Line::raw(format!("Current task: {}", task)));
                }
                if let Some(progress) = h.progress {
                    text.push(Line::raw(format!("Progress: {}%", progress)));
                }
            }
            MessageType::Response(r) => {
                text.push(Line::raw(format!("Status: {:?}", r.status)));
                if let Some(ref message) = r.message {
                    text.push(Line::raw(message));
                }
            }
        }

        let content = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        f.render_widget(content, chunks[1]);

        // Help - show available actions
        let action_hint = match &msg.message.message_type {
            MessageType::Request(_) => "  a/d: Approve/Deny",
            MessageType::Lock(_) => "  b: Break Lock",
            _ => "",
        };

        let help_text = vec![Line::from(vec![
            Span::styled("Esc/Enter: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("Back to Inbox"),
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
