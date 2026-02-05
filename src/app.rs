use std::io::{self, Stdout};
use std::process::{Command, Stdio};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;
use ratatui::Terminal;

use crate::ai::{AiClient, AssessmentResult, ClaudeClient};
use crate::error::{AppError, Result};
use crate::models::{AppState, Screen, Session};
use crate::screens::{
    DeepReviewScreen, LoadingScreen, PseudocodeReviewScreen, ScreenTrait, SummaryScreen,
    TriageScreen,
};
use crate::session::{load_session, save_session, SessionLoadResult};

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
    #[allow(dead_code)]
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.state.session
    }

    /// Get a reference to the app state (used in tests)
    #[allow(dead_code)]
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

        // Always attempt restore, but don't let restore failure mask main_loop error
        if let Err(restore_err) = restore_terminal(&mut terminal) {
            eprintln!("CRITICAL: Failed to restore terminal: {}", restore_err);
            eprintln!("Your terminal may be in a broken state. Run 'reset' to fix.");
            // If main_loop succeeded but restore failed, return the restore error
            // If main_loop failed, return that original error (user has been warned about terminal)
            if result.is_ok() {
                return Err(restore_err);
            }
        }

        result
    }

    fn main_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        loop {
            // Tick animations for loading screen
            if matches!(self.state.screen, Screen::Loading) {
                self.loading_screen.tick();
            }

            // Render current screen BEFORE handling retries so "waiting" screens
            // are visible during blocking AI calls
            terminal.draw(|frame| self.render(frame))?;

            // Handle retry requests after rendering (blocking AI call happens here)
            self.handle_retries();

            // Poll for events with timeout (allows ~4 FPS updates for responsive UI)
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
        // Handle assessment retry
        if self.state.ui.needs_assessment_retry {
            self.state.ui.needs_assessment_retry = false;
            self.retry_assessment();
        }

        // Handle section split
        if let Some(section_id) = self.state.ui.needs_split.take() {
            self.split_current_section(&section_id);
        }
    }

    /// Retry assessing the current hypothesis with AI
    fn retry_assessment(&mut self) {
        // Early return checks before any cloning
        if self.state.ui.input_text.is_empty() {
            return;
        }

        let selected_idx = self.state.ui.selected_index;
        let (code, hypothesis) = {
            if let Some(section) = self.state.session.sections.get(selected_idx) {
                (section.code(), self.state.ui.input_text.clone())
            } else {
                return;
            }
        };

        match self.ai_client.assess_hypothesis(&code, &hypothesis) {
            AssessmentResult::Success(assessment) => {
                // Only clear draft if it matches the submitted hypothesis (user may have started a new draft)
                if self.state.session.draft_hypothesis.as_ref() == Some(&hypothesis) {
                    self.state.session.draft_hypothesis = None;
                }
                if let Some(section) = self.state.session.sections.get_mut(selected_idx) {
                    section.hypothesis = Some(hypothesis);
                    section.assessment = Some(assessment);
                }
                self.state.ui.input_text.clear();
                self.state.ui.cursor_position = 0;
            }
            AssessmentResult::Error(e) => {
                self.state.ui.set_error(e.message);
            }
        }
    }

    /// Reload session from disk, replacing in-memory state
    fn reload_session(&mut self) -> Result<()> {
        let name = self.state.session.name.clone();
        match load_session(&name)? {
            SessionLoadResult::Loaded(session) => {
                self.state.session = session;
                Ok(())
            }
            SessionLoadResult::Corrupted { path, error } => Err(AppError::Session(format!(
                "Session file corrupted after split ({}): {}",
                path.display(),
                error
            ))),
            SessionLoadResult::NotFound => Err(AppError::Session(format!(
                "Session '{}' not found after split",
                name
            ))),
        }
    }

    /// Split the current section by shelling out to Claude CLI
    fn split_current_section(&mut self, section_id: &str) {
        // Save current session to disk so Claude can read it
        if let Err(e) = save_session(&self.state.session) {
            self.state.ui.set_error(format!("Failed to save session before split: {}", e));
            self.state.ui.ai_loading = false;
            return;
        }

        let session_name = self.state.session.name.clone();
        let prompt = build_split_prompt(&session_name, section_id);

        // Shell out to Claude CLI
        let result = Command::new("claude")
            .args(["--dangerously-skip-permissions", "-p", &prompt])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .and_then(|child| child.wait_with_output());

        match result {
            Ok(output) if output.status.success() => {
                // Reload session from disk (Claude modified it via gauge commands)
                match self.reload_session() {
                    Ok(()) => {
                        self.state.goto(Screen::Triage);
                    }
                    Err(e) => {
                        self.state
                            .ui
                            .set_error(format!("Failed to reload session after split: {}", e));
                    }
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                self.state
                    .ui
                    .set_error(format!("Split failed: {}", stderr.trim()));
            }
            Err(e) => {
                self.state
                    .ui
                    .set_error(format!("Failed to run Claude CLI: {}", e));
            }
        }

        self.state.ui.ai_loading = false;
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

/// Build the prompt for Claude to split a section using gauge CLI commands
fn build_split_prompt(session_name: &str, section_id: &str) -> String {
    format!(
        r#"You are splitting a code review section into smaller, more focused sections.

The active gauge session is "{session_name}". The section to split is "{section_id}".

First, run `gauge section show {section_id}` and `gauge code list {section_id}` to understand the section.
Then read the code blocks with `gauge code show {section_id} <code_id>` for each block.

Analyze the code and split it into 2-5 smaller sections, each covering a cohesive unit of functionality.

For each new section:
1. Create it: `gauge section add --title "<title>" --description "<description>"`
   (this prints the new section ID)
2. Move relevant code blocks to it: `gauge code move {section_id} <code_id> <new_section_id>`

After moving all code blocks out, delete the original empty section:
`gauge section delete {section_id}`

Important:
- Every code block must be moved to exactly one new section
- Do not leave any code blocks in the original section
- Delete the original section only after all blocks are moved
- Keep titles concise and descriptions informative"#,
        session_name = session_name,
        section_id = section_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ReviewStage;

    #[test]
    fn test_app_new() {
        let session = Session::new("test".to_string(), "abc123".to_string());
        let app = App::new(session);

        assert_eq!(app.state().screen, Screen::Loading);
        assert!(!app.state().should_quit);
    }

    #[test]
    fn test_app_state_access() {
        let session = Session::new("test".to_string(), "abc123".to_string());
        let mut app = App::new(session);

        assert_eq!(app.session().name, "test");
        assert_eq!(app.session().base_ref, "abc123");

        app.session_mut().name = "modified".to_string();
        assert_eq!(app.session().name, "modified");
    }

    #[test]
    fn test_screen_transitions() {
        let session = Session::new("test".to_string(), "abc123".to_string());
        let mut app = App::new(session);

        app.state_mut().goto(Screen::Triage);
        assert_eq!(app.state().screen, Screen::Triage);
        assert_eq!(app.state().session.stage, ReviewStage::Triage);

        app.state_mut().goto(Screen::Summary);
        assert_eq!(app.state().screen, Screen::Summary);
        assert_eq!(app.state().session.stage, ReviewStage::Summary);
    }
}
