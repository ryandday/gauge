// @task(P1-T4) Section data model with Tag enum
use serde::{Deserialize, Serialize};

/// Confidence tag for a section
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Tag {
    #[default]
    Untagged,
    GotIt,
    Shaky,
    Lost,
}

impl Tag {
    #[allow(dead_code)] // Used in PHASE-3 (TriageScreen)
    pub fn badge(&self) -> &'static str {
        match self {
            Tag::Untagged => "-",
            Tag::GotIt => "1",
            Tag::Shaky => "2",
            Tag::Lost => "3",
        }
    }

    #[allow(dead_code)] // Used in PHASE-3 (TriageScreen)
    pub fn label(&self) -> &'static str {
        match self {
            Tag::Untagged => "untagged",
            Tag::GotIt => "got it",
            Tag::Shaky => "shaky",
            Tag::Lost => "lost",
        }
    }
}

/// A logical grouping of related code changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    /// Unique identifier for the section
    pub id: String,

    /// Human-readable title describing the section
    pub title: String,

    /// Brief description of what this section contains
    pub description: String,

    /// The actual code diff for this section
    pub code: String,

    /// Files affected by this section
    pub files: Vec<String>,

    /// User's confidence tag
    pub tag: Tag,

    /// User's hypothesis about what this code does
    pub hypothesis: Option<String>,

    /// AI assessment of the user's hypothesis (if submitted)
    pub assessment: Option<Assessment>,
}

impl Section {
    pub fn new(id: String, title: String, description: String, code: String) -> Self {
        Self {
            id,
            title,
            description,
            code,
            files: Vec::new(),
            tag: Tag::default(),
            hypothesis: None,
            assessment: None,
        }
    }

    /// Returns true if this section needs deep review (shaky or lost)
    pub fn needs_review(&self) -> bool {
        matches!(self.tag, Tag::Shaky | Tag::Lost)
    }

    /// Returns true if this section has been reviewed (has an assessment)
    pub fn is_reviewed(&self) -> bool {
        self.assessment.is_some()
    }
}

/// AI assessment of a user's hypothesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assessment {
    /// Points where user's understanding matches the code behavior
    pub correct: Vec<String>,

    /// Points where user's interpretation differs from actual behavior
    pub diverges: Vec<String>,

    /// Code behaviors the user did not identify
    pub missed: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_badges() {
        assert_eq!(Tag::Untagged.badge(), "-");
        assert_eq!(Tag::GotIt.badge(), "1");
        assert_eq!(Tag::Shaky.badge(), "2");
        assert_eq!(Tag::Lost.badge(), "3");
    }

    #[test]
    fn test_section_needs_review() {
        let mut section = Section::new(
            "s1".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            "code".to_string(),
        );

        section.tag = Tag::Untagged;
        assert!(!section.needs_review());

        section.tag = Tag::GotIt;
        assert!(!section.needs_review());

        section.tag = Tag::Shaky;
        assert!(section.needs_review());

        section.tag = Tag::Lost;
        assert!(section.needs_review());
    }

    #[test]
    fn test_section_serialization() {
        let section = Section::new(
            "s1".to_string(),
            "Test Section".to_string(),
            "Description".to_string(),
            "diff --git a/file.rs".to_string(),
        );

        let json = serde_json::to_string(&section).unwrap();
        let deserialized: Section = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, section.id);
        assert_eq!(deserialized.title, section.title);
    }

    #[test]
    fn test_section_serialization_special_chars() {
        // Test Unicode, escape sequences, quotes, and special characters
        let section = Section::new(
            "s1".to_string(),
            "日本語タイトル 🚀".to_string(), // Japanese + emoji
            "\"quotes\" and 'apostrophes' with\nnewlines".to_string(),
            r#"fn main() { println!("Hello, 世界!"); }"#.to_string(),
        );

        let json = serde_json::to_string(&section).unwrap();
        let deserialized: Section = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.title, "日本語タイトル 🚀");
        assert!(deserialized.description.contains("\"quotes\""));
        assert!(deserialized.code.contains("世界"));
    }
}
