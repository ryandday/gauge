// This module provides the real AI client that shells out to the Claude CLI.
#![allow(dead_code)]

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::Duration;

use wait_timeout::ChildExt;

use crate::models::{Assessment, Section};

/// Timeout for Claude CLI subprocess calls.
/// Set to 2 minutes to allow for complex diff analysis while preventing hung processes.
const CLAUDE_TIMEOUT_SECS: u64 = 120;

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

    /// Run a prompt through the Claude CLI with a 2-minute timeout
    fn run_claude(&self, prompt: &str) -> Result<String, String> {
        let mut child = Command::new("claude")
            .args(["--dangerously-skip-permissions", "-p", prompt])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn claude process: {}", e))?;

        let timeout = Duration::from_secs(CLAUDE_TIMEOUT_SECS);
        let status = child
            .wait_timeout(timeout)
            .map_err(|e| format!("Failed to wait on claude process: {}", e))?;

        match status {
            Some(exit_status) => {
                // Process completed within timeout
                let mut stdout = String::new();
                let mut stderr = String::new();

                if let Some(mut stdout_handle) = child.stdout.take() {
                    stdout_handle
                        .read_to_string(&mut stdout)
                        .map_err(|e| format!("Failed to read stdout: {}", e))?;
                }
                if let Some(mut stderr_handle) = child.stderr.take() {
                    stderr_handle
                        .read_to_string(&mut stderr)
                        .map_err(|e| format!("Failed to read stderr: {}", e))?;
                }

                if !exit_status.success() {
                    return Err(format!("Claude CLI failed: {}", stderr.trim()));
                }

                Ok(stdout)
            }
            None => {
                // Timeout - kill the process
                if let Err(e) = child.kill() {
                    eprintln!("Warning: Failed to kill timed-out Claude process: {}", e);
                }
                Err("Claude CLI timed out after 2 minutes".to_string())
            }
        }
    }

    /// Parse a JSON array of sections from Claude's response
    fn parse_sections(&self, response: &str) -> Result<Vec<Section>, String> {
        // Extract JSON from response - Claude may include markdown code blocks
        let json_str = extract_json_array(response).ok_or_else(|| {
            format!(
                "No valid JSON array found in response: {}",
                truncate(response, 200)
            )
        })?;

        #[derive(serde::Deserialize)]
        struct SectionResponse {
            id: String,
            title: String,
            description: String,
            code: String,
            #[serde(default)]
            files: Vec<String>,
        }

        let sections: Vec<SectionResponse> = serde_json::from_str(&json_str).map_err(|e| {
            format!(
                "Failed to parse sections JSON: {}. Raw: {}",
                e,
                truncate(&json_str, 200)
            )
        })?;

        Ok(sections
            .into_iter()
            .map(|s| {
                let mut section = Section::new(s.id, s.title, s.description, s.code);
                section.files = s.files;
                section
            })
            .collect())
    }

    /// Parse an assessment response from Claude
    fn parse_assessment(&self, response: &str) -> Result<Assessment, String> {
        // Extract JSON from response
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
    fn chunk_diff(&mut self, diff: &str) -> ChunkingResult {
        if self.busy {
            return ChunkingResult::Error(ChunkingError::new(
                "AI client is busy with another operation",
            ));
        }

        self.busy = true;
        let result = self.chunk_diff_impl(diff);
        self.busy = false;
        result
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
    fn chunk_diff_impl(&self, diff: &str) -> ChunkingResult {
        let prompt = build_chunking_prompt(diff);

        match self.run_claude(&prompt) {
            Ok(response) => match self.parse_sections(&response) {
                Ok(sections) => ChunkingResult::Success(sections),
                Err(e) => ChunkingResult::Error(ChunkingError::new(e).with_output(response)),
            },
            Err(e) => ChunkingResult::Error(ChunkingError::new(e)),
        }
    }

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

/// Build the prompt for chunking a diff into sections
fn build_chunking_prompt(diff: &str) -> String {
    format!(
        r#"You are analyzing a git diff to help a developer understand the changes. Your task is to chunk this diff into logical sections that can be reviewed one at a time.

IMPORTANT: Respond ONLY with a JSON array. No markdown, no explanation, just the JSON.

For each section, provide:
- id: A unique identifier (e.g., "sec_1", "sec_2")
- title: A short, descriptive title (e.g., "Core Data Models", "API Handlers")
- description: A brief description of what this section contains (1-2 sentences)
- code: The relevant diff text for this section
- files: Array of file paths affected by this section

Order sections by importance: core types and data structures first, then main logic, then utilities and helpers last.

Example output format:
[
  {{
    "id": "sec_1",
    "title": "Core Data Models",
    "description": "Defines the fundamental types used throughout the application.",
    "code": "diff --git a/src/models.rs ...",
    "files": ["src/models.rs"]
  }}
]

Here is the diff to analyze:

{}"#,
        diff
    )
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
        // Skip optional language identifier on the same line
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
        // Find matching bracket
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

/// Extract a JSON array from a response that may contain markdown or other text
fn extract_json_array(text: &str) -> Option<String> {
    extract_json(text, '[', ']')
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
            // Use checked_sub to handle malformed JSON with more closing than opening brackets
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
        // Find a valid UTF-8 boundary at or before max_len
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
    fn test_extract_json_array_from_code_block() {
        let response = r#"Here is the analysis:

```json
[
  {"id": "sec_1", "title": "Test", "description": "Test desc", "code": "code", "files": []}
]
```

That's the breakdown."#;

        let extracted = extract_json_array(response).unwrap();
        assert!(extracted.starts_with('['));
        assert!(extracted.contains("sec_1"));
    }

    #[test]
    fn test_extract_json_array_raw() {
        let response = r#"[{"id": "sec_1", "title": "Test", "description": "Desc", "code": "c", "files": []}]"#;
        let extracted = extract_json_array(response).unwrap();
        assert_eq!(extracted, response);
    }

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
        assert_eq!(find_matching_bracket("[1, 2, 3]", '[', ']'), Some(8));
        assert_eq!(find_matching_bracket("{}", '{', '}'), Some(1));
    }

    #[test]
    fn test_find_matching_bracket_nested() {
        assert_eq!(find_matching_bracket("[[1], [2]]", '[', ']'), Some(9));
        assert_eq!(
            find_matching_bracket(r#"{"a": {"b": 1}}"#, '{', '}'),
            Some(14)
        );
    }

    #[test]
    fn test_find_matching_bracket_with_strings() {
        // Brackets inside strings should be ignored
        assert_eq!(
            find_matching_bracket(r#"["hello [world]"]"#, '[', ']'),
            Some(16)
        );
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
    fn test_build_chunking_prompt() {
        let prompt = build_chunking_prompt("diff text");
        assert!(prompt.contains("diff text"));
        assert!(prompt.contains("JSON array"));
        assert!(prompt.contains("core types"));
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
    fn test_parse_sections_valid() {
        let client = ClaudeClient::new();
        let json = r#"[
            {
                "id": "sec_1",
                "title": "Test Section",
                "description": "A test",
                "code": "diff --git",
                "files": ["src/test.rs"]
            }
        ]"#;

        let sections = client.parse_sections(json).unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].id, "sec_1");
        assert_eq!(sections[0].title, "Test Section");
        assert_eq!(sections[0].files, vec!["src/test.rs"]);
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

    // Edge case tests for AI response parsing (TA-006)

    #[test]
    fn test_parse_sections_malformed_json_returns_error() {
        let client = ClaudeClient::new();
        let malformed = r#"[{"id": "sec_1", "title": "Incomplete"#; // truncated JSON
        let result = client.parse_sections(malformed);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No valid JSON array found"));
    }

    #[test]
    fn test_parse_sections_with_unicode_in_title() {
        let client = ClaudeClient::new();
        let json = r#"[
            {
                "id": "sec_1",
                "title": "Unicode Test: \u4e2d\u6587 \u65e5\u672c\u8a9e \ud83d\ude80",
                "description": "Contains emoji and CJK characters",
                "code": "// \u4ee3\u7801",
                "files": []
            }
        ]"#;

        let sections = client.parse_sections(json).unwrap();
        assert_eq!(sections.len(), 1);
        assert!(sections[0].title.contains("Unicode"));
    }

    #[test]
    fn test_parse_sections_empty_array() {
        let client = ClaudeClient::new();
        let json = "[]";
        let sections = client.parse_sections(json).unwrap();
        assert!(sections.is_empty());
    }

    #[test]
    fn test_parse_sections_with_explanation_text_before_json() {
        let client = ClaudeClient::new();
        let response = r#"Here's my analysis of the code changes:

The diff shows modifications to several files.

[
    {
        "id": "sec_1",
        "title": "Core Changes",
        "description": "Main logic updates",
        "code": "diff --git",
        "files": ["main.rs"]
    }
]

Hope this helps!"#;

        let sections = client.parse_sections(response).unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].id, "sec_1");
    }

    #[test]
    fn test_parse_sections_with_nested_objects_in_code() {
        let client = ClaudeClient::new();
        // Code field contains JSON-like content that should not confuse the parser
        let json = r#"[
            {
                "id": "sec_1",
                "title": "Config Changes",
                "description": "Updated config",
                "code": "let config = {\"key\": \"value\", \"nested\": {\"a\": 1}}",
                "files": ["config.rs"]
            }
        ]"#;

        let sections = client.parse_sections(json).unwrap();
        assert_eq!(sections.len(), 1);
        assert!(sections[0].code.contains("nested"));
    }

    #[test]
    fn test_truncate_with_multibyte_utf8() {
        // Test with a string that has multi-byte characters
        let s = "Hello \u{1F600} World"; // emoji is 4 bytes
        // Try to truncate in the middle of the emoji
        let result = truncate(s, 8); // Would be in middle of emoji
        // Should not panic and should truncate at valid boundary
        assert!(result.ends_with("..."));
        assert!(result.is_char_boundary(result.len() - 3)); // before "..."
    }

    #[test]
    fn test_truncate_cjk_characters() {
        let s = "\u{4e2d}\u{6587}\u{6d4b}\u{8bd5}"; // Chinese characters, 3 bytes each
        let result = truncate(s, 4); // Would be in middle of second char
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_extract_json_array_no_valid_json() {
        let response = "This is just plain text without any JSON";
        assert!(extract_json_array(response).is_none());
    }

    #[test]
    fn test_extract_json_object_partial_json() {
        let response = r#"{"incomplete": true, "missing_close"#;
        assert!(extract_json_object(response).is_none());
    }

    // Tests for escaped character handling (TA-014)

    #[test]
    fn test_extract_json_with_escaped_backslashes() {
        // JSON with Windows-style paths that have escaped backslashes
        let response = r#"[{"id": "sec_1", "title": "Path Test", "description": "Test", "code": "path\\to\\file", "files": ["C:\\Users\\test"]}]"#;
        let extracted = extract_json_array(response).unwrap();
        assert!(extracted.contains(r#"path\\to\\file"#));

        // Verify it can be parsed as valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&extracted).unwrap();
        assert!(parsed[0]["code"].as_str().unwrap().contains("path\\to\\file"));
    }

    #[test]
    fn test_extract_json_with_nested_quotes_in_code() {
        // JSON where the code field contains escaped quotes
        let response = r#"{"correct": ["User said \"hello\""], "diverges": [], "missed": []}"#;
        let extracted = extract_json_object(response).unwrap();

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&extracted).unwrap();
        assert!(parsed["correct"][0].as_str().unwrap().contains("\"hello\""));
    }

    #[test]
    fn test_find_matching_bracket_with_escaped_backslash_before_quote() {
        // Edge case: backslash-backslash-quote should NOT escape the quote
        // "string\\" followed by }, the quote is NOT escaped
        let json = r#"{"key": "value\\"}"#;
        let result = find_matching_bracket(json, '{', '}');
        assert_eq!(result, Some(17));
    }

    #[test]
    fn test_extract_json_with_newlines_in_code() {
        // JSON with literal \n in strings (as escaped sequence)
        let response = r#"[{"id": "sec_1", "title": "Test", "description": "Line1\nLine2", "code": "fn main() {\n    println!(\"hello\");\n}", "files": []}]"#;
        let extracted = extract_json_array(response).unwrap();

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&extracted).unwrap();
        assert!(parsed[0]["code"].as_str().unwrap().contains("println"));
    }

    // Tests for empty diff edge case (TA-009)

    #[test]
    fn test_build_chunking_prompt_with_empty_diff() {
        // Verify prompt is built correctly even with empty diff
        let prompt = build_chunking_prompt("");
        assert!(prompt.contains("JSON array"));
        assert!(prompt.contains("Here is the diff to analyze:"));
    }

    #[test]
    fn test_build_chunking_prompt_with_whitespace_only_diff() {
        // Verify prompt handles whitespace-only diff
        let prompt = build_chunking_prompt("   \n\t  \n  ");
        assert!(prompt.contains("JSON array"));
        assert!(prompt.contains("Here is the diff to analyze:"));
    }

    #[test]
    fn test_parse_sections_handles_empty_diff_response() {
        // When AI receives empty diff, it should return empty array which parses correctly
        let client = ClaudeClient::new();
        let response = "[]";
        let sections = client.parse_sections(response).unwrap();
        assert!(sections.is_empty());
    }

    #[test]
    fn test_find_matching_bracket_malformed_extra_closing() {
        // Test for CR-001: More closing brackets than opening should return None
        assert_eq!(find_matching_bracket("}", '{', '}'), None);
        assert_eq!(find_matching_bracket("]", '[', ']'), None);
        assert_eq!(find_matching_bracket("[]]", '[', ']'), Some(1)); // First valid match
    }
}
