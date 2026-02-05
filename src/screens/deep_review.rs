use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Wrap};

use super::{
    handle_error_state_input, handle_scroll_input, highlight_diff_lines, render_error_panel,
    render_footer_hints, wrapped_height, ErrorInputResult, ScreenTrait,
};
use crate::error::Result;
use crate::models::{AppState, Screen};

/// Deep review screen for reviewing sections marked as shaky or lost
pub struct DeepReviewScreen {
    /// Index into sections needing review
    current_review_index: usize,
}

impl DeepReviewScreen {
    pub fn new() -> Self {
        Self {
            current_review_index: 0,
        }
    }

    /// Get all reviewable sections (shaky or lost), regardless of review status.
    ///
    /// Returns `(original_index, section)` pairs where `original_index` is the section's
    /// position in `session.sections`, not the filtered list. This index is needed for
    /// correct updates when the user selects a section.
    fn get_reviewable_sections<'a>(
        &self,
        state: &'a AppState,
    ) -> Vec<(usize, &'a crate::models::Section)> {
        state
            .session
            .sections
            .iter()
            .enumerate()
            .filter(|(_, s)| s.needs_review())
            .collect()
    }

    /// Get total count of sections that need review (including reviewed ones)
    fn total_needing_review(&self, state: &AppState) -> usize {
        state
            .session
            .sections
            .iter()
            .filter(|s| s.needs_review())
            .count()
    }

    /// Get count of reviewed sections
    fn reviewed_count(&self, state: &AppState) -> usize {
        state
            .session
            .sections
            .iter()
            .filter(|s| s.needs_review() && s.is_reviewed())
            .count()
    }
}

impl Default for DeepReviewScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenTrait for DeepReviewScreen {
    fn render(&self, frame: &mut Frame, state: &AppState) {
        let area = frame.area();

        // Handle error state
        if let Some(error) = &state.ui.error {
            self.render_error(frame, area, error);
            return;
        }

        // Get all reviewable sections (shaky/lost)
        let reviewable = self.get_reviewable_sections(state);

        // Handle empty case (no sections need review)
        if reviewable.is_empty() {
            self.render_empty(frame, area);
            return;
        }

        // Check if all reviews are complete
        let all_reviewed = reviewable.iter().all(|(_, s)| s.is_reviewed());
        if all_reviewed {
            self.render_completed(frame, area, state);
            return;
        }

        // Compute dynamic description height
        let idx = self
            .current_review_index
            .min(reviewable.len().saturating_sub(1));
        let description = reviewable
            .get(idx)
            .map(|(_, s)| s.description.as_str())
            .unwrap_or("");
        let inner_width = area.width.saturating_sub(2);
        let text_lines = wrapped_height(description, inner_width);
        let desc_height = (text_lines + 2) // +2 for borders
            .max(3)                         // at least 1 line + borders
            .min(area.height / 2);          // cap at half screen

        // Main layout
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),           // Progress bar
                Constraint::Length(desc_height), // Section header (dynamic)
                Constraint::Min(0),              // Content area
                Constraint::Length(2),            // Footer hints
            ])
            .split(area);

        self.render_progress(frame, layout[0], state);
        self.render_section_header(frame, layout[1], state, &reviewable);
        self.render_content(frame, layout[2], state, &reviewable);
        self.render_footer(frame, layout[3], state, &reviewable);
    }

    fn handle_input(&mut self, key: KeyEvent, state: &mut AppState) -> Result<bool> {
        // Handle error state using shared helper + screen-specific 's' to skip
        if state.ui.error.is_some() {
            return match handle_error_state_input(&key) {
                ErrorInputResult::Quit => {
                    state.quit();
                    Ok(true)
                }
                ErrorInputResult::Retry => {
                    state.ui.clear_error();
                    Ok(true)
                }
                ErrorInputResult::NotHandled => {
                    // Screen-specific: 's' to skip current section
                    if key.code == KeyCode::Char('s') {
                        self.advance_to_next(state);
                        state.ui.clear_error();
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
            };
        }

        // Get all reviewable sections
        let reviewable = self.get_reviewable_sections(state);

        // Check if all reviews are complete
        let all_reviewed = !reviewable.is_empty() && reviewable.iter().all(|(_, s)| s.is_reviewed());
        if all_reviewed {
            return match key.code {
                KeyCode::Char('q') => {
                    state.quit();
                    Ok(true)
                }
                KeyCode::Enter => {
                    state.goto(Screen::Summary);
                    Ok(true)
                }
                _ => Ok(false),
            };
        }

        match key.code {
            KeyCode::Char('q') => {
                state.quit();
                Ok(true)
            }
            // Navigate to next section
            KeyCode::Char('n') => {
                self.advance_to_next(state);
                Ok(true)
            }
            // Navigate to previous section
            KeyCode::Char('p') => {
                self.go_to_previous(state);
                Ok(true)
            }
            // Enter to review current section
            KeyCode::Enter => {
                if !reviewable.is_empty() {
                    let clamped_index = self.current_review_index.min(reviewable.len() - 1);
                    if let Some(&(section_idx, _)) = reviewable.get(clamped_index) {
                        // Note: goto() resets UiState, so set selected_index after goto()
                        state.goto(Screen::PseudocodeReview);
                        state.ui.selected_index = section_idx;
                    }
                }
                Ok(true)
            }
            // Back to triage
            KeyCode::Esc => {
                state.goto(Screen::Triage);
                Ok(true)
            }
            // Scroll code preview (for large diffs)
            _ => {
                if let Some(consumed) = handle_scroll_input(&key, &mut state.ui.scroll_offset) {
                    Ok(consumed)
                } else {
                    Ok(false)
                }
            }
        }
    }
}

impl DeepReviewScreen {
    fn render_progress(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let total = self.total_needing_review(state);
        let reviewed = self.reviewed_count(state);
        let skipped = state
            .session
            .sections
            .iter()
            .filter(|s| matches!(s.tag, crate::models::Tag::GotIt))
            .count();

        let ratio = if total > 0 {
            reviewed as f64 / total as f64
        } else {
            0.0
        };

        let label = format!(
            "Reviewing {} of {} sections (skipped {} 'got it')",
            reviewed + 1,
            total,
            skipped
        );

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(Style::default().fg(Color::Cyan))
            .ratio(ratio)
            .label(label);

        frame.render_widget(gauge, area);
    }

    fn render_section_header(
        &self,
        frame: &mut Frame,
        area: Rect,
        _state: &AppState,
        needs_review: &[(usize, &crate::models::Section)],
    ) {
        let idx = self
            .current_review_index
            .min(needs_review.len().saturating_sub(1));

        let (title, description) = if let Some((_, section)) = needs_review.get(idx) {
            (
                format!("{} ({})", section.title, section.tag.label()),
                section.description.as_str(),
            )
        } else {
            ("No section".to_string(), "")
        };

        let header = Paragraph::new(description)
            .block(Block::default().borders(Borders::ALL).title(title))
            .wrap(Wrap { trim: true });

        frame.render_widget(header, area);
    }

    fn render_content(
        &self,
        frame: &mut Frame,
        area: Rect,
        state: &AppState,
        needs_review: &[(usize, &crate::models::Section)],
    ) {
        let idx = self
            .current_review_index
            .min(needs_review.len().saturating_sub(1));
        let section = needs_review.get(idx).map(|(_, s)| *s);

        if state.ui.ai_loading {
            // Waiting for AI state - show spinner overlay
            let text = "Waiting for AI response...\n\nYou can navigate away with 'n'/'p' keys.";
            let paragraph = Paragraph::new(text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Processing")
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
        } else if let Some(section) = section {
            if section.is_reviewed() {
                // Already reviewed — show assessment summary
                self.render_assessment_summary(frame, area, section);
            } else {
                // Not yet reviewed — show code preview
                let code_lines: Vec<Line> = highlight_diff_lines(&section.code_blocks);

                let paragraph = Paragraph::new(code_lines)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Code - Press Enter to review"),
                    )
                    .scroll((state.ui.scroll_offset as u16, 0));

                frame.render_widget(paragraph, area);
            }
        } else {
            let paragraph = Paragraph::new("No section selected")
                .block(Block::default().borders(Borders::ALL).title("Code"))
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
        }
    }

    fn render_footer(
        &self,
        frame: &mut Frame,
        area: Rect,
        _state: &AppState,
        reviewable: &[(usize, &crate::models::Section)],
    ) {
        let idx = self
            .current_review_index
            .min(reviewable.len().saturating_sub(1));
        let is_reviewed = reviewable
            .get(idx)
            .is_some_and(|(_, s)| s.is_reviewed());

        let hints = if is_reviewed {
            "n: next | p: previous | Enter: view assessment | Esc: back | q: quit (saved)"
        } else {
            "n: next | p: previous | Enter: review | Esc: back | q: quit (saved)"
        };
        render_footer_hints(frame, area, hints);
    }

    fn render_assessment_summary(
        &self,
        frame: &mut Frame,
        area: Rect,
        section: &crate::models::Section,
    ) {
        if let Some(assessment) = &section.assessment {
            let mut lines: Vec<Line> = Vec::new();

            if !assessment.correct.is_empty() {
                lines.push(Line::from(Span::styled(
                    "Correct:",
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                )));
                for item in &assessment.correct {
                    lines.push(Line::from(Span::styled(
                        format!("  - {}", item),
                        Style::default().fg(Color::Green),
                    )));
                }
                lines.push(Line::from(""));
            }

            if !assessment.diverges.is_empty() {
                lines.push(Line::from(Span::styled(
                    "Diverges:",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )));
                for item in &assessment.diverges {
                    lines.push(Line::from(Span::styled(
                        format!("  - {}", item),
                        Style::default().fg(Color::Yellow),
                    )));
                }
                lines.push(Line::from(""));
            }

            if !assessment.missed.is_empty() {
                lines.push(Line::from(Span::styled(
                    "Missed:",
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
                )));
                for item in &assessment.missed {
                    lines.push(Line::from(Span::styled(
                        format!("  - {}", item),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }

            let paragraph = Paragraph::new(lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Assessment (reviewed)")
                        .border_style(Style::default().fg(Color::Green)),
                )
                .scroll((0, 0));

            frame.render_widget(paragraph, area);
        }
    }

    fn render_completed(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let total = self.total_needing_review(state);
        let text = format!(
            "All {} sections reviewed!\n\nPress Enter to view summary.",
            total
        );

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Deep Review")
                    .border_style(Style::default().fg(Color::Green)),
            )
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Green));

        frame.render_widget(paragraph, area);
    }

    fn render_empty(&self, frame: &mut Frame, area: Rect) {
        let text = "No sections need deep review.\n\nPress Esc to go back to triage.";
        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Deep Review"))
            .alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
    }

    fn render_error(&self, frame: &mut Frame, area: Rect, error: &str) {
        render_error_panel(
            frame,
            area,
            "Deep Review",
            error,
            "Press 'r' to retry, 's' to skip, or 'q' to quit",
        );
    }

    fn advance_to_next(&mut self, state: &AppState) {
        let needs_review = self.get_reviewable_sections(state);
        if !needs_review.is_empty() {
            self.current_review_index = (self.current_review_index + 1) % needs_review.len();
        }
    }

    fn go_to_previous(&mut self, state: &AppState) {
        let needs_review = self.get_reviewable_sections(state);
        if !needs_review.is_empty() {
            if self.current_review_index == 0 {
                self.current_review_index = needs_review.len() - 1;
            } else {
                self.current_review_index -= 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Assessment, Section, Session, Tag};

    fn create_test_state() -> AppState {
        let mut session = Session::new("test".to_string(), "abc123".to_string());
        session.sections = vec![
            Section::new(
                "s1".to_string(),
                "Section 1".to_string(),
                "Description 1".to_string(),
                "code".to_string(),
            ),
            Section::new(
                "s2".to_string(),
                "Section 2".to_string(),
                "Description 2".to_string(),
                "code".to_string(),
            ),
            Section::new(
                "s3".to_string(),
                "Section 3".to_string(),
                "Description 3".to_string(),
                "code".to_string(),
            ),
        ];
        // Tag section 1 as got it, sections 2 and 3 need review
        session.sections[0].tag = Tag::GotIt;
        session.sections[1].tag = Tag::Shaky;
        session.sections[2].tag = Tag::Lost;
        AppState::new(session)
    }

    #[test]
    fn test_deep_review_screen_new() {
        let screen = DeepReviewScreen::new();
        assert_eq!(screen.current_review_index, 0);
    }

    #[test]
    fn test_deep_review_sections_needing_review() {
        let screen = DeepReviewScreen::new();
        let state = create_test_state();

        let needs_review = screen.get_reviewable_sections(&state);
        assert_eq!(needs_review.len(), 2);
        assert_eq!(needs_review[0].1.title, "Section 2");
        assert_eq!(needs_review[1].1.title, "Section 3");
    }

    #[test]
    fn test_deep_review_navigation() {
        let mut screen = DeepReviewScreen::new();
        let state = create_test_state();

        screen.advance_to_next(&state);
        assert_eq!(screen.current_review_index, 1);

        screen.advance_to_next(&state);
        assert_eq!(screen.current_review_index, 0); // Wraps around

        screen.go_to_previous(&state);
        assert_eq!(screen.current_review_index, 1);
    }

    #[test]
    fn test_deep_review_handle_quit() {
        let mut screen = DeepReviewScreen::new();
        let mut state = create_test_state();

        let key = KeyEvent::new(
            crossterm::event::KeyCode::Char('q'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key, &mut state).unwrap();
        assert!(state.should_quit);
    }

    #[test]
    fn test_deep_review_enter_goes_to_pseudocode() {
        let mut screen = DeepReviewScreen::new();
        let mut state = create_test_state();

        let key = KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key, &mut state).unwrap();
        assert_eq!(state.screen, Screen::PseudocodeReview);
    }

    #[test]
    fn test_deep_review_esc_goes_back() {
        let mut screen = DeepReviewScreen::new();
        let mut state = create_test_state();

        let key = KeyEvent::new(
            crossterm::event::KeyCode::Esc,
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key, &mut state).unwrap();
        assert_eq!(state.screen, Screen::Triage);
    }

    #[test]
    fn test_deep_review_completed_state() {
        let mut screen = DeepReviewScreen::new();
        let mut state = create_test_state();

        // Mark all review sections as reviewed
        state.session.sections[1].assessment = Some(Assessment {
            correct: vec!["Good".to_string()],
            diverges: vec![],
            missed: vec![],
        });
        state.session.sections[2].assessment = Some(Assessment {
            correct: vec!["Good".to_string()],
            diverges: vec![],
            missed: vec![],
        });

        let reviewable = screen.get_reviewable_sections(&state);
        assert_eq!(reviewable.len(), 2); // Still 2 sections, but all reviewed
        assert!(reviewable.iter().all(|(_, s)| s.is_reviewed()));
        assert_eq!(screen.reviewed_count(&state), 2);

        // Enter should go to summary
        let key = KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key, &mut state).unwrap();
        assert_eq!(state.screen, Screen::Summary);
    }

    #[test]
    fn test_deep_review_error_skip() {
        let mut screen = DeepReviewScreen::new();
        let mut state = create_test_state();
        state.ui.set_error("Test error");

        let key_s = KeyEvent::new(
            crossterm::event::KeyCode::Char('s'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_s, &mut state).unwrap();

        assert!(state.ui.error.is_none());
        assert_eq!(screen.current_review_index, 1); // Advanced to next
    }

    #[test]
    fn test_deep_review_waiting_ai_state() {
        let mut screen = DeepReviewScreen::new();
        let mut state = create_test_state();
        state.ui.ai_loading = true;

        // Navigation should still work during AI loading
        let key_n = KeyEvent::new(
            crossterm::event::KeyCode::Char('n'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_n, &mut state).unwrap();
        assert_eq!(screen.current_review_index, 1);

        let key_p = KeyEvent::new(
            crossterm::event::KeyCode::Char('p'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_p, &mut state).unwrap();
        assert_eq!(screen.current_review_index, 0);
    }

    #[test]
    fn test_deep_review_error_retry() {
        let mut screen = DeepReviewScreen::new();
        let mut state = create_test_state();
        state.ui.set_error("Test error");

        let key_r = KeyEvent::new(
            crossterm::event::KeyCode::Char('r'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_r, &mut state).unwrap();

        assert!(state.ui.error.is_none());
        // Index should not change on retry
        assert_eq!(screen.current_review_index, 0);
    }

    #[test]
    fn test_deep_review_out_of_bounds_index_clamps_correctly() {
        let mut screen = DeepReviewScreen::new();
        let mut state = create_test_state();

        // needs_review has 2 sections (indices 0 and 1)
        let needs_review = screen.get_reviewable_sections(&state);
        assert_eq!(needs_review.len(), 2);

        // Set current_review_index to out-of-bounds value (simulating sections being removed)
        screen.current_review_index = 10;

        // advance_to_next should wrap around correctly using modulo
        screen.advance_to_next(&state);
        // (10 + 1) % 2 = 1
        assert_eq!(screen.current_review_index, 1);

        // Reset to out-of-bounds
        screen.current_review_index = 10;

        // go_to_previous should also handle out-of-bounds
        // Since 10 != 0, it decrements: 10 - 1 = 9
        screen.go_to_previous(&state);
        assert_eq!(screen.current_review_index, 9);

        // Now test that rendering clamps correctly by checking Enter navigates properly
        screen.current_review_index = 100;
        let key = KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        // This should not panic and should clamp to valid index
        screen.handle_input(key, &mut state).unwrap();
        assert_eq!(state.screen, Screen::PseudocodeReview);
        // selected_index should be set to a valid section index (clamped)
        // needs_review[1] is the last valid index, which maps to session.sections[2]
        assert_eq!(state.ui.selected_index, 2);
    }
}
