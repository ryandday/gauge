use crossterm::event::KeyEvent;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders};
use throbber_widgets_tui::{Throbber, ThrobberState, BRAILLE_SIX};

use super::{handle_error_state_input, render_error_panel, ErrorInputResult, ScreenTrait};
use crate::error::Result;
use crate::models::AppState;

/// Loading screen shown while AI chunks the diff
pub struct LoadingScreen {
    throbber_state: ThrobberState,
}

impl LoadingScreen {
    pub fn new() -> Self {
        Self {
            throbber_state: ThrobberState::default(),
        }
    }

    /// Advance the spinner animation (called each render frame by App::main_loop)
    pub fn tick(&mut self) {
        self.throbber_state.calc_next();
    }
}

impl Default for LoadingScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenTrait for LoadingScreen {
    fn render(&self, frame: &mut Frame, state: &AppState) {
        let area = frame.area();

        // Create centered layout
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Length(5),
                Constraint::Percentage(40),
            ])
            .split(area);

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(50),
                Constraint::Percentage(25),
            ])
            .split(vertical[1]);

        let content_area = horizontal[1];

        if let Some(error) = &state.ui.error {
            render_error_panel(
                frame,
                content_area,
                "Gauge",
                error,
                "Press 'r' to retry or 'q' to quit\nIf this persists, check your input format.",
            );
        } else {
            // Loading state: show spinner with status text
            let throbber = Throbber::default()
                .throbber_set(BRAILLE_SIX)
                .label("Analyzing changes...")
                .style(Style::default().fg(Color::Cyan));

            let block = Block::default()
                .borders(Borders::ALL)
                .title("Gauge")
                .border_style(Style::default().fg(Color::Cyan));

            // Render block first, then throbber inside
            let inner = block.inner(content_area);
            frame.render_widget(block, content_area);

            // Center the throbber vertically within the inner area
            let throbber_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Length(1),
                    Constraint::Percentage(50),
                ])
                .split(inner)[1];

            frame.render_stateful_widget(throbber, throbber_area, &mut self.throbber_state.clone());
        }
    }

    fn handle_input(&mut self, key: KeyEvent, state: &mut AppState) -> Result<bool> {
        // Handle error state using shared helper
        if state.ui.error.is_some() {
            return match handle_error_state_input(&key) {
                ErrorInputResult::Quit => {
                    state.quit();
                    Ok(true)
                }
                ErrorInputResult::Retry => {
                    // Request retry - main loop will re-attempt chunking
                    state.ui.request_chunking_retry();
                    Ok(true)
                }
                ErrorInputResult::NotHandled => Ok(false),
            };
        }

        // Non-error state: only quit works
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('q') => {
                state.quit();
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Session;

    #[test]
    fn test_loading_screen_new() {
        let screen = LoadingScreen::new();
        // Verify throbber state is initialized with default index (0)
        assert_eq!(screen.throbber_state.index(), 0);
    }

    #[test]
    fn test_loading_screen_handle_quit() {
        let mut screen = LoadingScreen::new();
        let session = Session::new("test".to_string(), "abc123".to_string());
        let mut state = AppState::new(session);

        let key = KeyEvent::new(
            crossterm::event::KeyCode::Char('q'),
            crossterm::event::KeyModifiers::NONE,
        );
        let result = screen.handle_input(key, &mut state);

        assert!(result.is_ok());
        assert!(state.should_quit);
    }

    #[test]
    fn test_loading_screen_handle_retry() {
        let mut screen = LoadingScreen::new();
        let session = Session::new("test".to_string(), "abc123".to_string());
        let mut state = AppState::new(session);
        state.ui.set_error("Test error");

        let key = KeyEvent::new(
            crossterm::event::KeyCode::Char('r'),
            crossterm::event::KeyModifiers::NONE,
        );
        let result = screen.handle_input(key, &mut state);

        assert!(result.is_ok());
        // Error is cleared and retry is requested
        assert!(state.ui.error.is_none());
        assert!(state.ui.needs_chunking_retry);
    }

    #[test]
    fn test_loading_screen_retry_ignored_without_error() {
        let mut screen = LoadingScreen::new();
        let session = Session::new("test".to_string(), "abc123".to_string());
        let mut state = AppState::new(session);

        let key = KeyEvent::new(
            crossterm::event::KeyCode::Char('r'),
            crossterm::event::KeyModifiers::NONE,
        );
        let result = screen.handle_input(key, &mut state);

        assert!(result.is_ok());
        assert!(!result.unwrap()); // Event not consumed when no error
    }
}
