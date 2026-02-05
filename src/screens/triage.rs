use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use super::{
    handle_error_state_input, highlight_diff_lines, render_error_panel, render_footer_hints,
    wrapped_height, ErrorInputResult, ScreenTrait,
};
use crate::error::Result;
use crate::models::{AppState, Screen, Tag};

/// Screen for triaging all sections before deep review.
/// See `UiState` documentation for the hybrid selection state pattern.
pub struct TriageScreen {
    list_state: ListState,
    code_focused: bool,
}

impl TriageScreen {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            list_state,
            code_focused: false,
        }
    }
}

impl Default for TriageScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenTrait for TriageScreen {
    fn render(&self, frame: &mut Frame, state: &AppState) {
        let area = frame.area();

        // Check for error state
        if let Some(error) = &state.ui.error {
            self.render_error(frame, area, error);
            return;
        }

        // Check for empty state
        if state.session.sections.is_empty() {
            self.render_empty(frame, area);
            return;
        }

        // Main layout: header, content, footer
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header with progress
                Constraint::Min(0),    // Main content
                Constraint::Length(2), // Footer hints
            ])
            .split(area);

        self.render_header(frame, layout[0], state);
        self.render_content(frame, layout[1], state);
        self.render_footer(frame, layout[2], state);
    }

    /// Handle keyboard input for the triage screen.
    ///
    /// # Error Handling Pattern
    /// All screens use `handle_error_state_input()` for common error keys (q, r), then add
    /// screen-specific error behavior. For example:
    /// - TriageScreen: standard q/r only
    /// - DeepReviewScreen: adds 's' to skip current section
    /// - PseudocodeReviewScreen: adds Esc to go back with hypothesis preserved
    fn handle_input(&mut self, key: KeyEvent, state: &mut AppState) -> Result<bool> {
        // Handle error state using shared helper
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
                ErrorInputResult::NotHandled => Ok(false),
            };
        }

        // Handle empty state
        if state.session.sections.is_empty() {
            if key.code == KeyCode::Char('q') {
                state.quit();
                return Ok(true);
            }
            return Ok(false);
        }

        match key.code {
            KeyCode::Char('q') => {
                state.quit();
                Ok(true)
            }
            _ if self.code_focused => self.handle_code_input(key, state),
            _ => self.handle_list_input(key, state),
        }
    }
}

impl TriageScreen {
    fn handle_list_input(&mut self, key: KeyEvent, state: &mut AppState) -> Result<bool> {
        match key.code {
            // Navigation
            KeyCode::Char('j') | KeyCode::Down => {
                self.select_next(state);
                Ok(true)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.select_previous(state);
                Ok(true)
            }
            // Focus code panel
            KeyCode::Enter | KeyCode::Char('l') => {
                self.code_focused = true;
                state.ui.scroll_offset = 0;
                Ok(true)
            }
            // Tagging (auto-advances; transitions when all tagged)
            KeyCode::Char('1') => {
                self.tag_and_advance(state, Tag::GotIt);
                Ok(true)
            }
            KeyCode::Char('2') => {
                self.tag_and_advance(state, Tag::Shaky);
                Ok(true)
            }
            KeyCode::Char('3') => {
                self.tag_and_advance(state, Tag::Lost);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn handle_code_input(&mut self, key: KeyEvent, state: &mut AppState) -> Result<bool> {
        match key.code {
            KeyCode::Char('h') => {
                self.code_focused = false;
                Ok(true)
            }
            KeyCode::Char('j') | KeyCode::Down => {
                state.ui.scroll_offset = state.ui.scroll_offset.saturating_add(1);
                Ok(true)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                state.ui.scroll_offset = state.ui.scroll_offset.saturating_sub(1);
                Ok(true)
            }
            KeyCode::Char('d') => {
                state.ui.scroll_offset = state.ui.scroll_offset.saturating_add(10);
                Ok(true)
            }
            KeyCode::Char('u') => {
                state.ui.scroll_offset = state.ui.scroll_offset.saturating_sub(10);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn render_header(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let counts = state.session.tag_counts();
        let all_tagged = state.session.all_tagged();

        let progress_text = format!("{}/{} sections tagged", counts.tagged(), counts.total());

        let style = if all_tagged {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Yellow)
        };

        let header = Paragraph::new(progress_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Triage")
                    .border_style(style),
            )
            .alignment(Alignment::Center)
            .style(style);

        frame.render_widget(header, area);
    }

    fn render_content(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        // Split content: 40% section list, 60% code preview
        let content_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        self.render_section_list(frame, content_layout[0], state);
        self.render_code_preview(frame, content_layout[1], state);
    }

    fn render_section_list(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let items: Vec<ListItem> = state
            .session
            .sections
            .iter()
            .map(|section| {
                let badge = section.tag.badge();
                let (badge_style, title_style) = match section.tag {
                    Tag::Untagged => (
                        Style::default().fg(Color::DarkGray),
                        Style::default().fg(Color::White),
                    ),
                    Tag::GotIt => (
                        Style::default().fg(Color::DarkGray),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Tag::Shaky => (
                        Style::default().fg(Color::Yellow),
                        Style::default().fg(Color::Yellow),
                    ),
                    Tag::Lost => (
                        Style::default().fg(Color::Red),
                        Style::default().fg(Color::Red),
                    ),
                };

                let line = Line::from(vec![
                    Span::styled(format!("[{}] ", badge), badge_style),
                    Span::styled(&section.title, title_style),
                ]);

                ListItem::new(line)
            })
            .collect();

        let border_style = if self.code_focused {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Sections")
                    .border_style(border_style),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut self.list_state.clone());
    }

    fn render_code_preview(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let selected_idx = self.list_state.selected().unwrap_or(0);
        let section = state.session.sections.get(selected_idx);

        let (title, description) = match section {
            Some(s) => (s.title.as_str(), s.description.as_str()),
            None => ("No section selected", ""),
        };

        // Split preview: description at top (dynamic height), code below
        let inner_width = area.width.saturating_sub(2); // subtract borders
        let text_lines = wrapped_height(description, inner_width);
        // +2 for title line and blank separator, +2 for borders
        let desc_height = (text_lines + 4)
            .max(5)                         // at least title + blank + 1 desc line + borders
            .min(area.height / 2);          // cap at half the panel
        let preview_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(desc_height), Constraint::Min(0)])
            .split(area);

        // Description panel: bold title line + description body
        let desc_text = vec![
            Line::from(Span::styled(
                title,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(description),
        ];
        let desc_paragraph = Paragraph::new(desc_text)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        frame.render_widget(desc_paragraph, preview_layout[0]);

        // Code preview panel with diff syntax highlighting
        let empty_blocks = Vec::new();
        let blocks = match section {
            Some(s) => &s.code_blocks,
            None => &empty_blocks,
        };
        let code_lines: Vec<Line> = highlight_diff_lines(blocks);

        let code_border_style = if self.code_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let code_paragraph = Paragraph::new(code_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Code")
                    .border_style(code_border_style),
            )
            .scroll((state.ui.scroll_offset as u16, 0));

        frame.render_widget(code_paragraph, preview_layout[1]);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect, _state: &AppState) {
        let hints = if self.code_focused {
            "j/k: scroll | u/d: half-page | h: back | q: quit"
        } else {
            "j/k: navigate | l/Enter: view code | 1-3: tag & next | q: quit"
        };

        render_footer_hints(frame, area, hints);
    }

    fn render_empty(&self, frame: &mut Frame, area: Rect) {
        let text = "No changes to review.\n\nThe diff is empty or no sections were created.\n\nPress 'q' to quit.";

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Triage"))
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    fn render_error(&self, frame: &mut Frame, area: Rect, error: &str) {
        render_error_panel(frame, area, "Triage", error, "Press 'r' to retry or 'q' to quit");
    }

    fn select_next(&mut self, state: &AppState) {
        let len = state.session.sections.len();
        if len == 0 {
            return;
        }

        let current = self.list_state.selected().unwrap_or(0);
        let next = if current >= len - 1 { 0 } else { current + 1 };
        self.list_state.select(Some(next));
    }

    fn select_previous(&mut self, state: &AppState) {
        let len = state.session.sections.len();
        if len == 0 {
            return;
        }

        let current = self.list_state.selected().unwrap_or(0);
        let prev = if current == 0 { len - 1 } else { current - 1 };
        self.list_state.select(Some(prev));
    }

    fn tag_and_advance(&mut self, state: &mut AppState, tag: Tag) {
        self.tag_current(state, tag);
        if state.session.all_tagged() {
            state.goto(Screen::DeepReview);
        } else {
            self.select_next(state);
            state.ui.scroll_offset = 0;
        }
    }

    fn tag_current(&mut self, state: &mut AppState, tag: Tag) {
        let selected = self.list_state.selected().unwrap_or(0);
        if let Some(section) = state.session.sections.get_mut(selected) {
            section.tag = tag;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Section, Session};

    fn create_test_state() -> AppState {
        let mut session = Session::new("test".to_string(), "abc123".to_string());
        session.sections = vec![
            Section::new(
                "s1".to_string(),
                "Section 1".to_string(),
                "Description 1".to_string(),
                "+added line\n-removed line".to_string(),
            ),
            Section::new(
                "s2".to_string(),
                "Section 2".to_string(),
                "Description 2".to_string(),
                "code".to_string(),
            ),
        ];
        let mut state = AppState::new(session);
        state.goto(Screen::Triage);
        state
    }

    #[test]
    fn test_triage_screen_new() {
        let screen = TriageScreen::new();
        assert_eq!(screen.list_state.selected(), Some(0));
    }

    #[test]
    fn test_triage_screen_navigation() {
        let mut screen = TriageScreen::new();
        let state = create_test_state();

        // Move down
        screen.select_next(&state);
        assert_eq!(screen.list_state.selected(), Some(1));

        // Wrap around
        screen.select_next(&state);
        assert_eq!(screen.list_state.selected(), Some(0));

        // Move up
        screen.select_previous(&state);
        assert_eq!(screen.list_state.selected(), Some(1));
    }

    #[test]
    fn test_triage_screen_tagging() {
        let mut screen = TriageScreen::new();
        let mut state = create_test_state();

        screen.tag_current(&mut state, Tag::GotIt);
        assert_eq!(state.session.sections[0].tag, Tag::GotIt);

        // tag_current doesn't advance; the key handler does
        screen.select_next(&state);
        screen.tag_current(&mut state, Tag::Shaky);
        assert_eq!(state.session.sections[1].tag, Tag::Shaky);
    }

    #[test]
    fn test_triage_screen_handle_quit() {
        let mut screen = TriageScreen::new();
        let mut state = create_test_state();

        let key = KeyEvent::new(
            crossterm::event::KeyCode::Char('q'),
            crossterm::event::KeyModifiers::NONE,
        );
        let result = screen.handle_input(key, &mut state);

        assert!(result.is_ok());
        assert!(state.should_quit);
    }

    #[test]
    fn test_triage_screen_handle_navigation_keys() {
        let mut screen = TriageScreen::new();
        let mut state = create_test_state();

        // Test j key
        let key_j = KeyEvent::new(
            crossterm::event::KeyCode::Char('j'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_j, &mut state).unwrap();
        assert_eq!(screen.list_state.selected(), Some(1));

        // Test k key
        let key_k = KeyEvent::new(
            crossterm::event::KeyCode::Char('k'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_k, &mut state).unwrap();
        assert_eq!(screen.list_state.selected(), Some(0));
    }

    #[test]
    fn test_triage_screen_handle_tagging_keys() {
        let mut screen = TriageScreen::new();
        let mut state = create_test_state();

        // Test 1 key (got it) — tags section 0, auto-advances to section 1
        let key_1 = KeyEvent::new(
            crossterm::event::KeyCode::Char('1'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_1, &mut state).unwrap();
        assert_eq!(state.session.sections[0].tag, Tag::GotIt);
        assert_eq!(screen.list_state.selected(), Some(1));
        assert_eq!(state.screen, Screen::Triage); // not all tagged yet

        // Test 2 key (shaky) — tags section 1, all tagged → transitions to DeepReview
        let key_2 = KeyEvent::new(
            crossterm::event::KeyCode::Char('2'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_2, &mut state).unwrap();
        assert_eq!(state.session.sections[1].tag, Tag::Shaky);
        assert_eq!(state.screen, Screen::DeepReview);
    }

    #[test]
    fn test_triage_retag_does_not_transition() {
        let mut screen = TriageScreen::new();
        let mut state = create_test_state();

        // Tag section 0 → advances to section 1
        let key_1 = KeyEvent::new(
            crossterm::event::KeyCode::Char('1'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_1, &mut state).unwrap();
        assert_eq!(screen.list_state.selected(), Some(1));

        // Navigate back to section 0 and retag with 3 (lost)
        let key_k = KeyEvent::new(
            crossterm::event::KeyCode::Char('k'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_k, &mut state).unwrap();
        let key_3 = KeyEvent::new(
            crossterm::event::KeyCode::Char('3'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_3, &mut state).unwrap();
        assert_eq!(state.session.sections[0].tag, Tag::Lost);
        // Section 1 still untagged, so should stay in triage
        assert_eq!(state.screen, Screen::Triage);
        assert_eq!(screen.list_state.selected(), Some(1));
    }

    #[test]
    fn test_enter_enters_code_focus() {
        let mut screen = TriageScreen::new();
        let mut state = create_test_state();

        let key_enter = KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_enter, &mut state).unwrap();
        assert!(screen.code_focused);
        assert_eq!(state.ui.scroll_offset, 0);
    }

    #[test]
    fn test_l_enters_code_focus() {
        let mut screen = TriageScreen::new();
        let mut state = create_test_state();

        let key_l = KeyEvent::new(
            crossterm::event::KeyCode::Char('l'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_l, &mut state).unwrap();
        assert!(screen.code_focused);
    }

    #[test]
    fn test_h_returns_to_list_focus() {
        let mut screen = TriageScreen::new();
        let mut state = create_test_state();

        // Enter code focus
        screen.code_focused = true;

        let key_h = KeyEvent::new(
            crossterm::event::KeyCode::Char('h'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_h, &mut state).unwrap();
        assert!(!screen.code_focused);
    }

    #[test]
    fn test_code_focus_jk_scrolls_by_one() {
        let mut screen = TriageScreen::new();
        let mut state = create_test_state();
        screen.code_focused = true;

        let key_j = KeyEvent::new(
            crossterm::event::KeyCode::Char('j'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_j, &mut state).unwrap();
        assert_eq!(state.ui.scroll_offset, 1);

        screen.handle_input(key_j, &mut state).unwrap();
        assert_eq!(state.ui.scroll_offset, 2);

        let key_k = KeyEvent::new(
            crossterm::event::KeyCode::Char('k'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_k, &mut state).unwrap();
        assert_eq!(state.ui.scroll_offset, 1);
    }

    #[test]
    fn test_code_focus_ud_scrolls_by_ten() {
        let mut screen = TriageScreen::new();
        let mut state = create_test_state();
        screen.code_focused = true;

        let key_d = KeyEvent::new(
            crossterm::event::KeyCode::Char('d'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_d, &mut state).unwrap();
        assert_eq!(state.ui.scroll_offset, 10);

        screen.handle_input(key_d, &mut state).unwrap();
        assert_eq!(state.ui.scroll_offset, 20);

        let key_u = KeyEvent::new(
            crossterm::event::KeyCode::Char('u'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_u, &mut state).unwrap();
        assert_eq!(state.ui.scroll_offset, 10);

        // Test saturating subtraction
        screen.handle_input(key_u, &mut state).unwrap();
        screen.handle_input(key_u, &mut state).unwrap();
        assert_eq!(state.ui.scroll_offset, 0);
    }

    #[test]
    fn test_enter_resets_scroll_offset() {
        let mut screen = TriageScreen::new();
        let mut state = create_test_state();

        // Set a non-zero scroll offset
        state.ui.scroll_offset = 15;

        let key_enter = KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_enter, &mut state).unwrap();
        assert!(screen.code_focused);
        assert_eq!(state.ui.scroll_offset, 0);
    }

    #[test]
    fn test_triage_screen_progress_counts() {
        let mut state = create_test_state();

        let counts = state.session.tag_counts();
        assert_eq!(counts.tagged(), 0);
        assert_eq!(counts.total(), 2);

        state.session.sections[0].tag = Tag::GotIt;
        let counts = state.session.tag_counts();
        assert_eq!(counts.tagged(), 1);
    }

    #[test]
    fn test_triage_screen_empty_sections() {
        let mut screen = TriageScreen::new();
        let session = Session::new("test".to_string(), "abc123".to_string());
        let mut state = AppState::new(session);

        // Navigation should not panic with empty sections
        screen.select_next(&state);
        screen.select_previous(&state);

        // Quit should still work
        let key_q = KeyEvent::new(
            crossterm::event::KeyCode::Char('q'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_q, &mut state).unwrap();
        assert!(state.should_quit);
    }

    #[test]
    fn test_triage_screen_error_state() {
        let mut screen = TriageScreen::new();
        let mut state = create_test_state();
        state.ui.set_error("Test error");

        // In error state, only q and r should work
        let key_j = KeyEvent::new(
            crossterm::event::KeyCode::Char('j'),
            crossterm::event::KeyModifiers::NONE,
        );
        let result = screen.handle_input(key_j, &mut state).unwrap();
        assert!(!result); // Not consumed

        let key_r = KeyEvent::new(
            crossterm::event::KeyCode::Char('r'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key_r, &mut state).unwrap();
        assert!(state.ui.error.is_none());
    }

    #[test]
    fn test_triage_screen_single_section_navigation() {
        let mut screen = TriageScreen::new();
        let mut session = Session::new("test".to_string(), "abc123".to_string());
        session.sections = vec![Section::new(
            "s1".to_string(),
            "Single Section".to_string(),
            "Description".to_string(),
            "code".to_string(),
        )];
        let state = AppState::new(session);

        // With single section, navigation should keep selection at 0
        assert_eq!(screen.list_state.selected(), Some(0));

        // select_next should wrap back to 0
        screen.select_next(&state);
        assert_eq!(screen.list_state.selected(), Some(0));

        // select_previous should also stay at 0
        screen.select_previous(&state);
        assert_eq!(screen.list_state.selected(), Some(0));
    }
}
