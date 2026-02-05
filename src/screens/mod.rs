mod deep_review;
mod loading;
mod pseudocode;
mod summary;
mod triage;

pub use deep_review::DeepReviewScreen;
pub use loading::LoadingScreen;
pub use pseudocode::PseudocodeReviewScreen;
pub use summary::SummaryScreen;
pub use triage::TriageScreen;

use crossterm::event::KeyEvent;
use ratatui::prelude::*;
use ratatui::Frame;

use crate::error::Result;
use crate::models::AppState;

/// Apply syntax highlighting to diff lines.
/// Returns styled Line elements for green (+), red (-), cyan (@@), and blue (diff/index) lines.
pub fn highlight_diff_lines(code: &str) -> Vec<Line<'_>> {
    code.lines()
        .map(|line| {
            let style = if line.starts_with('+') && !line.starts_with("+++") {
                Style::default().fg(Color::Green)
            } else if line.starts_with('-') && !line.starts_with("---") {
                Style::default().fg(Color::Red)
            } else if line.starts_with("@@") {
                Style::default().fg(Color::Cyan)
            } else if line.starts_with("diff") || line.starts_with("index") {
                Style::default().fg(Color::Blue)
            } else {
                Style::default()
            };
            Line::styled(line, style)
        })
        .collect()
}

use ratatui::widgets::{Block, Borders, Paragraph};

/// Shared helper to render an error panel with retry/quit hints.
/// This consolidates the common error rendering pattern used across screens.
pub fn render_error_panel(frame: &mut Frame, area: Rect, title: &str, error: &str, hints: &str) {
    let text = format!("Error: {}\n\n{}", error, hints);

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Red)),
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red));

    frame.render_widget(paragraph, area);
}

/// Shared helper to render footer hints with consistent styling.
/// Displays the hints text centered in DarkGray color.
pub fn render_footer_hints(frame: &mut Frame, area: Rect, hints: &str) {
    let footer = Paragraph::new(hints)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, area);
}

use crossterm::event::{KeyCode, KeyModifiers};

/// Scroll increment for Ctrl+D/Ctrl+U (half-page scroll)
const SCROLL_INCREMENT_HALF: usize = 10;

/// Scroll increment for PageDown/PageUp (full-page scroll)
const SCROLL_INCREMENT_FULL: usize = 20;

/// Handle scroll-related keyboard input for code preview panels.
/// Returns `Some(true)` if the event was consumed, `None` if not a scroll key.
/// Supports Ctrl+D (down half-page), Ctrl+U (up half-page), PageDown (down full-page), PageUp (up full-page).
pub fn handle_scroll_input(key: &KeyEvent, scroll_offset: &mut usize) -> Option<bool> {
    match key.code {
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            *scroll_offset = scroll_offset.saturating_add(SCROLL_INCREMENT_HALF);
            Some(true)
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            *scroll_offset = scroll_offset.saturating_sub(SCROLL_INCREMENT_HALF);
            Some(true)
        }
        KeyCode::PageDown => {
            *scroll_offset = scroll_offset.saturating_add(SCROLL_INCREMENT_FULL);
            Some(true)
        }
        KeyCode::PageUp => {
            *scroll_offset = scroll_offset.saturating_sub(SCROLL_INCREMENT_FULL);
            Some(true)
        }
        _ => None,
    }
}

/// Common response type for error state input handling.
/// Used by screens to handle the standard 'q' to quit and 'r' to retry keys.
pub enum ErrorInputResult {
    /// User pressed 'q' - quit the application
    Quit,
    /// User pressed 'r' - clear error and retry
    Retry,
    /// Key was not a common error-state key - screen should handle it
    NotHandled,
}

/// Handle common error state keyboard input (quit and retry).
/// Returns the action to take. Screens with additional error-state keys
/// should check for NotHandled and process their custom keys.
pub fn handle_error_state_input(key: &KeyEvent) -> ErrorInputResult {
    match key.code {
        KeyCode::Char('q') => ErrorInputResult::Quit,
        KeyCode::Char('r') => ErrorInputResult::Retry,
        _ => ErrorInputResult::NotHandled,
    }
}

/// Trait that all screens must implement
pub trait ScreenTrait {
    /// Render the screen to the terminal
    fn render(&self, frame: &mut Frame, state: &AppState);

    /// Handle keyboard input, returns true if the event was consumed
    fn handle_input(&mut self, key: KeyEvent, state: &mut AppState) -> Result<bool>;
}
