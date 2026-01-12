//! Terminal User Interface for AllBeads
//!
//! Provides a Kanban-style dashboard for viewing beads across multiple contexts.

mod app;
pub mod graph_view;
pub mod mail_view;
pub mod stats_view;
pub mod swarm_view;
mod ui;

pub use app::{App, Tab};
pub use graph_view::GraphView;
pub use stats_view::StatsView;
pub use swarm_view::SwarmView;

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
                }
            }
        }
    }
}
