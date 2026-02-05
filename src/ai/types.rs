// Result types for AI operations (chunking and assessment) used throughout the application.

use crate::models::{Assessment, Section};

/// Result of AI diff chunking operation
#[derive(Debug)]
#[allow(dead_code)]
pub enum ChunkingResult {
    /// Successfully chunked the diff into sections
    Success(Vec<Section>),
    /// Failed to chunk the diff
    Error(ChunkingError),
}

#[allow(dead_code)] // Used in tests
impl ChunkingResult {
    pub fn is_success(&self) -> bool {
        matches!(self, ChunkingResult::Success(_))
    }

    pub fn sections(self) -> Option<Vec<Section>> {
        match self {
            ChunkingResult::Success(sections) => Some(sections),
            ChunkingResult::Error(_) => None,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ChunkingError {
    pub message: String,
    pub raw_output: Option<String>,
}

#[allow(dead_code)]
impl ChunkingError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            raw_output: None,
        }
    }

    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.raw_output = Some(output.into());
        self
    }
}

/// Result of AI hypothesis assessment operation
#[derive(Debug)]
pub enum AssessmentResult {
    /// Successfully assessed the hypothesis
    Success(Assessment),
    /// Failed to assess the hypothesis
    Error(AssessmentError),
}

#[allow(dead_code)] // Used in tests
impl AssessmentResult {
    pub fn is_success(&self) -> bool {
        matches!(self, AssessmentResult::Success(_))
    }

    pub fn assessment(self) -> Option<Assessment> {
        match self {
            AssessmentResult::Success(assessment) => Some(assessment),
            AssessmentResult::Error(_) => None,
        }
    }
}

#[derive(Debug)]
pub struct AssessmentError {
    pub message: String,
    pub raw_output: Option<String>,
}

impl AssessmentError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            raw_output: None,
        }
    }

    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.raw_output = Some(output.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunking_result_success() {
        let result = ChunkingResult::Success(vec![]);
        assert!(result.is_success());
    }

    #[test]
    fn test_chunking_result_error() {
        let result = ChunkingResult::Error(ChunkingError::new("test error"));
        assert!(!result.is_success());
    }

    #[test]
    fn test_assessment_result_success() {
        let assessment = Assessment {
            correct: vec![],
            diverges: vec![],
            missed: vec![],
        };
        let result = AssessmentResult::Success(assessment);
        assert!(result.is_success());
    }
}
