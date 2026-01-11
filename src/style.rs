//! Terminal styling utilities
//!
//! Provides consistent color scheme matching bd's output style.
//! Uses crossterm for cross-platform terminal colors.

use crossterm::style::{StyledContent, Stylize};

/// Priority colors (matches bd)
/// - P0/P1: Red/Orange (urgent)
/// - P2: Yellow (medium)
/// - P3/P4: Dim (low/backlog)
pub fn priority_style(priority: u8) -> StyledContent<String> {
    let label = format!("P{}", priority);
    match priority {
        0 => label.red().bold(),
        1 => label.dark_yellow().bold(), // Orange-ish
        2 => label.yellow(),
        3 => label.dark_grey(),
        4 => label.dark_grey(),
        _ => label.white(),
    }
}

/// Issue type colors (matches bd)
/// - epic: Magenta
/// - feature: Green
/// - bug: Red
/// - task: Cyan
/// - chore: Grey
pub fn type_style(issue_type: &str) -> StyledContent<String> {
    let label = format!("[{}]", issue_type);
    match issue_type.to_lowercase().as_str() {
        "epic" => label.magenta(),
        "feature" => label.green(),
        "bug" => label.red(),
        "task" => label.cyan(),
        "chore" => label.dark_grey(),
        "gate" => label.blue(),
        _ => label.white(),
    }
}

/// Status colors (matches bd)
/// - open: Default/white
/// - in_progress: Yellow
/// - blocked: Red
/// - closed: Dim grey
pub fn status_style(status: &str) -> StyledContent<String> {
    match status.to_lowercase().as_str() {
        "open" => status.to_string().white(),
        "in_progress" => status.to_string().yellow(),
        "blocked" => status.to_string().red(),
        "closed" => status.to_string().dark_grey(),
        _ => status.to_string().white(),
    }
}

/// Status indicator (circle)
pub fn status_indicator(status: &str) -> StyledContent<&'static str> {
    match status.to_lowercase().as_str() {
        "open" => "○".white(),
        "in_progress" => "◐".yellow(),
        "blocked" => "●".red(),
        "closed" => "✓".dark_grey(),
        _ => "○".white(),
    }
}

/// Count styling based on context
/// - Zero: Dim
/// - Positive: Green (for ready/open)
/// - Warning: Yellow
/// - Error: Red (for blocked)
pub fn count_ready(n: usize) -> StyledContent<String> {
    if n == 0 {
        n.to_string().dark_grey()
    } else {
        n.to_string().green()
    }
}

pub fn count_blocked(n: usize) -> StyledContent<String> {
    if n == 0 {
        n.to_string().dark_grey()
    } else {
        n.to_string().red()
    }
}

pub fn count_in_progress(n: usize) -> StyledContent<String> {
    if n == 0 {
        n.to_string().dark_grey()
    } else {
        n.to_string().yellow()
    }
}

pub fn count_normal(n: usize) -> StyledContent<String> {
    n.to_string().white()
}

/// Section headers
pub fn header(text: &str) -> StyledContent<String> {
    text.to_string().bold()
}

/// Subheaders
pub fn subheader(text: &str) -> StyledContent<String> {
    text.to_string().underlined()
}

/// Dim/muted text
pub fn dim(text: &str) -> StyledContent<String> {
    text.to_string().dark_grey()
}

/// Success text
pub fn success(text: &str) -> StyledContent<String> {
    text.to_string().green()
}

/// Warning text
pub fn warning(text: &str) -> StyledContent<String> {
    text.to_string().yellow()
}

/// Error text
pub fn error(text: &str) -> StyledContent<String> {
    text.to_string().red()
}

/// ID styling (matches bd's issue ID style)
pub fn issue_id(id: &str) -> StyledContent<String> {
    id.to_string().cyan()
}

/// Highlight important text (yellow)
pub fn highlight(text: &str) -> StyledContent<String> {
    text.to_string().yellow()
}

/// Path styling
pub fn path(p: &str) -> StyledContent<String> {
    p.to_string().blue()
}

/// Context/folder status colors
/// Matches Dry→Wet progression
pub fn folder_status(status: &str) -> StyledContent<String> {
    match status.to_lowercase().as_str() {
        "dry" => status.to_string().dark_grey(),
        "git" => status.to_string().blue(),
        "beads" => status.to_string().cyan(),
        "configured" | "config" => status.to_string().yellow(),
        "wet" => status.to_string().green(),
        _ => status.to_string().white(),
    }
}

/// Folder status indicator with emoji (minimal use)
pub fn folder_status_indicator(status: &str) -> &'static str {
    match status.to_lowercase().as_str() {
        "dry" => "○",                   // Empty circle
        "git" => "◔",                   // Quarter filled
        "beads" => "◑",                 // Half filled
        "configured" | "config" => "◕", // Three-quarter filled
        "wet" => "●",                   // Full circle
        _ => "○",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_colors() {
        // Just ensure they don't panic
        let _ = priority_style(0);
        let _ = priority_style(1);
        let _ = priority_style(2);
        let _ = priority_style(3);
        let _ = priority_style(4);
    }

    #[test]
    fn test_type_colors() {
        let _ = type_style("epic");
        let _ = type_style("task");
        let _ = type_style("bug");
        let _ = type_style("feature");
    }

    #[test]
    fn test_status_colors() {
        let _ = status_style("open");
        let _ = status_style("in_progress");
        let _ = status_style("blocked");
        let _ = status_style("closed");
    }
}
