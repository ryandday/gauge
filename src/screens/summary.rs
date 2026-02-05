use std::fs;
use std::path::PathBuf;

use chrono::Local;
use crossterm::event::KeyEvent;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use super::{render_footer_hints, ScreenTrait};
use crate::error::Result;
use crate::git::get_repo_root;
use crate::models::{AppState, Screen};

/// Summary screen showing review statistics and export option
pub struct SummaryScreen {
    /// Path to exported file (if any)
    exported_path: Option<PathBuf>,
}

impl SummaryScreen {
    pub fn new() -> Self {
        Self {
            exported_path: None,
        }
    }
}

impl Default for SummaryScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenTrait for SummaryScreen {
    fn render(&self, frame: &mut Frame, state: &AppState) {
        let area = frame.area();

        // Main layout
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Confidence breakdown
                Constraint::Length(5), // Accuracy breakdown
                Constraint::Min(5),    // Misconceptions list
                Constraint::Length(4), // Session info
                Constraint::Length(2), // Footer
            ])
            .split(area);

        self.render_confidence_breakdown(frame, layout[0], state);
        self.render_accuracy_breakdown(frame, layout[1], state);
        self.render_misconceptions(frame, layout[2], state);
        self.render_session_info(frame, layout[3], state);
        self.render_footer(frame, layout[4]);
    }

    fn handle_input(&mut self, key: KeyEvent, state: &mut AppState) -> Result<bool> {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('q') => {
                state.quit();
                Ok(true)
            }
            KeyCode::Char('e') => {
                // Export to markdown
                match self.export_markdown(state) {
                    Ok(path) => {
                        self.exported_path = Some(path);
                    }
                    Err(e) => {
                        state.ui.set_error(format!("Export failed: {}", e));
                    }
                }
                Ok(true)
            }
            // Back to deep review
            KeyCode::Esc => {
                state.goto(Screen::DeepReview);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

impl SummaryScreen {
    /// Calculate proportional width for bar visualization segments
    fn proportional_width(count: usize, total: usize, bar_width: usize) -> usize {
        if total > 0 {
            (count * bar_width) / total
        } else {
            0
        }
    }

    fn render_confidence_breakdown(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let counts = state.session.tag_counts();

        let text = format!(
            "Sections: {} got it | {} shaky | {} lost",
            counts.got_it, counts.shaky, counts.lost
        );

        // Create a simple bar visualization
        let total = counts.total();
        let bar_width = area.width.saturating_sub(4) as usize;
        let got_it_width = Self::proportional_width(counts.got_it, total, bar_width);
        let shaky_width = Self::proportional_width(counts.shaky, total, bar_width);
        let lost_width = Self::proportional_width(counts.lost, total, bar_width);

        let bar = Line::from(vec![
            Span::styled(
                "\u{2588}".repeat(got_it_width),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                "\u{2588}".repeat(shaky_width),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                "\u{2588}".repeat(lost_width),
                Style::default().fg(Color::Red),
            ),
        ]);

        let content = vec![Line::from(text), Line::from(""), bar];

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Confidence Breakdown"),
            )
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    fn render_accuracy_breakdown(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let counts = state.session.hypothesis_counts();

        let text = format!(
            "Hypotheses: {} confirmed | {} corrected",
            counts.confirmed, counts.corrected
        );

        let accuracy_pct = if counts.total() > 0 {
            ((counts.confirmed as f64 / counts.total() as f64) * 100.0).round() as usize
        } else {
            0
        };

        let accuracy_text = if counts.total() > 0 {
            format!("Accuracy: {}%", accuracy_pct)
        } else {
            "No hypotheses submitted".to_string()
        };

        // Color thresholds: green (>=80% excellent), yellow (>=50% needs work), red (<50% significant gaps)
        let content = vec![
            Line::from(text),
            Line::from(""),
            Line::styled(
                accuracy_text,
                if accuracy_pct >= 80 {
                    Style::default().fg(Color::Green)
                } else if accuracy_pct >= 50 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Red)
                },
            ),
        ];

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Accuracy Breakdown"),
            )
            .alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    fn render_misconceptions(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        // Collect all misconceptions (diverges + missed) from assessments
        let mut misconceptions: Vec<ListItem> = Vec::new();

        for section in &state.session.sections {
            if let Some(assessment) = &section.assessment {
                for diverge in &assessment.diverges {
                    let item = ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("[{}] ", section.title),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::raw(diverge),
                    ]));
                    misconceptions.push(item);
                }
                for missed in &assessment.missed {
                    let item = ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("[{}] ", section.title),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(missed, Style::default().fg(Color::DarkGray)),
                    ]));
                    misconceptions.push(item);
                }
            }
        }

        if misconceptions.is_empty() {
            let paragraph = Paragraph::new("No misconceptions recorded.")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Key Misconceptions"),
                )
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
        } else {
            let list = List::new(misconceptions).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Key Misconceptions"),
            );
            frame.render_widget(list, area);
        }
    }

    fn render_session_info(&self, frame: &mut Frame, area: Rect, state: &AppState) {
        let mut lines = vec![Line::from(format!("Session: {}", state.session.name))];

        if let Some(path) = &self.exported_path {
            lines.push(Line::styled(
                format!("Exported to: {}", path.display()),
                Style::default().fg(Color::Green),
            ));
        }

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Session Info"))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let hints = "e: export to markdown | Esc: back | q: quit";
        render_footer_hints(frame, area, hints);
    }

    /// Export the review session to a markdown file
    fn export_markdown(&self, state: &AppState) -> std::result::Result<PathBuf, String> {
        // Use repository root docs/ as default output directory
        let cwd = std::env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?;
        let repo_root = get_repo_root(&cwd)
            .map_err(|e| format!("Failed to find repository root: {}", e))?;
        let output_dir = repo_root.join("docs");
        self.export_markdown_to(state, output_dir)
    }

    /// Export the review session to a markdown file in the specified directory
    fn export_markdown_to(&self, state: &AppState, output_dir: PathBuf) -> std::result::Result<PathBuf, String> {
        let now = Local::now();
        let filename = format!("gauge-review-{}.md", now.format("%Y-%m-%d-%H%M%S"));

        if !output_dir.exists() {
            fs::create_dir_all(&output_dir)
                .map_err(|e| format!("Failed to create output directory: {}", e))?;
        }

        let path = output_dir.join(&filename);

        let mut content = String::new();

        // Header
        content.push_str(&format!("# Code Review: {}\n\n", state.session.name));
        content.push_str(&format!(
            "Generated: {}\n\n",
            now.format("%Y-%m-%d %H:%M:%S")
        ));

        // Summary stats
        let tag_counts = state.session.tag_counts();
        let hyp_counts = state.session.hypothesis_counts();

        content.push_str("## Summary\n\n");
        content.push_str(&format!(
            "- **Sections**: {} got it, {} shaky, {} lost\n",
            tag_counts.got_it, tag_counts.shaky, tag_counts.lost
        ));
        content.push_str(&format!(
            "- **Hypotheses**: {} confirmed, {} corrected\n\n",
            hyp_counts.confirmed, hyp_counts.corrected
        ));

        // Sections detail
        content.push_str("## Sections\n\n");

        for section in &state.session.sections {
            content.push_str(&format!(
                "### {} [{}]\n\n",
                section.title,
                section.tag.label()
            ));
            content.push_str(&format!("{}\n\n", section.description));

            if let Some(hypothesis) = &section.hypothesis {
                content.push_str("**My Understanding:**\n\n");
                content.push_str(&format!("> {}\n\n", hypothesis));
            }

            if let Some(assessment) = &section.assessment {
                if !assessment.correct.is_empty() {
                    content.push_str("**Correct:**\n");
                    for item in &assessment.correct {
                        content.push_str(&format!("- {}\n", item));
                    }
                    content.push('\n');
                }

                if !assessment.diverges.is_empty() {
                    content.push_str("**Diverges:**\n");
                    for item in &assessment.diverges {
                        content.push_str(&format!("- {}\n", item));
                    }
                    content.push('\n');
                }

                if !assessment.missed.is_empty() {
                    content.push_str("**Missed:**\n");
                    for item in &assessment.missed {
                        content.push_str(&format!("- {}\n", item));
                    }
                    content.push('\n');
                }
            }

            content.push_str("---\n\n");
        }

        fs::write(&path, content).map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Assessment, Section, Session, Tag};
    use tempfile::tempdir;

    fn create_test_state() -> AppState {
        let mut session = Session::new("test-review".to_string(), "abc123".to_string());
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
        ];
        session.sections[0].tag = Tag::GotIt;
        session.sections[1].tag = Tag::Shaky;
        session.sections[1].hypothesis = Some("My hypothesis".to_string());
        session.sections[1].assessment = Some(Assessment {
            correct: vec!["Good point".to_string()],
            diverges: vec!["Incorrect assumption".to_string()],
            missed: vec!["Missed feature".to_string()],
        });

        AppState::new(session)
    }

    #[test]
    fn test_summary_screen_new() {
        let screen = SummaryScreen::new();
        assert!(screen.exported_path.is_none());
    }

    #[test]
    fn test_summary_screen_quit() {
        let mut screen = SummaryScreen::new();
        let mut state = create_test_state();

        let key = KeyEvent::new(
            crossterm::event::KeyCode::Char('q'),
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key, &mut state).unwrap();
        assert!(state.should_quit);
    }

    #[test]
    fn test_summary_screen_back() {
        let mut screen = SummaryScreen::new();
        let mut state = create_test_state();

        let key = KeyEvent::new(
            crossterm::event::KeyCode::Esc,
            crossterm::event::KeyModifiers::NONE,
        );
        screen.handle_input(key, &mut state).unwrap();
        assert_eq!(state.screen, Screen::DeepReview);
    }

    #[test]
    fn test_summary_export_markdown() {
        let screen = SummaryScreen::new();
        let state = create_test_state();

        // Use temp directory for testing - pass it directly to avoid changing global cwd
        let temp = tempdir().unwrap();
        let output_dir = temp.path().to_path_buf();

        let result = screen.export_markdown_to(&state, output_dir);
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.exists());
        assert!(path.to_string_lossy().contains("gauge-review"));
        assert!(path.to_string_lossy().ends_with(".md"));

        // Verify content
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Code Review: test-review"));
        assert!(content.contains("Section 1"));
        assert!(content.contains("Section 2"));
        assert!(content.contains("My hypothesis"));
        assert!(content.contains("Good point"));
        assert!(content.contains("Incorrect assumption"));
    }

    #[test]
    fn test_summary_tag_counts() {
        let state = create_test_state();
        let counts = state.session.tag_counts();

        assert_eq!(counts.got_it, 1);
        assert_eq!(counts.shaky, 1);
        assert_eq!(counts.lost, 0);
    }

    #[test]
    fn test_summary_hypothesis_counts() {
        let state = create_test_state();
        let counts = state.session.hypothesis_counts();

        // One section with assessment, it has diverges so counts as corrected
        assert_eq!(counts.confirmed, 0);
        assert_eq!(counts.corrected, 1);
    }
}
