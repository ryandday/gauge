// Note: assess_hypothesis and is_busy methods are defined here for PHASE-2/3 parallel work
// but not used until those phases are implemented.
#![allow(dead_code)]

use super::types::{AssessmentResult, ChunkingResult};

/// Trait defining the AI client interface for code review operations.
///
/// ClaudeClient provides the real implementation via Claude CLI subprocess.
/// MockAiClient is used for testing.
pub trait AiClient {
    /// Chunk a git diff into logical sections ordered by importance.
    ///
    /// The AI should:
    /// - Group related changes across files
    /// - Create meaningful titles and descriptions
    /// - Order sections with core types first, utilities last
    ///
    /// # Arguments
    /// * `diff` - The unified diff text to chunk
    ///
    /// # Returns
    /// A `ChunkingResult` with either the sections or an error
    fn chunk_diff(&mut self, diff: &str) -> ChunkingResult;

    /// Assess a user's hypothesis about what code does.
    ///
    /// The AI should categorize the response into:
    /// - Correct: points matching the code's actual behavior
    /// - Diverges: points where the user's interpretation differs
    /// - Missed: behaviors the user did not identify
    ///
    /// # Arguments
    /// * `code` - The code section being reviewed
    /// * `hypothesis` - The user's description of what the code does
    ///
    /// # Returns
    /// An `AssessmentResult` with either the assessment or an error
    fn assess_hypothesis(&mut self, code: &str, hypothesis: &str) -> AssessmentResult;

    /// Check if an AI operation is currently in progress.
    ///
    /// Per spec: only one AI call at a time, but user can navigate while waiting.
    fn is_busy(&self) -> bool;
}

/// Mock implementation for testing purposes
///
/// Note: The mock implementation mirrors the concurrency behavior of ClaudeClient
/// to ensure consistent behavior during testing.
pub struct MockAiClient {
    busy: bool,
}

impl MockAiClient {
    pub fn new() -> Self {
        Self { busy: false }
    }

    /// Set the busy state manually for testing concurrent call rejection
    #[cfg(test)]
    pub fn set_busy(&mut self, busy: bool) {
        self.busy = busy;
    }
}

impl Default for MockAiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AiClient for MockAiClient {
    fn chunk_diff(&mut self, _diff: &str) -> ChunkingResult {
        use super::types::ChunkingError;
        use crate::models::Section;

        // Check for concurrent call (consistent with ClaudeClient)
        if self.busy {
            return ChunkingResult::Error(ChunkingError::new(
                "AI client is busy with another operation",
            ));
        }

        self.busy = true;

        // Return stub sections for testing
        let result = ChunkingResult::Success(vec![
            Section::new(
                "sec_1".to_string(),
                "Core Data Models".to_string(),
                "Defines the fundamental types used throughout the application".to_string(),
                "// stub code for section 1".to_string(),
            ),
            Section::new(
                "sec_2".to_string(),
                "API Handlers".to_string(),
                "HTTP request handlers for the REST API endpoints".to_string(),
                "// stub code for section 2".to_string(),
            ),
            Section::new(
                "sec_3".to_string(),
                "Utility Functions".to_string(),
                "Helper functions for string manipulation and validation".to_string(),
                "// stub code for section 3".to_string(),
            ),
        ]);

        self.busy = false;
        result
    }

    fn assess_hypothesis(&mut self, _code: &str, _hypothesis: &str) -> AssessmentResult {
        use super::types::AssessmentError;
        use crate::models::Assessment;

        // Check for concurrent call (consistent with ClaudeClient)
        if self.busy {
            return AssessmentResult::Error(AssessmentError::new(
                "AI client is busy with another operation",
            ));
        }

        self.busy = true;

        // Return stub assessment for testing
        let result = AssessmentResult::Success(Assessment {
            correct: vec![
                "Correctly identified the main purpose of the function".to_string(),
                "Understood the error handling flow".to_string(),
            ],
            diverges: vec!["The function actually returns early on empty input".to_string()],
            missed: vec!["The function also validates the input format".to_string()],
        });

        self.busy = false;
        result
    }

    fn is_busy(&self) -> bool {
        self.busy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_client_chunk_diff() {
        let mut client = MockAiClient::new();
        let result = client.chunk_diff("some diff");
        assert!(result.is_success());

        let sections = result.sections().unwrap();
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].title, "Core Data Models");
    }

    #[test]
    fn test_mock_client_is_busy() {
        let client = MockAiClient::new();
        assert!(!client.is_busy());
    }

    #[test]
    fn test_mock_client_rejects_concurrent_chunk_diff() {
        let mut client = MockAiClient::new();
        client.set_busy(true);

        let result = client.chunk_diff("some diff");
        assert!(!result.is_success());
    }

    #[test]
    fn test_mock_client_rejects_concurrent_assess_hypothesis() {
        let mut client = MockAiClient::new();
        client.set_busy(true);

        let result = client.assess_hypothesis("code", "hypothesis");
        assert!(!result.is_success());
    }

    #[test]
    fn test_concurrent_rejection_then_success_after_completion() {
        let mut client = MockAiClient::new();

        // Call 1 starts (simulated by setting busy)
        client.set_busy(true);
        assert!(client.is_busy());

        // Call 2 is rejected while call 1 is in progress
        let result2 = client.chunk_diff("second diff");
        assert!(!result2.is_success());

        // Call 1 completes (simulated by clearing busy)
        client.set_busy(false);
        assert!(!client.is_busy());

        // Call 3 succeeds after call 1 completes
        let result3 = client.chunk_diff("third diff");
        assert!(result3.is_success());
        let sections = result3.sections().unwrap();
        assert_eq!(sections.len(), 3);
    }

    #[test]
    fn test_assess_concurrent_rejection_then_success_after_completion() {
        let mut client = MockAiClient::new();

        // Call 1 starts (simulated by setting busy)
        client.set_busy(true);
        assert!(client.is_busy());

        // Call 2 is rejected while call 1 is in progress
        let result2 = client.assess_hypothesis("code", "hypothesis");
        assert!(!result2.is_success());

        // Call 1 completes (simulated by clearing busy)
        client.set_busy(false);
        assert!(!client.is_busy());

        // Call 3 succeeds after call 1 completes
        let result3 = client.assess_hypothesis("code", "hypothesis");
        assert!(result3.is_success());
        let assessment = result3.assessment().unwrap();
        assert!(!assessment.correct.is_empty());
    }
}
