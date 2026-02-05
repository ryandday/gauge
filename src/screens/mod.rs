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
use ratatui::prelude::*;
use ratatui::Frame;
use std::sync::LazyLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;

use crate::error::Result;
use crate::models::{AppState, CodeBlock, CodeSource};

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME: LazyLock<Theme> = LazyLock::new(|| {
    let ts = ThemeSet::load_defaults();
    ts.themes["base16-ocean.dark"].clone()
});

/// Convert a syntect foreground color to a ratatui Style with RGB color.
fn syntect_to_ratatui_style(syntect_style: syntect::highlighting::Style) -> Style {
    let fg = syntect_style.foreground;
    Style::default().fg(Color::Rgb(fg.r, fg.g, fg.b))
}

/// Extract a file extension from a CodeSource for syntax detection.
fn extension_from_source(source: &CodeSource) -> Option<&str> {
    let path = match source {
        CodeSource::Diff { paths, .. } => paths.first().map(|s| s.as_str()),
        CodeSource::File { path, .. } => Some(path.as_str()),
    };
    path.and_then(|p| p.rsplit('.').next())
}

/// Apply syntax highlighting to diff lines from code blocks.
/// Uses syntect for language-aware highlighting while preserving diff prefix coloring.
pub fn highlight_diff_lines(blocks: &[CodeBlock]) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for block in blocks {
        let ext = extension_from_source(&block.source);
        let syntax = ext
            .and_then(|e| SYNTAX_SET.find_syntax_by_extension(e))
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

        let mut highlighter = HighlightLines::new(syntax, &THEME);

        for line in block.content.lines() {
            // Metadata lines: keep simple coloring
            if line.starts_with("diff ")
                || line.starts_with("index ")
                || line.starts_with("--- ")
                || line.starts_with("+++ ")
            {
                lines.push(Line::styled(
                    line.to_string(),
                    Style::default().fg(Color::Blue),
                ));
                continue;
            }
            if line.starts_with("@@") {
                lines.push(Line::styled(
                    line.to_string(),
                    Style::default().fg(Color::Cyan),
                ));
                continue;
            }

            // Code lines: strip prefix, highlight, prepend colored prefix
            let (prefix_char, prefix_style) = if line.starts_with('+') {
                ("+", Style::default().fg(Color::Green))
            } else if line.starts_with('-') {
                ("-", Style::default().fg(Color::Red))
            } else if line.starts_with(' ') {
                (" ", Style::default())
            } else {
                // No recognized prefix — highlight the whole line as-is
                match highlighter.highlight_line(line, &SYNTAX_SET) {
                    Ok(ranges) => {
                        let spans: Vec<Span<'static>> = ranges
                            .into_iter()
                            .map(|(style, text)| {
                                Span::styled(text.to_string(), syntect_to_ratatui_style(style))
                            })
                            .collect();
                        lines.push(Line::from(spans));
                    }
                    Err(_) => lines.push(Line::raw(line.to_string())),
                }
                continue;
            };

            let code_part = &line[1..];
            match highlighter.highlight_line(code_part, &SYNTAX_SET) {
                Ok(ranges) => {
                    let mut spans: Vec<Span<'static>> = Vec::with_capacity(ranges.len() + 1);
                    spans.push(Span::styled(prefix_char.to_string(), prefix_style));
                    for (style, text) in ranges {
                        spans.push(Span::styled(
                            text.to_string(),
                            syntect_to_ratatui_style(style),
                        ));
                    }
                    lines.push(Line::from(spans));
                }
                Err(_) => lines.push(Line::styled(line.to_string(), prefix_style)),
            }
        }
    }

    lines
}

use ratatui::widgets::{Block, Borders, Paragraph};

/// Shared helper to render an error panel with retry/quit hints.
/// This consolidates the common error rendering pattern used across screens.
pub fn render_error_panel(frame: &mut Frame, area: Rect, title: &str, error: &str, hints: &str) {
    let text = format!("Error: {}\n\n{}", error, hints);

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Red)),
        )
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red));

    frame.render_widget(paragraph, area);
}

/// Shared helper to render footer hints with consistent styling.
/// Displays the hints text centered in DarkGray color.
pub fn render_footer_hints(frame: &mut Frame, area: Rect, hints: &str) {
    let footer = Paragraph::new(hints)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, area);
}

use crossterm::event::{KeyCode, KeyModifiers};

/// Scroll increment for Ctrl+D/Ctrl+U (half-page scroll)
const SCROLL_INCREMENT_HALF: usize = 10;

/// Scroll increment for PageDown/PageUp (full-page scroll)
const SCROLL_INCREMENT_FULL: usize = 20;

/// Handle scroll-related keyboard input for code preview panels.
/// Returns `Some(true)` if the event was consumed, `None` if not a scroll key.
/// Supports Ctrl+D (down half-page), Ctrl+U (up half-page), PageDown (down full-page), PageUp (up full-page).
pub fn handle_scroll_input(key: &KeyEvent, scroll_offset: &mut usize) -> Option<bool> {
    match key.code {
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            *scroll_offset = scroll_offset.saturating_add(SCROLL_INCREMENT_HALF);
            Some(true)
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            *scroll_offset = scroll_offset.saturating_sub(SCROLL_INCREMENT_HALF);
            Some(true)
        }
        KeyCode::PageDown => {
            *scroll_offset = scroll_offset.saturating_add(SCROLL_INCREMENT_FULL);
            Some(true)
        }
        KeyCode::PageUp => {
            *scroll_offset = scroll_offset.saturating_sub(SCROLL_INCREMENT_FULL);
            Some(true)
        }
        _ => None,
    }
}

/// Common response type for error state input handling.
/// Used by screens to handle the standard 'q' to quit and 'r' to retry keys.
pub enum ErrorInputResult {
    /// User pressed 'q' - quit the application
    Quit,
    /// User pressed 'r' - clear error and retry
    Retry,
    /// Key was not a common error-state key - screen should handle it
    NotHandled,
}

/// Handle common error state keyboard input (quit and retry).
/// Returns the action to take. Screens with additional error-state keys
/// should check for NotHandled and process their custom keys.
pub fn handle_error_state_input(key: &KeyEvent) -> ErrorInputResult {
    match key.code {
        KeyCode::Char('q') => ErrorInputResult::Quit,
        KeyCode::Char('r') => ErrorInputResult::Retry,
        _ => ErrorInputResult::NotHandled,
    }
}

/// Estimate how many visual lines wrapped text will occupy at the given width.
/// Approximates ratatui's `Wrap { trim: true }` word-wrapping behavior.
pub fn wrapped_height(text: &str, width: u16) -> u16 {
    if text.is_empty() || width == 0 {
        return 0;
    }
    let width = width as usize;
    let mut total_lines: usize = 0;
    for line in text.split('\n') {
        if line.trim().is_empty() {
            total_lines += 1;
            continue;
        }
        let mut current_width: usize = 0;
        let mut line_count: usize = 1;
        for word in line.split_whitespace() {
            let word_len = word.len();
            if current_width == 0 {
                current_width = word_len;
            } else if current_width + 1 + word_len > width {
                line_count += 1;
                current_width = word_len;
            } else {
                current_width += 1 + word_len;
            }
        }
        total_lines += line_count;
    }
    total_lines as u16
}

/// Trait that all screens must implement
pub trait ScreenTrait {
    /// Render the screen to the terminal
    fn render(&self, frame: &mut Frame, state: &AppState);

    /// Handle keyboard input, returns true if the event was consumed
    fn handle_input(&mut self, key: KeyEvent, state: &mut AppState) -> Result<bool>;
}
