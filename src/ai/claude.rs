// This module provides the real AI client that shells out to the Claude CLI.
#![allow(dead_code)]

use std::process::{Command, Stdio};

use crate::models::Assessment;

use super::types::{AssessmentError, AssessmentResult, ChunkingError, ChunkingResult};
use super::AiClient;

/// Real AI client that shells out to the Claude CLI
pub struct ClaudeClient {
    busy: bool,
}

impl ClaudeClient {
    pub fn new() -> Self {
        Self { busy: false }
    }

    /// Run a prompt through the Claude CLI, waiting until completion
    fn run_claude(&self, prompt: &str) -> Result<String, String> {
        let child = Command::new("claude")
            .args(["--dangerously-skip-permissions", "-p", prompt])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn claude process: {}", e))?;

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait on claude process: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Claude CLI failed: {}", stderr.trim()));
        }

        String::from_utf8(output.stdout)
            .map_err(|e| format!("Claude output was not valid UTF-8: {}", e))
    }

    /// Parse an assessment response from Claude
    fn parse_assessment(&self, response: &str) -> Result<Assessment, String> {
        let json_str = extract_json_object(response).ok_or_else(|| {
            format!(
                "No valid JSON object found in response: {}",
                truncate(response, 200)
            )
        })?;

        #[derive(serde::Deserialize)]
        struct AssessmentResponse {
            correct: Vec<String>,
            diverges: Vec<String>,
            missed: Vec<String>,
        }

        let assessment: AssessmentResponse = serde_json::from_str(&json_str).map_err(|e| {
            format!(
                "Failed to parse assessment JSON: {}. Raw: {}",
                e,
                truncate(&json_str, 200)
            )
        })?;

        Ok(Assessment {
            correct: assessment.correct,
            diverges: assessment.diverges,
            missed: assessment.missed,
        })
    }
}

impl Default for ClaudeClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AiClient for ClaudeClient {
    fn chunk_diff(&mut self, _diff: &str) -> ChunkingResult {
        // Chunking is no longer done via AI - sections are created manually via CLI
        ChunkingResult::Error(ChunkingError::new(
            "AI chunking is no longer supported. Use 'sherpa section add' instead.",
        ))
    }

    fn assess_hypothesis(&mut self, code: &str, hypothesis: &str) -> AssessmentResult {
        if self.busy {
            return AssessmentResult::Error(AssessmentError::new(
                "AI client is busy with another operation",
            ));
        }

        self.busy = true;
        let result = self.assess_hypothesis_impl(code, hypothesis);
        self.busy = false;
        result
    }

    fn is_busy(&self) -> bool {
        self.busy
    }
}

impl ClaudeClient {
    fn assess_hypothesis_impl(&self, code: &str, hypothesis: &str) -> AssessmentResult {
        let prompt = build_assessment_prompt(code, hypothesis);

        match self.run_claude(&prompt) {
            Ok(response) => match self.parse_assessment(&response) {
                Ok(assessment) => AssessmentResult::Success(assessment),
                Err(e) => AssessmentResult::Error(AssessmentError::new(e).with_output(response)),
            },
            Err(e) => AssessmentResult::Error(AssessmentError::new(e)),
        }
    }
}

/// Build the prompt for assessing a hypothesis
fn build_assessment_prompt(code: &str, hypothesis: &str) -> String {
    format!(
        r#"You are evaluating a developer's understanding of a code section. Compare their hypothesis to what the code actually does.

IMPORTANT: Respond ONLY with a JSON object. No markdown, no explanation, just the JSON.

The response must have exactly three arrays:
- correct: Points where the developer's understanding matches the code behavior
- diverges: Points where the developer's interpretation differs from what the code actually does
- missed: Important code behaviors the developer did not mention

Be specific and constructive. Each point should be a clear, complete sentence.

Example output format:
{{
  "correct": [
    "Correctly identified that the function validates input before processing",
    "Understood the error handling flow for invalid data"
  ],
  "diverges": [
    "The function returns early on empty input, not null as stated"
  ],
  "missed": [
    "The function also logs to an audit trail on each call"
  ]
}}

CODE SECTION:
{}

DEVELOPER'S HYPOTHESIS:
{}"#,
        code, hypothesis
    )
}

/// Claude responses may include JSON in markdown code blocks (```json), plain code blocks (```),
/// or as raw text. This function handles all these formats to robustly extract JSON.
fn extract_json(text: &str, open: char, close: char) -> Option<String> {
    // First try to find JSON in a code block
    if let Some(start) = text.find("```json") {
        let after_marker = &text[start + 7..];
        if let Some(end) = after_marker.find("```") {
            return Some(after_marker[..end].trim().to_string());
        }
    }

    // Try plain code block
    if let Some(start) = text.find("```") {
        let after_marker = &text[start + 3..];
        let content_start = after_marker.find('\n').map(|i| i + 1).unwrap_or(0);
        let after_newline = &after_marker[content_start..];
        if let Some(end) = after_newline.find("```") {
            let content = after_newline[..end].trim();
            if content.starts_with(open) {
                return Some(content.to_string());
            }
        }
    }

    // Look for raw JSON starting with the expected bracket
    let trimmed = text.trim();
    if trimmed.starts_with(open) {
        if let Some(end) = find_matching_bracket(trimmed, open, close) {
            return Some(trimmed[..=end].to_string());
        }
    }

    // Search for JSON anywhere in text
    if let Some(start) = text.find(open) {
        let from_bracket = &text[start..];
        if let Some(end) = find_matching_bracket(from_bracket, open, close) {
            return Some(from_bracket[..=end].to_string());
        }
    }

    None
}

/// Extract a JSON object from a response that may contain markdown or other text
fn extract_json_object(text: &str) -> Option<String> {
    extract_json(text, '{', '}')
}

/// Find the index of the matching closing bracket
fn find_matching_bracket(text: &str, open: char, close: char) -> Option<usize> {
    let mut depth: usize = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, c) in text.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        if c == '\\' && in_string {
            escape_next = true;
            continue;
        }

        if c == '"' {
            in_string = !in_string;
            continue;
        }

        if in_string {
            continue;
        }

        if c == open {
            depth += 1;
        } else if c == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(i);
            }
        }
    }

    None
}

/// Truncate a string for error messages, handling multi-byte UTF-8 safely
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut boundary = max_len;
        while boundary > 0 && !s.is_char_boundary(boundary) {
            boundary -= 1;
        }
        format!("{}...", &s[..boundary])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_object_from_code_block() {
        let response = r#"```json
{
  "correct": ["point 1"],
  "diverges": [],
  "missed": ["point 2"]
}
```"#;

        let extracted = extract_json_object(response).unwrap();
        assert!(extracted.starts_with('{'));
        assert!(extracted.contains("correct"));
    }

    #[test]
    fn test_extract_json_object_raw() {
        let response = r#"{"correct": [], "diverges": [], "missed": []}"#;
        let extracted = extract_json_object(response).unwrap();
        assert_eq!(extracted, response);
    }

    #[test]
    fn test_find_matching_bracket_simple() {
        assert_eq!(find_matching_bracket("{}", '{', '}'), Some(1));
    }

    #[test]
    fn test_find_matching_bracket_nested() {
        assert_eq!(
            find_matching_bracket(r#"{"a": {"b": 1}}"#, '{', '}'),
            Some(14)
        );
    }

    #[test]
    fn test_find_matching_bracket_with_strings() {
        assert_eq!(
            find_matching_bracket(r#"{"key": "value with } brace"}"#, '{', '}'),
            Some(28)
        );
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello...");
    }

    #[test]
    fn test_build_assessment_prompt() {
        let prompt = build_assessment_prompt("code here", "my hypothesis");
        assert!(prompt.contains("code here"));
        assert!(prompt.contains("my hypothesis"));
        assert!(prompt.contains("correct"));
        assert!(prompt.contains("diverges"));
        assert!(prompt.contains("missed"));
    }

    #[test]
    fn test_claude_client_is_not_busy_by_default() {
        let client = ClaudeClient::new();
        assert!(!client.is_busy());
    }

    #[test]
    fn test_parse_assessment_valid() {
        let client = ClaudeClient::new();
        let json = r#"{
            "correct": ["Point 1", "Point 2"],
            "diverges": ["Wrong assumption"],
            "missed": ["Missed detail"]
        }"#;

        let assessment = client.parse_assessment(json).unwrap();
        assert_eq!(assessment.correct.len(), 2);
        assert_eq!(assessment.diverges.len(), 1);
        assert_eq!(assessment.missed.len(), 1);
    }

    #[test]
    fn test_truncate_with_multibyte_utf8() {
        let s = "Hello \u{1F600} World";
        let result = truncate(s, 8);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_cjk_characters() {
        let s = "\u{4e2d}\u{6587}\u{6d4b}\u{8bd5}";
        let result = truncate(s, 4);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_extract_json_object_partial_json() {
        let response = r#"{"incomplete": true, "missing_close"#;
        assert!(extract_json_object(response).is_none());
    }

    #[test]
    fn test_extract_json_with_escaped_backslashes() {
        let response = r#"{"correct": ["path\\to\\file"], "diverges": [], "missed": []}"#;
        let extracted = extract_json_object(response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&extracted).unwrap();
        assert!(parsed["correct"][0].as_str().unwrap().contains("path\\to\\file"));
    }

    #[test]
    fn test_extract_json_with_nested_quotes_in_code() {
        let response = r#"{"correct": ["User said \"hello\""], "diverges": [], "missed": []}"#;
        let extracted = extract_json_object(response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&extracted).unwrap();
        assert!(parsed["correct"][0].as_str().unwrap().contains("\"hello\""));
    }

    #[test]
    fn test_find_matching_bracket_with_escaped_backslash_before_quote() {
        let json = r#"{"key": "value\\"}"#;
        let result = find_matching_bracket(json, '{', '}');
        assert_eq!(result, Some(17));
    }

    #[test]
    fn test_find_matching_bracket_malformed_extra_closing() {
        assert_eq!(find_matching_bracket("}", '{', '}'), None);
    }
}
