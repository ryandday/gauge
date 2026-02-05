// @task(P1-T6) Screen module definitions for app shell routing
// Comprehensive tests in each screen module for rendering, keybindings, state transitions
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
use ratatui::Frame;

use crate::error::Result;
use crate::models::AppState;

/// Trait that all screens must implement
pub trait ScreenTrait {
    /// Render the screen to the terminal
    fn render(&self, frame: &mut Frame, state: &AppState);

    /// Handle keyboard input, returns true if the event was consumed
    fn handle_input(&mut self, key: KeyEvent, state: &mut AppState) -> Result<bool>;
}
