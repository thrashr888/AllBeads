//! Terminal User Interface for AllBeads
//!
//! Provides a Kanban-style dashboard for viewing beads across multiple contexts.

pub mod aiki_view;
mod app;
pub mod contexts_view;
pub mod github_picker_view;
pub mod governance_view;
pub mod graph_view;
pub mod mail_view;
pub mod stats_view;
pub mod swarm_view;
pub mod timeline_view;
mod ui;

pub use aiki_view::AikiView;
pub use app::{App, Tab};
pub use contexts_view::ContextsView;
pub use github_picker_view::GitHubPickerView;
pub use governance_view::GovernanceView;
pub use graph_view::GraphView;
pub use stats_view::StatsView;
pub use swarm_view::SwarmView;
pub use timeline_view::TimelineView;

use crate::graph::FederatedGraph;
use crate::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::PathBuf;

/// Run the TUI application (without mail support)
pub fn run(graph: FederatedGraph) -> Result<()> {
    run_with_mail(graph, None, "default")
}

/// Run the TUI application with optional mail support
pub fn run_with_mail(
    graph: FederatedGraph,
    mail_db_path: Option<PathBuf>,
    project_id: &str,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app with or without mail
    let mut app = if let Some(db_path) = mail_db_path {
        App::with_mail(graph, db_path, project_id)
    } else {
        App::new(graph)
    };

    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Handle deferred context loading after draw so loading message shows
        app.do_contexts_refresh();

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Global keys
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(())
                    }
                    KeyCode::Tab => {
                        app.next_tab();
                        continue;
                    }
                    _ => {}
                }

                // Tab-specific keys
                match app.current_tab {
                    Tab::Kanban => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.next(),
                        KeyCode::Char('k') | KeyCode::Up => app.previous(),
                        KeyCode::Char('h') | KeyCode::Left => app.previous_column(),
                        KeyCode::Char('l') | KeyCode::Right => app.next_column(),
                        KeyCode::Enter => app.toggle_detail(),
                        KeyCode::Esc => app.close_detail(),
                        _ => {}
                    },
                    Tab::Mail => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.mail_view.next(),
                        KeyCode::Char('k') | KeyCode::Up => app.mail_view.previous(),
                        KeyCode::Enter => app.mail_view.toggle_detail(),
                        KeyCode::Esc => app.mail_view.close_detail(),
                        KeyCode::Char('r') => app.mark_message_read(),
                        _ => {}
                    },
                    Tab::Graph => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.graph_view.next(),
                        KeyCode::Char('k') | KeyCode::Up => app.graph_view.previous(),
                        KeyCode::Enter => app.graph_view.toggle_detail(),
                        KeyCode::Esc => app.graph_view.close_detail(),
                        KeyCode::Char('f') => app.graph_view.cycle_filter(),
                        _ => {}
                    },
                    Tab::Stats => {
                        // Stats is a read-only view, no special keys needed
                    }
                    Tab::Timeline => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.timeline_view.next(),
                        KeyCode::Char('k') | KeyCode::Up => app.timeline_view.previous(),
                        KeyCode::Enter => app.timeline_view.toggle_detail(),
                        KeyCode::Esc => app.timeline_view.close_detail(),
                        KeyCode::Char('+') | KeyCode::Char('=') => app.timeline_view.zoom_out(),
                        KeyCode::Char('-') => app.timeline_view.zoom_in(),
                        _ => {}
                    },
                    Tab::Governance => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.governance_view.next(),
                        KeyCode::Char('k') | KeyCode::Up => app.governance_view.previous(),
                        KeyCode::Char('h') | KeyCode::Left => app.governance_view.next_section(),
                        KeyCode::Char('l') | KeyCode::Right => app.governance_view.next_section(),
                        KeyCode::Enter => app.governance_view.toggle_detail(),
                        KeyCode::Esc => app.governance_view.close_detail(),
                        _ => {}
                    },
                    Tab::Swarm => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.swarm_view.next(),
                        KeyCode::Char('k') | KeyCode::Up => app.swarm_view.previous(),
                        KeyCode::Enter => app.swarm_view.toggle_detail(),
                        KeyCode::Esc => app.swarm_view.close_detail(),
                        KeyCode::Char('p') => app.swarm_view.pause_selected(),
                        KeyCode::Char('r') => app.swarm_view.resume_selected(),
                        KeyCode::Char('x') => app.swarm_view.kill_selected(),
                        _ => {}
                    },
                    Tab::Aiki => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.aiki_view.next(),
                        KeyCode::Char('k') | KeyCode::Up => app.aiki_view.previous(),
                        KeyCode::Char('r') => app.refresh_aiki_view(),
                        _ => {}
                    },
                    Tab::Contexts => match key.code {
                        KeyCode::Char('j') | KeyCode::Down => app.contexts_view.next(),
                        KeyCode::Char('k') | KeyCode::Up => app.contexts_view.previous(),
                        KeyCode::Char('r') => app.force_refresh_contexts_view(),
                        KeyCode::Char('s') => app.contexts_view.cycle_sort(),
                        KeyCode::Char('o') => app.contexts_view.cycle_org_filter(),
                        KeyCode::Enter => app.contexts_view.toggle_detail(),
                        KeyCode::Esc => app.contexts_view.close_detail(),
                        _ => {}
                    },
                    Tab::GitHubPicker => {
                        if app.github_picker_view.input_mode {
                            // Input mode - capture characters for search query
                            match key.code {
                                KeyCode::Enter => {
                                    // Execute search or exit input mode
                                    app.github_picker_view.toggle_input_mode();
                                }
                                KeyCode::Esc => app.github_picker_view.toggle_input_mode(),
                                KeyCode::Backspace => app.github_picker_view.pop_char(),
                                KeyCode::Char(c) => app.github_picker_view.push_char(c),
                                _ => {}
                            }
                        } else {
                            // Navigation mode
                            match key.code {
                                KeyCode::Char('j') | KeyCode::Down => app.github_picker_view.next(),
                                KeyCode::Char('k') | KeyCode::Up => app.github_picker_view.previous(),
                                KeyCode::Char('m') => app.github_picker_view.toggle_mode(),
                                KeyCode::Char('/') => app.github_picker_view.toggle_input_mode(),
                                KeyCode::Enter => app.github_picker_view.toggle_detail(),
                                KeyCode::Esc => app.github_picker_view.close_detail(),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
}
