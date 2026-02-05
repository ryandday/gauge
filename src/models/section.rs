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
    /// Get the keyboard shortcut badge for this tag
    pub fn badge(&self) -> &'static str {
        match self {
            Tag::Untagged => "-",
            Tag::GotIt => "1",
            Tag::Shaky => "2",
            Tag::Lost => "3",
        }
    }

    /// Get the human-readable label for this tag
    pub fn label(&self) -> &'static str {
        match self {
            Tag::Untagged => "untagged",
            Tag::GotIt => "got it",
            Tag::Shaky => "shaky",
            Tag::Lost => "lost",
        }
    }
}

/// Source of code content for a code block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CodeSource {
    Diff {
        paths: Vec<String>,
        hunks: Option<Vec<usize>>,
        lines: Option<(usize, usize)>,
    },
    File {
        path: String,
        lines: Option<(usize, usize)>,
    },
}

/// A block of code content within a section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    pub id: String,
    pub source: CodeSource,
    pub content: String,
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

    /// Code blocks containing the actual code/diff content
    pub code_blocks: Vec<CodeBlock>,

    /// Files affected by this section (auto-derived from code_blocks)
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
            code_blocks: if code.is_empty() {
                Vec::new()
            } else {
                vec![CodeBlock {
                    id: "code_1".to_string(),
                    source: CodeSource::Diff {
                        paths: Vec::new(),
                        hunks: None,
                        lines: None,
                    },
                    content: code,
                }]
            },
            files: Vec::new(),
            tag: Tag::default(),
            hypothesis: None,
            assessment: None,
        }
    }

    /// Returns concatenated content of all code blocks
    pub fn code(&self) -> String {
        self.code_blocks
            .iter()
            .map(|cb| cb.content.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Derive files list from code_blocks
    pub fn derive_files(&mut self) {
        let mut files = Vec::new();
        for cb in &self.code_blocks {
            match &cb.source {
                CodeSource::Diff { paths, .. } => {
                    for p in paths {
                        if !files.contains(p) {
                            files.push(p.clone());
                        }
                    }
                }
                CodeSource::File { path, .. } => {
                    if !files.contains(path) {
                        files.push(path.clone());
                    }
                }
            }
        }
        self.files = files;
    }

    /// Returns true if this section needs deep review (shaky or lost)
    pub fn needs_review(&self) -> bool {
        matches!(self.tag, Tag::Shaky | Tag::Lost)
    }

    /// Returns true if this section has been reviewed (has an assessment)
    pub fn is_reviewed(&self) -> bool {
        self.assessment.is_some()
    }

    /// Auto-generate the next code block ID
    pub fn next_code_id(&self) -> String {
        let max = self
            .code_blocks
            .iter()
            .filter_map(|cb| {
                cb.id
                    .strip_prefix("code_")
                    .and_then(|n| n.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(0);
        format!("code_{}", max + 1)
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
    fn test_section_code_concatenation() {
        let mut section = Section::new(
            "s1".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            String::new(),
        );
        section.code_blocks = vec![
            CodeBlock {
                id: "code_1".to_string(),
                source: CodeSource::Diff {
                    paths: vec!["a.rs".to_string()],
                    hunks: None,
                    lines: None,
                },
                content: "block 1".to_string(),
            },
            CodeBlock {
                id: "code_2".to_string(),
                source: CodeSource::File {
                    path: "b.rs".to_string(),
                    lines: None,
                },
                content: "block 2".to_string(),
            },
        ];
        assert_eq!(section.code(), "block 1\nblock 2");
    }

    #[test]
    fn test_section_derive_files() {
        let mut section = Section::new(
            "s1".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            String::new(),
        );
        section.code_blocks = vec![
            CodeBlock {
                id: "code_1".to_string(),
                source: CodeSource::Diff {
                    paths: vec!["a.rs".to_string(), "b.rs".to_string()],
                    hunks: None,
                    lines: None,
                },
                content: "".to_string(),
            },
            CodeBlock {
                id: "code_2".to_string(),
                source: CodeSource::File {
                    path: "a.rs".to_string(),
                    lines: None,
                },
                content: "".to_string(),
            },
        ];
        section.derive_files();
        assert_eq!(section.files, vec!["a.rs", "b.rs"]);
    }

    #[test]
    fn test_next_code_id() {
        let mut section = Section::new(
            "s1".to_string(),
            "Test".to_string(),
            "Desc".to_string(),
            String::new(),
        );
        assert_eq!(section.next_code_id(), "code_1");

        section.code_blocks.push(CodeBlock {
            id: "code_1".to_string(),
            source: CodeSource::Diff {
                paths: vec![],
                hunks: None,
                lines: None,
            },
            content: "".to_string(),
        });
        assert_eq!(section.next_code_id(), "code_2");
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
        let section = Section::new(
            "s1".to_string(),
            "日本語タイトル 🚀".to_string(),
            "\"quotes\" and 'apostrophes' with\nnewlines".to_string(),
            r#"fn main() { println!("Hello, 世界!"); }"#.to_string(),
        );

        let json = serde_json::to_string(&section).unwrap();
        let deserialized: Section = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.title, "日本語タイトル 🚀");
        assert!(deserialized.description.contains("\"quotes\""));
        assert!(deserialized.code().contains("世界"));
    }
}
