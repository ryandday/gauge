// @task(P1-T6) Create app shell: terminal setup, main event loop, screen routing
use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;
use ratatui::Terminal;

use crate::ai::{AiClient, AssessmentResult, ChunkingResult, ClaudeClient};
use crate::error::{AppError, Result};
use crate::models::{AppState, Screen, Session};
use crate::screens::{
    DeepReviewScreen, LoadingScreen, PseudocodeReviewScreen, ScreenTrait, SummaryScreen,
    TriageScreen,
};

/// Main application that manages the TUI
pub struct App {
    state: AppState,
    ai_client: ClaudeClient,
    loading_screen: LoadingScreen,
    triage_screen: TriageScreen,
    deep_review_screen: DeepReviewScreen,
    pseudocode_screen: PseudocodeReviewScreen,
    summary_screen: SummaryScreen,
}

impl App {
    pub fn new(session: Session) -> Self {
        Self {
            state: AppState::new(session),
            ai_client: ClaudeClient::new(),
            loading_screen: LoadingScreen::new(),
            triage_screen: TriageScreen::new(),
            deep_review_screen: DeepReviewScreen::new(),
            pseudocode_screen: PseudocodeReviewScreen::new(),
            summary_screen: SummaryScreen::new(),
        }
    }

    /// Get a reference to the current session
    pub fn session(&self) -> &Session {
        &self.state.session
    }

    /// Get a mutable reference to the current session
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.state.session
    }

    /// Get a reference to the app state
    #[allow(dead_code)] // Used in PHASE-3/4 testing
    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Get a mutable reference to the app state
    pub fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }

    /// Run the main application loop
    pub fn run(&mut self) -> Result<()> {
        let mut terminal = setup_terminal()?;

        let result = self.main_loop(&mut terminal);

        // Ensure terminal is restored even on error
        restore_terminal(&mut terminal)?;

        result
    }

    fn main_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        loop {
            // Handle retry requests before rendering
            self.handle_retries();

            // Render current screen
            terminal.draw(|frame| self.render(frame))?;

            // Poll for events with timeout (allows 4+ FPS for smooth UI)
            if event::poll(Duration::from_millis(250))? {
                if let Event::Key(key) = event::read()? {
                    // Handle global quit (Ctrl+C)
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        self.state.quit();
                    } else {
                        self.handle_input(key)?;
                    }
                }
            }

            // Check if we should quit
            if self.state.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Handle retry requests for AI operations
    fn handle_retries(&mut self) {
        // Handle chunking retry
        if self.state.ui.needs_chunking_retry {
            self.state.ui.needs_chunking_retry = false;
            self.retry_chunking();
        }

        // Handle assessment retry
        if self.state.ui.needs_assessment_retry {
            self.state.ui.needs_assessment_retry = false;
            self.retry_assessment();
        }
    }

    /// Retry chunking the diff with AI
    fn retry_chunking(&mut self) {
        match self.ai_client.chunk_diff(&self.state.session.diff_text) {
            ChunkingResult::Success(sections) => {
                if sections.is_empty() {
                    self.state
                        .ui
                        .set_error("AI returned no sections. The diff may be too small.");
                } else {
                    self.state.session.sections = sections;
                    self.state.goto(Screen::Triage);
                }
            }
            ChunkingResult::Error(e) => {
                self.state.ui.set_error(e.message);
            }
        }
    }

    /// Retry assessing the current hypothesis with AI
    fn retry_assessment(&mut self) {
        let selected_idx = self.state.ui.selected_index;
        let (code, hypothesis) = {
            if let Some(section) = self.state.session.sections.get(selected_idx) {
                let code = section.code.clone();
                let hypothesis = self.state.ui.input_text.clone();
                (code, hypothesis)
            } else {
                return;
            }
        };

        if hypothesis.is_empty() {
            return;
        }

        match self.ai_client.assess_hypothesis(&code, &hypothesis) {
            AssessmentResult::Success(assessment) => {
                if let Some(section) = self.state.session.sections.get_mut(selected_idx) {
                    section.hypothesis = Some(hypothesis);
                    section.assessment = Some(assessment);
                }
                // Clear draft hypothesis since it's been submitted
                self.state.session.draft_hypothesis = None;
                self.state.ui.input_text.clear();
                self.state.ui.cursor_position = 0;
            }
            AssessmentResult::Error(e) => {
                self.state.ui.set_error(e.message);
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        match self.state.screen {
            Screen::Loading => self.loading_screen.render(frame, &self.state),
            Screen::Triage => self.triage_screen.render(frame, &self.state),
            Screen::DeepReview => self.deep_review_screen.render(frame, &self.state),
            Screen::PseudocodeReview => self.pseudocode_screen.render(frame, &self.state),
            Screen::Summary => self.summary_screen.render(frame, &self.state),
        }
    }

    fn handle_input(&mut self, key: event::KeyEvent) -> Result<bool> {
        match self.state.screen {
            Screen::Loading => self.loading_screen.handle_input(key, &mut self.state),
            Screen::Triage => self.triage_screen.handle_input(key, &mut self.state),
            Screen::DeepReview => self.deep_review_screen.handle_input(key, &mut self.state),
            Screen::PseudocodeReview => self.pseudocode_screen.handle_input(key, &mut self.state),
            Screen::Summary => self.summary_screen.handle_input(key, &mut self.state),
        }
    }
}

/// Set up the terminal for TUI mode
fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()
        .map_err(|e| AppError::Terminal(format!("Failed to enable raw mode: {}", e)))?;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)
        .map_err(|e| AppError::Terminal(format!("Failed to enter alternate screen: {}", e)))?;

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)
        .map_err(|e| AppError::Terminal(format!("Failed to create terminal: {}", e)))?;

    Ok(terminal)
}

/// Restore the terminal to normal mode
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()
        .map_err(|e| AppError::Terminal(format!("Failed to disable raw mode: {}", e)))?;

    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .map_err(|e| AppError::Terminal(format!("Failed to leave alternate screen: {}", e)))?;

    terminal
        .show_cursor()
        .map_err(|e| AppError::Terminal(format!("Failed to show cursor: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ReviewStage;

    #[test]
    fn test_app_new() {
        let session = Session::new("test".to_string(), "".to_string());
        let app = App::new(session);

        assert_eq!(app.state().screen, Screen::Loading);
        assert!(!app.state().should_quit);
    }

    #[test]
    fn test_app_state_access() {
        let session = Session::new("test".to_string(), "diff".to_string());
        let mut app = App::new(session);

        assert_eq!(app.session().identifier, "test");
        assert_eq!(app.session().diff_text, "diff");

        app.session_mut().identifier = "modified".to_string();
        assert_eq!(app.session().identifier, "modified");
    }

    #[test]
    fn test_screen_transitions() {
        let session = Session::new("test".to_string(), "".to_string());
        let mut app = App::new(session);

        app.state_mut().goto(Screen::Triage);
        assert_eq!(app.state().screen, Screen::Triage);
        assert_eq!(app.state().session.stage, ReviewStage::Triage);

        app.state_mut().goto(Screen::Summary);
        assert_eq!(app.state().screen, Screen::Summary);
        assert_eq!(app.state().session.stage, ReviewStage::Summary);
    }
}
