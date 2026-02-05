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

/// Ephemeral UI state that resets on navigation.
///
/// # Selection State Architecture (Hybrid Pattern)
///
/// This app uses a hybrid approach for selection state:
///
/// - **UiState** holds `selected_index` for cross-screen communication (e.g., passing
///   the selected section index from DeepReview to PseudocodeReview via `goto()`).
///
/// - **Individual screens** own their own selection state (e.g., `TriageScreen.list_state`,
///   `DeepReviewScreen.current_review_index`) because ratatui's `ListState` is widget-specific
///   and must be passed to `render_stateful_widget()`.
///
/// This hybrid exists due to framework constraints: ratatui widgets require their own state
/// objects, but we also need a shared location for screen transitions. The pattern works
/// because screens are responsible for their own rendering and navigation logic, while
/// UiState facilitates handoffs between screens.
#[derive(Debug, Default)]
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

    /// Section ID to split (set when user presses 's' in deep review)
    pub needs_split: Option<String>,
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

        // Restore draft hypothesis when entering PseudocodeReview
        if matches!(screen, Screen::PseudocodeReview) {
            if let Some(draft) = &self.session.draft_hypothesis {
                self.ui.input_text = draft.clone();
                self.ui.cursor_position = self.ui.input_text.len();
            }
        }
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
    #[allow(dead_code)] // Reserved for future direct section mutation
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
        let session = Session::new("test".to_string(), "abc123".to_string());
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
