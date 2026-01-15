//! GitHub Repository Picker view for the TUI
//!
//! Allows searching for GitHub repositories and selecting them for onboarding.

use crate::config::AllBeadsConfig;
use crate::governance::scanner::{
    GitHubScanner, OnboardingPriority, ScanFilter, ScanOptions, ScannedRepo,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::sync::mpsc;
use std::thread;

/// Search mode for the picker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    /// Search user's repositories
    User,
    /// Search organization's repositories
    Org,
}

impl SearchMode {
    pub fn name(&self) -> &'static str {
        match self {
            SearchMode::User => "User",
            SearchMode::Org => "Org",
        }
    }
}

/// View state for GitHub repository picker
pub struct GitHubPickerView {
    /// Search mode (user or org)
    pub search_mode: SearchMode,
    /// Username or org name to search
    pub search_query: String,
    /// Whether currently searching/loading
    pub is_loading: bool,
    /// Scanned repositories
    pub repos: Vec<ScannedRepo>,
    /// List selection state
    pub list_state: ListState,
    /// Whether to show detail view
    pub show_detail: bool,
    /// Search filters
    pub filters: ScanFilter,
    /// Error message if any
    pub error: Option<String>,
    /// Currently managed context names (to highlight already managed repos)
    pub managed_repos: Vec<String>,
    /// Whether search has been performed
    pub has_searched: bool,
    /// Input mode (for entering search query)
    pub input_mode: bool,
    /// Channel for receiving search results
    pub result_receiver: Option<mpsc::Receiver<Result<Vec<ScannedRepo>, String>>>,
    /// Whether a search is pending
    pub search_pending: bool,
    /// Repository marked for onboarding (clone_url)
    pub pending_onboard: Option<String>,
    /// Status message to display
    pub status_message: Option<String>,
}

impl Default for GitHubPickerView {
    fn default() -> Self {
        Self::new()
    }
}

impl GitHubPickerView {
    /// Create a new GitHub picker view
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            search_mode: SearchMode::User,
            search_query: String::new(),
            is_loading: false,
            repos: Vec::new(),
            list_state,
            show_detail: false,
            filters: ScanFilter::default(),
            error: None,
            managed_repos: Vec::new(),
            has_searched: false,
            input_mode: true,
            result_receiver: None,
            search_pending: false,
            pending_onboard: None,
            status_message: None,
        }
    }

    /// Mark the currently selected repo for onboarding
    pub fn mark_for_onboard(&mut self) {
        // Clone data first to avoid borrow issues
        let repo_info = self.selected_repo().map(|r| (r.name.clone(), r.clone_url.clone()));

        if let Some((name, clone_url)) = repo_info {
            if !self.is_managed(&name) {
                self.pending_onboard = Some(clone_url);
                self.status_message = Some(format!(
                    "Marked '{}' for onboarding. Press 'q' to exit and run onboard.",
                    name
                ));
            }
        }
    }

    /// Get the pending onboard URL and clear it
    pub fn take_pending_onboard(&mut self) -> Option<String> {
        self.pending_onboard.take()
    }

    /// Clear status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Execute search in a background thread
    pub fn execute_search(&mut self) {
        if self.search_query.is_empty() {
            self.error = Some("Please enter a username or organization".to_string());
            return;
        }

        let query = self.search_query.clone();
        let mode = self.search_mode;
        let filters = self.filters.clone();

        // Create channel for results
        let (tx, rx) = mpsc::channel();
        self.result_receiver = Some(rx);
        self.is_loading = true;
        self.search_pending = true;
        self.error = None;

        // Spawn background thread to run async search
        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(async {
                // Get GITHUB_TOKEN from environment
                let token = std::env::var("GITHUB_TOKEN").ok();
                let scanner = GitHubScanner::new(token);

                let options = ScanOptions {
                    concurrency: 10,
                    use_search_api: true,
                    show_progress: false,
                };

                let scan_result = match mode {
                    SearchMode::User => scanner.scan_user_with_options(&query, &filters, &options).await,
                    SearchMode::Org => scanner.scan_org_with_options(&query, &filters, &options).await,
                };

                match scan_result {
                    Ok(result) => Ok(result.repositories),
                    Err(e) => Err(format!("Scan failed: {}", e)),
                }
            });

            let _ = tx.send(result);
        });
    }

    /// Check for completed search results
    pub fn poll_results(&mut self) {
        if let Some(ref rx) = self.result_receiver {
            // Non-blocking check for results
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(repos) => {
                        self.repos = repos;
                        self.repos.sort_by(|a, b| a.onboarding_priority.cmp(&b.onboarding_priority));
                        self.has_searched = true;
                        self.list_state.select(Some(0));
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
                self.is_loading = false;
                self.search_pending = false;
                self.result_receiver = None;
            }
        }
    }

    /// Load managed repos from config
    pub fn load_managed_repos(&mut self, config: &AllBeadsConfig) {
        self.managed_repos = config
            .contexts
            .iter()
            .map(|c| c.name.clone())
            .collect();
    }

    /// Set search results
    pub fn set_results(&mut self, repos: Vec<ScannedRepo>) {
        self.repos = repos;
        self.is_loading = false;
        self.has_searched = true;
        self.error = None;
        self.list_state.select(Some(0));
        // Sort by priority (High first)
        self.repos.sort_by(|a, b| a.onboarding_priority.cmp(&b.onboarding_priority));
    }

    /// Set error message
    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
        self.is_loading = false;
    }

    /// Start loading
    pub fn start_loading(&mut self) {
        self.is_loading = true;
        self.error = None;
    }

    /// Toggle search mode
    pub fn toggle_mode(&mut self) {
        self.search_mode = match self.search_mode {
            SearchMode::User => SearchMode::Org,
            SearchMode::Org => SearchMode::User,
        };
    }

    /// Toggle input mode
    pub fn toggle_input_mode(&mut self) {
        self.input_mode = !self.input_mode;
    }

    /// Navigate down in the list
    pub fn next(&mut self) {
        if self.repos.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let next = if current >= self.repos.len().saturating_sub(1) {
            0
        } else {
            current + 1
        };
        self.list_state.select(Some(next));
    }

    /// Navigate up in the list
    pub fn previous(&mut self) {
        if self.repos.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let prev = if current == 0 {
            self.repos.len().saturating_sub(1)
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

    /// Get selected repository
    pub fn selected_repo(&self) -> Option<&ScannedRepo> {
        let index = self.list_state.selected()?;
        self.repos.get(index)
    }

    /// Check if a repo is already managed
    pub fn is_managed(&self, repo_name: &str) -> bool {
        self.managed_repos.iter().any(|n| n == repo_name)
    }

    /// Add a character to search query
    pub fn push_char(&mut self, c: char) {
        if self.input_mode {
            self.search_query.push(c);
        }
    }

    /// Remove last character from search query
    pub fn pop_char(&mut self) {
        if self.input_mode {
            self.search_query.pop();
        }
    }

    /// Render the view
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Split into header, search bar, status (optional), and results
        let has_status = self.status_message.is_some();
        let constraints = if has_status {
            vec![
                Constraint::Length(3), // Header
                Constraint::Length(3), // Search bar
                Constraint::Length(3), // Status bar
                Constraint::Min(0),    // Results
            ]
        } else {
            vec![
                Constraint::Length(3), // Header
                Constraint::Length(3), // Search bar
                Constraint::Min(0),    // Results
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        self.render_header(frame, chunks[0]);
        self.render_search_bar(frame, chunks[1]);

        if has_status {
            self.render_status_bar(frame, chunks[2]);
            if self.show_detail {
                self.render_detail(frame, chunks[3]);
            } else {
                self.render_results(frame, chunks[3]);
            }
        } else if self.show_detail {
            self.render_detail(frame, chunks[2]);
        } else {
            self.render_results(frame, chunks[2]);
        }
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let msg = self.status_message.as_deref().unwrap_or("");
        let status = Paragraph::new(msg)
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title("Status"));
        frame.render_widget(status, area);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let mode_text = format!("[m] Mode: {} | [/] Search | [o] Onboard | [Enter] Detail | [Tab] Switch view", self.search_mode.name());
        let header = Paragraph::new(mode_text)
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL).title("GitHub Repo Picker"));
        frame.render_widget(header, area);
    }

    fn render_search_bar(&self, frame: &mut Frame, area: Rect) {
        let cursor = if self.input_mode { "_" } else { "" };
        let query_display = format!(
            "{} {} {}{}",
            if self.search_mode == SearchMode::User { "User:" } else { "Org:" },
            self.search_query,
            cursor,
            if self.is_loading { " (Loading...)" } else { "" }
        );

        let style = if self.input_mode {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let search_bar = Paragraph::new(query_display)
            .style(style)
            .block(Block::default().borders(Borders::ALL).title(if self.input_mode { "Search (type to enter, Enter to search)" } else { "Search" }));
        frame.render_widget(search_bar, area);
    }

    fn render_results(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(ref error) = self.error {
            let error_msg = Paragraph::new(format!("Error: {}", error))
                .style(Style::default().fg(Color::Red))
                .block(Block::default().borders(Borders::ALL).title("Error"));
            frame.render_widget(error_msg, area);
            return;
        }

        if !self.has_searched {
            let instructions = Paragraph::new("Enter a GitHub username or organization name to search.\n\nKeys:\n  [m] Toggle User/Org mode\n  [/] Enter search mode\n  [Enter] Execute search / Select repo\n  [j/k] Navigate results")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL).title("Instructions"));
            frame.render_widget(instructions, area);
            return;
        }

        if self.repos.is_empty() && !self.is_loading {
            let no_results = Paragraph::new("No repositories found. Try a different search.")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL).title("Results"));
            frame.render_widget(no_results, area);
            return;
        }

        let items: Vec<ListItem> = self
            .repos
            .iter()
            .map(|repo| {
                let is_managed = self.is_managed(&repo.name);
                let priority_indicator = match repo.onboarding_priority {
                    OnboardingPriority::High => "🔴",
                    OnboardingPriority::Medium => "🟡",
                    OnboardingPriority::Low => "🟢",
                    OnboardingPriority::Skip => "⚪",
                };

                let managed_indicator = if is_managed { " ✓" } else { "" };
                let agents_str = if repo.detected_agents.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", repo.detected_agents.iter().map(|a| a.id()).collect::<Vec<_>>().join(", "))
                };

                let style = if is_managed {
                    Style::default().fg(Color::Green)
                } else {
                    match repo.onboarding_priority {
                        OnboardingPriority::High => Style::default().fg(Color::Red),
                        OnboardingPriority::Medium => Style::default().fg(Color::Yellow),
                        OnboardingPriority::Low => Style::default().fg(Color::White),
                        OnboardingPriority::Skip => Style::default().fg(Color::DarkGray),
                    }
                };

                let line = Line::from(vec![
                    Span::styled(format!("{} ", priority_indicator), style),
                    Span::styled(format!("{}{}", repo.name, managed_indicator), style.add_modifier(Modifier::BOLD)),
                    Span::styled(agents_str, Style::default().fg(Color::Cyan)),
                    Span::styled(
                        format!(" ★{}", repo.stars),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]);
                ListItem::new(line)
            })
            .collect();

        let title = format!(
            "Results ({} repos, {} managed)",
            self.repos.len(),
            self.repos.iter().filter(|r| self.is_managed(&r.name)).count()
        );

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect) {
        let Some(repo) = self.selected_repo() else {
            let no_selection = Paragraph::new("No repository selected")
                .block(Block::default().borders(Borders::ALL).title("Detail"));
            frame.render_widget(no_selection, area);
            return;
        };

        let is_managed = self.is_managed(&repo.name);
        let priority_str = match repo.onboarding_priority {
            OnboardingPriority::High => "🔴 High - Strong onboarding candidate",
            OnboardingPriority::Medium => "🟡 Medium - Good candidate",
            OnboardingPriority::Low => "🟢 Low - Optional",
            OnboardingPriority::Skip => "⚪ Skip - Not recommended",
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Repository: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&repo.full_name),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Priority: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(priority_str),
            ]),
            Line::from(vec![
                Span::styled("Stars: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", repo.stars)),
            ]),
            Line::from(vec![
                Span::styled("Language: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(repo.language.as_deref().unwrap_or("Unknown")),
            ]),
            Line::from(vec![
                Span::styled("Managed: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    if is_managed { "Yes ✓" } else { "No" },
                    if is_managed { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Yellow) },
                ),
            ]),
        ];

        if !repo.detected_agents.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Detected Agents: ", Style::default().add_modifier(Modifier::BOLD)),
            ]));
            for agent in &repo.detected_agents {
                lines.push(Line::from(vec![
                    Span::raw("  • "),
                    Span::styled(agent.name(), Style::default().fg(Color::Cyan)),
                ]));
            }
        }

        if !is_managed {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Press Enter to mark for onboarding", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ]));
        }

        let detail = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(format!("Detail: {}", repo.name)));
        frame.render_widget(detail, area);
    }
}
