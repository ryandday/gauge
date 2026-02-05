// @task(P1-T4) App state and screen routing models
// Screen navigation and backstack behavior (goto method, stage transitions)
use serde::{Deserialize, Serialize};

use super::session::Session;

/// The current review stage (persistent)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStage {
    #[default]
    Loading,
    Triage,
    DeepReview,
    Summary,
}

/// Active screen (determines which view is rendered)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Loading,
    Triage,
    DeepReview,
    PseudocodeReview,
    Summary,
}

impl From<ReviewStage> for Screen {
    fn from(stage: ReviewStage) -> Self {
        match stage {
            ReviewStage::Loading => Screen::Loading,
            ReviewStage::Triage => Screen::Triage,
            ReviewStage::DeepReview => Screen::DeepReview,
            ReviewStage::Summary => Screen::Summary,
        }
    }
}

/// Ephemeral UI state that resets on navigation
#[derive(Debug, Default)]
#[allow(dead_code)] // Fields used in PHASE-3 screens
pub struct UiState {
    /// Selected section index in triage/deep review
    pub selected_index: usize,

    /// Scroll offset for section list
    pub scroll_offset: usize,

    /// Cursor position in text input
    pub cursor_position: usize,

    /// Current text being typed in hypothesis input
    pub input_text: String,

    /// Whether AI is currently processing
    pub ai_loading: bool,

    /// Error message to display (if any)
    pub error: Option<String>,

    /// Whether chunking needs to be retried (set when user presses 'r' on error)
    pub needs_chunking_retry: bool,

    /// Whether assessment needs to be retried (set when user presses 'r' on error)
    pub needs_assessment_retry: bool,
}

impl UiState {
    pub fn clear_error(&mut self) {
        self.error = None;
    }

    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.error = Some(msg.into());
    }

    /// Request retry for chunking operation
    pub fn request_chunking_retry(&mut self) {
        self.clear_error();
        self.needs_chunking_retry = true;
    }

    /// Request retry for assessment operation
    pub fn request_assessment_retry(&mut self) {
        self.clear_error();
        self.needs_assessment_retry = true;
    }
}

/// Complete application state
pub struct AppState {
    /// Persistent session data (saved to disk)
    pub session: Session,

    /// Current screen being displayed
    pub screen: Screen,

    /// Ephemeral UI state
    pub ui: UiState,

    /// Whether the app should quit
    pub should_quit: bool,
}

impl AppState {
    pub fn new(session: Session) -> Self {
        let screen = session.stage.into();
        Self {
            session,
            screen,
            ui: UiState::default(),
            should_quit: false,
        }
    }

    /// Transition to a new screen
    pub fn goto(&mut self, screen: Screen) {
        self.screen = screen;
        // Update persistent stage for relevant screens
        match screen {
            Screen::Loading => self.session.stage = ReviewStage::Loading,
            Screen::Triage => self.session.stage = ReviewStage::Triage,
            Screen::DeepReview | Screen::PseudocodeReview => {
                self.session.stage = ReviewStage::DeepReview
            }
            Screen::Summary => self.session.stage = ReviewStage::Summary,
        }
        // Reset ephemeral UI state on screen transition
        self.ui = UiState::default();
    }

    /// Request app quit
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Get the current section (if any) based on selected index
    pub fn current_section(&self) -> Option<&super::Section> {
        self.session.sections.get(self.ui.selected_index)
    }

    /// Get the current section mutably
    #[allow(dead_code)] // Used in PHASE-3 for tagging
    pub fn current_section_mut(&mut self) -> Option<&mut super::Section> {
        self.session.sections.get_mut(self.ui.selected_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_stage_to_screen() {
        assert_eq!(Screen::from(ReviewStage::Loading), Screen::Loading);
        assert_eq!(Screen::from(ReviewStage::Triage), Screen::Triage);
        assert_eq!(Screen::from(ReviewStage::DeepReview), Screen::DeepReview);
        assert_eq!(Screen::from(ReviewStage::Summary), Screen::Summary);
    }

    #[test]
    fn test_app_state_goto() {
        let session = Session::new("test".to_string(), "".to_string());
        let mut app = AppState::new(session);

        app.goto(Screen::Triage);
        assert_eq!(app.screen, Screen::Triage);
        assert_eq!(app.session.stage, ReviewStage::Triage);
    }

    #[test]
    fn test_ui_state_error() {
        let mut ui = UiState::default();
        assert!(ui.error.is_none());

        ui.set_error("Test error");
        assert_eq!(ui.error, Some("Test error".to_string()));

        ui.clear_error();
        assert!(ui.error.is_none());
    }
}
