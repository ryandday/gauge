use crossterm::event::KeyEvent;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::{
    handle_error_state_input, highlight_diff_lines, render_footer_hints, ErrorInputResult,
    ScreenTrait,
};
use crate::error::Result;
use crate::models::{AppState, Screen};

/// Screen states for PseudocodeReview
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudocodeState {
    /// User is typing their hypothesis
    Input,
    /// Hypothesis submitted, waiting for AI response
    Submitted,
}

/// Screen for writing and submitting hypotheses about code sections
pub struct PseudocodeReviewScreen {
    state: PseudocodeState,
}

impl PseudocodeReviewScreen {
    pub fn new() -> Self {
        Self {
            state: PseudocodeState::Input,
        }
    }
}

impl Default for PseudocodeReviewScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenTrait for PseudocodeReviewScreen {
    fn render(&self, frame: &mut Frame, state: &AppState) {
        let area = frame.area();

        // Check for error state
        if let Some(error) = &state.ui.error {
            self.render_error(frame, area, state, error);
            return;
        }

        // Check if we have an AI response - this takes priority over submitted state
        if let Some(section) = state.current_section() {
            if section.assessment.is_some() {
                self.render_response(frame, area, state);
                return;
            }
        }

        // Check if waiting for AI response (submitted but no response yet)
        if matches!(self.state, PseudocodeState::Submitted) {
            self.render_waiting(frame, area, state);
            return;
        }

        // Main input state
        self.render_input(frame, area, state);
    }

    fn handle_input(&mut self, key: KeyEvent, state: &mut AppState) -> Result<bool> {
        use crossterm::event::KeyCode;

        // Reset internal state when returning to this screen after navigation.
        // This handles the case where user pressed Esc during Submitted state -
        // we need to restore Input mode since no AI response is pending.
        // Conditions: in Submitted state but no error, no assessment, and no pending retry.
        let section = state.current_section();
        let has_assessment = section.is_some_and(|s| s.assessment.is_some());
        if matches!(self.state, PseudocodeState::Submitted)
            && state.ui.error.is_none()
            && !has_assessment
            && !state.ui.needs_assessment_retry
        {
            self.state = PseudocodeState::Input;
        }

        // Handle error state using shared helper + screen-specific behavior
        if state.ui.error.is_some() {
            return match handle_error_state_input(&key) {
                ErrorInputResult::Quit => {
                    state.quit();
                    Ok(true)
                }
                ErrorInputResult::Retry => {
                    // Retry with preserved hypothesis
                    state.ui.request_assessment_retry();
                    self.state = PseudocodeState::Submitted;
                    Ok(true)
                }
                ErrorInputResult::NotHandled => {
                    // Screen-specific: Esc goes back with hypothesis preserved
                    if key.code == KeyCode::Esc {
                        state.goto(Screen::DeepReview);
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
            };
        }

        // Handle submitted state - waiting for AI
        if matches!(self.state, PseudocodeState::Submitted) {
            return match key.code {
                KeyCode::Char('q') => {
                    state.quit();
                    Ok(true)
                }
                _ => Ok(false),
            };
        }

        // Check if we're viewing AI response
        if let Some(section) = state.current_section() {
            if section.assessment.is_some() {
                return match key.code {
                    KeyCode::Char('q') => {
                        state.quit();
                        Ok(true)
                    }
                    KeyCode::Enter | KeyCode::Esc => {
                        // Auto-transition to Summary if all reviewable sections are done
                        let all_reviewed = state
                            .session
                            .sections
                            .iter()
                            .filter(|s| s.needs_review())
                            .all(|s| s.is_reviewed());
                        if all_reviewed {
                            state.goto(Screen::Summary);
                        } else {
                            state.goto(Screen::DeepReview);
                        }
                        Ok(true)
                    }
                    _ => Ok(false),
                };
            }
        }

        // Input mode handling
        match key.code {
            KeyCode::Char('q') if state.ui.input_text.is_empty() => {
                state.quit();
                Ok(true)
            }
            KeyCode::Char(c) => {
                state.ui.input_text.push(c);
                state.ui.cursor_position = state.ui.input_text.len();
                // Save draft to session
                state.session.draft_hypothesis = Some(state.ui.input_text.clone());
                Ok(true)
            }
            KeyCode::Backspace => {
                // Use pop() to safely handle multi-byte UTF-8 characters.
                // This correctly removes the last character regardless of byte size.
                if state.ui.input_text.pop().is_some() {
                    state.ui.cursor_position = state.ui.input_text.len();
                    state.session.draft_hypothesis = Some(state.ui.input_text.clone());
                }
                Ok(true)
            }
            KeyCode::Enter if !state.ui.input_text.is_empty() => {
                // Submit hypothesis - trigger AI assessment via app
                self.submit_hypothesis(state);
                Ok(true)
            }
            KeyCode::Esc => {
                // Confirm abandonment if there's text
                if state.ui.input_text.is_empty() {
                    state.goto(Screen::DeepReview);
                } else {
                    // Save draft and go back
                    state.session.draft_hypothesis = Some(state.ui.input_text.clone());
                    state.goto(Screen::DeepReview);
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

impl PseudocodeReviewScreen {
    fn render_input(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        // Layout: 75% code at top, 25% input at bottom
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
            .split(area);

        self.render_code_panel(frame, layout[0], state);
        self.render_input_panel(frame, layout[1], state);
    }

    fn render_response(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        // Layout: 50% code, 50% response
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        self.render_code_panel(frame, layout[0], state);
        self.render_assessment_panel(frame, layout[1], state);
    }

    fn render_error(&self, frame: &mut Frame, area: Rect, state: &AppState, error: &str) {
        // Layout: code, preserved input, error message
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(30),
                Constraint::Percentage(20),
            ])
            .split(area);

        self.render_code_panel(frame, layout[0], state);

        // Show preserved hypothesis
        let input_block = Block::default()
            .borders(Borders::ALL)
            .title("Your hypothesis (preserved)")
            .border_style(Style::default().fg(Color::Yellow));

        let input_text = &state.ui.input_text;
        let input = Paragraph::new(input_text.as_str())
            .block(input_block)
            .wrap(Wrap { trim: true });
        frame.render_widget(input, layout[1]);

        // Error message
        let error_text = format!(
            "Error: {}\n\nPress 'r' to retry, Esc to go back (hypothesis saved)",
            error
        );
        let error_para = Paragraph::new(error_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Error")
                    .border_style(Style::default().fg(Color::Red)),
            )
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        frame.render_widget(error_para, layout[2]);
    }

    fn render_waiting(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        // Layout: 60% code, 40% waiting message
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        self.render_code_panel(frame, layout[0], state);

        // Show hypothesis and waiting message
        let waiting_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(layout[1]);

        // Show the submitted hypothesis
        let hypothesis = Paragraph::new(state.ui.input_text.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Your hypothesis (submitted)")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(hypothesis, waiting_layout[0]);

        // Waiting message
        let waiting_text =
            "Analyzing your hypothesis...\n\nThe AI is evaluating your understanding.";
        let waiting = Paragraph::new(waiting_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Processing")
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(waiting, waiting_layout[1]);
    }

    fn render_code_panel(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let section = state.current_section();
        let title = match section {
            Some(s) => s.title.as_str(),
            None => "No section",
        };
        let empty_blocks = Vec::new();
        let blocks = match section {
            Some(s) => &s.code_blocks,
            None => &empty_blocks,
        };

        let code_lines: Vec<Line> = highlight_diff_lines(blocks);

        let code_para = Paragraph::new(code_lines)
            .block(Block::default().borders(Borders::ALL).title(title))
            .scroll((state.ui.scroll_offset as u16, 0));

        frame.render_widget(code_para, area);
    }

    fn render_input_panel(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(2)])
            .split(area);

        // Input area
        let input_block = Block::default()
            .borders(Borders::ALL)
            .title("Describe what this code does in your own words...")
            .border_style(Style::default().fg(Color::Cyan));

        let input_text = if state.ui.input_text.is_empty() {
            // Show cursor indicator
            "_".to_string()
        } else {
            format!("{}_", &state.ui.input_text)
        };

        let input = Paragraph::new(input_text)
            .block(input_block)
            .wrap(Wrap { trim: true });
        frame.render_widget(input, layout[0]);

        // Hints
        let hints = "Enter: submit hypothesis | Esc: back (saves draft)";
        render_footer_hints(frame, layout[1], hints);
    }

    fn render_assessment_panel(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let section = state.current_section();
        let assessment = section.and_then(|s| s.assessment.as_ref());

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(30),
                Constraint::Percentage(30),
                Constraint::Length(2),
            ])
            .split(area);

        if let Some(assessment) = assessment {
            // Correct section (green)
            let correct_items = assessment.correct.join("\n- ");
            let correct_text = if correct_items.is_empty() {
                "(none)".to_string()
            } else {
                format!("- {}", correct_items)
            };
            let correct_para = Paragraph::new(correct_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Correct")
                        .border_style(Style::default().fg(Color::Green)),
                )
                .style(Style::default().fg(Color::Green))
                .wrap(Wrap { trim: true });
            frame.render_widget(correct_para, layout[0]);

            // Diverges section (yellow)
            let diverges_items = assessment.diverges.join("\n- ");
            let diverges_text = if diverges_items.is_empty() {
                "(none)".to_string()
            } else {
                format!("- {}", diverges_items)
            };
            let diverges_para = Paragraph::new(diverges_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Diverges")
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .style(Style::default().fg(Color::Yellow))
                .wrap(Wrap { trim: true });
            frame.render_widget(diverges_para, layout[1]);

            // Missed section (dim gray)
            let missed_items = assessment.missed.join("\n- ");
            let missed_text = if missed_items.is_empty() {
                "(none)".to_string()
            } else {
                format!("- {}", missed_items)
            };
            let missed_para = Paragraph::new(missed_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Missed")
                        .border_style(Style::default().fg(Color::DarkGray)),
                )
                .style(Style::default().fg(Color::DarkGray))
                .wrap(Wrap { trim: true });
            frame.render_widget(missed_para, layout[2]);
        } else {
            let no_assessment = Paragraph::new("No assessment available")
                .block(Block::default().borders(Borders::ALL).title("Assessment"))
                .alignment(Alignment::Center);
            frame.render_widget(no_assessment, layout[0]);
        }

        // Footer hints
        let hints = "Enter: continue | Esc: back to deep review";
        render_footer_hints(frame, layout[3], hints);
    }

    /// Submit the hypothesis for assessment via real AI.
    ///
    /// Uses the retry flag to signal app's main loop to perform assessment.
    /// The 'retry' naming is a misnomer - this flag triggers both initial
    /// submissions and actual retries. The main loop handles both cases identically.
    fn submit_hypothesis(&mut self, state: &mut AppState) {
        state.ui.needs_assessment_retry = true;
        self.state = PseudocodeState::Submitted;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Section, Session, Tag};

    fn create_test_state() -> AppState {
        let mut session = Session::new("test".to_string(), "abc123".to_string());
        session.sections = vec![Section::new(
            "s1".to_string(),
            "Test Section".to_string(),
            "Description".to_string(),
            "+added line".to_string(),
        )];
        session.sections[0].tag = Tag::Shaky;
        let mut state = AppState::new(session);
        state.ui.selected_index = 0;
        state
    }

    #[test]
    fn test_pseudocode_screen_new() {
        let screen = PseudocodeReviewScreen::new();
        assert_eq!(screen.state, PseudocodeState::Input);
    }

    #[test]
    fn test_pseudocode_screen_typing() {
        let mut screen = PseudocodeReviewScreen::new();
        let mut state = create_test_state();

        // Type 'h'
        let key_h = KeyEvent::new(
            crossterm::event::KeyCode::Char('h'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_h, &mut state).unwrap();
        assert_eq!(state.ui.input_text, "h");

        // Type 'i'
        let key_i = KeyEvent::new(
            crossterm::event::KeyCode::Char('i'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_i, &mut state).unwrap();
        assert_eq!(state.ui.input_text, "hi");
        assert_eq!(state.session.draft_hypothesis, Some("hi".to_string()));
    }

    #[test]
    fn test_pseudocode_screen_backspace() {
        let mut screen = PseudocodeReviewScreen::new();
        let mut state = create_test_state();
        state.ui.input_text = "test".to_string();
        state.ui.cursor_position = 4;

        let key_bs = KeyEvent::new(
            crossterm::event::KeyCode::Backspace,
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_bs, &mut state).unwrap();
        assert_eq!(state.ui.input_text, "tes");
        assert_eq!(state.ui.cursor_position, 3);
    }

    #[test]
    fn test_pseudocode_screen_backspace_multibyte_utf8() {
        let mut screen = PseudocodeReviewScreen::new();
        let mut state = create_test_state();

        // Type multi-byte characters via handle_input to ensure cursor_position stays in sync
        let chars = ['h', 'e', 'l', 'l', 'o', '\u{4E2D}', '\u{6587}']; // "hello" + Chinese chars
        for c in chars {
            let key = KeyEvent::new(
                crossterm::event::KeyCode::Char(c),
                crossterm::event::KeyModifiers::NONE,
            );
            screen.handle_input(key, &mut state).unwrap();
        }
        // "hello" (5 bytes) + "中" (3 bytes) + "文" (3 bytes) = 11 bytes total
        assert_eq!(state.ui.input_text, "hello中文");
        assert_eq!(state.ui.cursor_position, state.ui.input_text.len()); // 11 bytes

        // Backspace should remove the last character ("文"), not panic
        let key_bs = KeyEvent::new(
            crossterm::event::KeyCode::Backspace,
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_bs, &mut state).unwrap();
        assert_eq!(state.ui.input_text, "hello中");
        assert_eq!(state.ui.cursor_position, state.ui.input_text.len()); // 8 bytes

        // Another backspace removes "中"
        screen.handle_input(key_bs, &mut state).unwrap();
        assert_eq!(state.ui.input_text, "hello");
        assert_eq!(state.ui.cursor_position, 5);
    }

    #[test]
    fn test_pseudocode_screen_submit() {
        let mut screen = PseudocodeReviewScreen::new();
        let mut state = create_test_state();
        state.ui.input_text = "My hypothesis".to_string();

        let key_enter = KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_enter, &mut state).unwrap();

        // Check screen state changed to Submitted (waiting for AI)
        assert_eq!(screen.state, PseudocodeState::Submitted);
        // Check retry flag set (app will perform assessment)
        assert!(state.ui.needs_assessment_retry);
        // Input text preserved for display during wait
        assert_eq!(state.ui.input_text, "My hypothesis");
    }

    #[test]
    fn test_pseudocode_screen_esc_saves_draft() {
        let mut screen = PseudocodeReviewScreen::new();
        let mut state = create_test_state();
        state.ui.input_text = "partial hypothesis".to_string();

        let key_esc = KeyEvent::new(
            crossterm::event::KeyCode::Esc,
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_esc, &mut state).unwrap();

        assert_eq!(
            state.session.draft_hypothesis,
            Some("partial hypothesis".to_string())
        );
        assert_eq!(state.screen, Screen::DeepReview);
    }

    #[test]
    fn test_pseudocode_screen_error_preserves_hypothesis() {
        let mut screen = PseudocodeReviewScreen::new();
        let mut state = create_test_state();
        state.ui.input_text = "my hypothesis".to_string();
        state.ui.set_error("AI error");

        // Try to type - should be blocked in error state
        let key_a = KeyEvent::new(
            crossterm::event::KeyCode::Char('a'),
            crossterm::event::KeyModifiers::NONE,
        );
        let result = screen.handle_input(key_a, &mut state).unwrap();
        assert!(!result); // Not consumed

        // Retry should clear error, preserve input, request retry
        let key_r = KeyEvent::new(
            crossterm::event::KeyCode::Char('r'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_r, &mut state).unwrap();

        assert!(state.ui.error.is_none());
        assert_eq!(state.ui.input_text, "my hypothesis");
        assert!(state.ui.needs_assessment_retry);
        assert_eq!(screen.state, PseudocodeState::Submitted);
    }

    #[test]
    fn test_pseudocode_screen_empty_enter_ignored() {
        let mut screen = PseudocodeReviewScreen::new();
        let mut state = create_test_state();
        // Empty input text

        let key_enter = KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        let result = screen.handle_input(key_enter, &mut state).unwrap();
        assert!(!result); // Not consumed
        assert!(state.session.sections[0].assessment.is_none());
    }

    #[test]
    fn test_pseudocode_screen_quit_only_when_empty() {
        let mut screen = PseudocodeReviewScreen::new();
        let mut state = create_test_state();

        // Quit should work when empty
        let key_q = KeyEvent::new(
            crossterm::event::KeyCode::Char('q'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_q, &mut state).unwrap();
        assert!(state.should_quit);

        // Reset
        state.should_quit = false;
        state.ui.input_text = "typing".to_string();

        // Quit should add 'q' to input when not empty
        screen.handle_input(key_q, &mut state).unwrap();
        assert!(!state.should_quit);
        assert_eq!(state.ui.input_text, "typingq");
    }

    #[test]
    fn test_pseudocode_response_auto_transitions_to_summary_when_all_reviewed() {
        use crate::models::Assessment;

        let mut screen = PseudocodeReviewScreen::new();
        let mut state = create_test_state();

        // Only one reviewable section, now reviewed → all done
        state.session.sections[0].hypothesis = Some("hypothesis".to_string());
        state.session.sections[0].assessment = Some(Assessment {
            correct: vec!["Good point".to_string()],
            diverges: vec![],
            missed: vec![],
        });

        let key_enter = KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_enter, &mut state).unwrap();
        assert_eq!(state.screen, Screen::Summary);
    }

    #[test]
    fn test_pseudocode_response_returns_to_deep_review_when_more_to_review() {
        use crate::models::Assessment;

        let mut screen = PseudocodeReviewScreen::new();
        let mut state = create_test_state();

        // Add a second reviewable section that is NOT reviewed
        state.session.sections.push(Section::new(
            "s2".to_string(),
            "Section 2".to_string(),
            "Description 2".to_string(),
            "+more code".to_string(),
        ));
        state.session.sections[1].tag = Tag::Lost;

        // First section is reviewed
        state.session.sections[0].hypothesis = Some("hypothesis".to_string());
        state.session.sections[0].assessment = Some(Assessment {
            correct: vec!["Good point".to_string()],
            diverges: vec![],
            missed: vec![],
        });

        let key_enter = KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_enter, &mut state).unwrap();
        assert_eq!(state.screen, Screen::DeepReview);
    }
}
