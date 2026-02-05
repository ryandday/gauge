// PseudocodeReviewScreen: code panel (top 80%), text input (bottom 20%)
// PseudocodeReviewScreen AI response display: Correct/Diverges/Missed sections
// Implement hypothesis preservation on error and retry
use crossterm::event::KeyEvent;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::ScreenTrait;
use crate::error::Result;
use crate::models::{AppState, Screen};

/// Screen states for PseudocodeReview
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudocodeState {
    /// User is typing their hypothesis
    Input,
    /// Hypothesis submitted, waiting for AI response
    Submitted,
    /// Showing AI response
    Response,
    /// Error occurred during AI assessment
    Error,
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
        let section = state.current_section();
        if section.is_some() && section.unwrap().assessment.is_some() {
            self.render_response(frame, area, state);
            return;
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

        // Handle error state - hypothesis is preserved
        if state.ui.error.is_some() {
            return match key.code {
                KeyCode::Char('q') => {
                    state.quit();
                    Ok(true)
                }
                KeyCode::Char('r') => {
                    // Retry with preserved hypothesis
                    state.ui.request_assessment_retry();
                    self.state = PseudocodeState::Submitted;
                    Ok(true)
                }
                KeyCode::Esc => {
                    // Go back to deep review, hypothesis preserved in draft
                    state.goto(Screen::DeepReview);
                    Ok(true)
                }
                _ => Ok(false),
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
        let section = state.current_section();
        if section.is_some() && section.unwrap().assessment.is_some() {
            return match key.code {
                KeyCode::Char('q') => {
                    state.quit();
                    Ok(true)
                }
                KeyCode::Enter | KeyCode::Esc => {
                    // Return to deep review
                    state.goto(Screen::DeepReview);
                    Ok(true)
                }
                _ => Ok(false),
            };
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
                if state.ui.cursor_position > 0 {
                    state.ui.cursor_position -= 1;
                    state.ui.input_text.remove(state.ui.cursor_position);
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
        let (title, code) = match section {
            Some(s) => (s.title.as_str(), s.code.as_str()),
            None => ("No section", ""),
        };

        let code_lines: Vec<Line> = code
            .lines()
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
            .collect();

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
        let footer = Paragraph::new(hints)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(footer, layout[1]);
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
        let footer = Paragraph::new(hints)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(footer, layout[3]);
    }

    /// Submit the hypothesis for assessment via real AI
    fn submit_hypothesis(&mut self, state: &mut AppState) {
        // Request AI assessment - the app's main loop will handle the actual call
        state.ui.needs_assessment_retry = true;
        self.state = PseudocodeState::Submitted;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Section, Session, Tag};

    fn create_test_state() -> AppState {
        let mut session = Session::new("test".to_string(), "".to_string());
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
    fn test_pseudocode_response_navigation() {
        use crate::models::Assessment;

        let mut screen = PseudocodeReviewScreen::new();
        let mut state = create_test_state();

        // Simulate assessment completed (app would do this after AI call)
        state.session.sections[0].hypothesis = Some("hypothesis".to_string());
        state.session.sections[0].assessment = Some(Assessment {
            correct: vec!["Good point".to_string()],
            diverges: vec![],
            missed: vec![],
        });

        // In response mode (has assessment), Enter should go back to deep review
        let key_enter = KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_enter, &mut state).unwrap();
        assert_eq!(state.screen, Screen::DeepReview);
    }
}
