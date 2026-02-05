use serde::{Deserialize, Serialize};

use super::section::Section;
use super::state::ReviewStage;

/// Persistent session data that survives app restarts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Version for schema migration
    pub version: u32,

    /// Session name (used as filename)
    pub name: String,

    /// Merge-base commit hash (diffs computed on demand from this)
    pub base_ref: String,

    /// Sections (created via CLI or loaded)
    pub sections: Vec<Section>,

    /// Current review stage
    pub stage: ReviewStage,

    /// Index of current section being reviewed (in deep review)
    pub current_section_index: usize,

    /// Draft hypothesis text being composed (saved on each keystroke)
    pub draft_hypothesis: Option<String>,
}

impl Session {
    /// Current schema version
    pub const CURRENT_VERSION: u32 = 2;

    /// Creates a new session with the given name and base ref
    pub fn new(name: String, base_ref: String) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            name,
            base_ref,
            sections: Vec::new(),
            stage: ReviewStage::Loading,
            current_section_index: 0,
            draft_hypothesis: None,
        }
    }

    /// Auto-generate the next section ID
    pub fn next_section_id(&self) -> String {
        let max = self
            .sections
            .iter()
            .filter_map(|s| {
                s.id.strip_prefix("sec_")
                    .and_then(|n| n.parse::<usize>().ok())
            })
            .max()
            .unwrap_or(0);
        format!("sec_{}", max + 1)
    }

    /// Get counts of sections by tag
    pub fn tag_counts(&self) -> TagCounts {
        let mut counts = TagCounts::default();
        for section in &self.sections {
            match section.tag {
                super::Tag::Untagged => counts.untagged += 1,
                super::Tag::GotIt => counts.got_it += 1,
                super::Tag::Shaky => counts.shaky += 1,
                super::Tag::Lost => counts.lost += 1,
            }
        }
        counts
    }

    /// Get counts of sections with hypotheses
    pub fn hypothesis_counts(&self) -> HypothesisCounts {
        let mut counts = HypothesisCounts::default();
        for section in &self.sections {
            if let Some(assessment) = &section.assessment {
                if assessment.diverges.is_empty() && assessment.missed.is_empty() {
                    counts.confirmed += 1;
                } else {
                    counts.corrected += 1;
                }
            }
        }
        counts
    }

    /// Get sections that need review (shaky or lost)
    #[allow(dead_code)]
    pub fn sections_needing_review(&self) -> Vec<&Section> {
        self.sections.iter().filter(|s| s.needs_review()).collect()
    }

    /// Get all unreviewed sections that need review
    #[allow(dead_code)]
    pub fn sections_needing_deep_review(&self) -> Vec<&Section> {
        self.sections
            .iter()
            .filter(|s| s.needs_review() && !s.is_reviewed())
            .collect()
    }

    /// Check if all sections have been tagged
    pub fn all_tagged(&self) -> bool {
        self.sections
            .iter()
            .all(|s| !matches!(s.tag, super::Tag::Untagged))
    }

    /// Check if triage is complete and can proceed
    #[allow(dead_code)]
    pub fn can_proceed_from_triage(&self) -> bool {
        self.all_tagged()
    }

    /// Check if all deep reviews are complete
    #[allow(dead_code)]
    pub fn deep_review_complete(&self) -> bool {
        self.sections
            .iter()
            .filter(|s| s.needs_review())
            .all(|s| s.is_reviewed())
    }
}

#[derive(Debug, Default, Clone)]
pub struct TagCounts {
    pub untagged: usize,
    pub got_it: usize,
    pub shaky: usize,
    pub lost: usize,
}

impl TagCounts {
    pub fn total(&self) -> usize {
        self.untagged + self.got_it + self.shaky + self.lost
    }

    pub fn tagged(&self) -> usize {
        self.got_it + self.shaky + self.lost
    }
}

#[derive(Debug, Default, Clone)]
pub struct HypothesisCounts {
    pub confirmed: usize,
    pub corrected: usize,
}

impl HypothesisCounts {
    /// Total number of hypotheses submitted
    pub fn total(&self) -> usize {
        self.confirmed + self.corrected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Section, Tag};

    #[test]
    fn test_session_new() {
        let session = Session::new("my-review".to_string(), "abc123".to_string());
        assert_eq!(session.name, "my-review");
        assert_eq!(session.base_ref, "abc123");
        assert_eq!(session.version, Session::CURRENT_VERSION);
        assert!(session.sections.is_empty());
    }

    #[test]
    fn test_next_section_id() {
        let mut session = Session::new("test".to_string(), "".to_string());
        assert_eq!(session.next_section_id(), "sec_1");

        session.sections.push(Section::new(
            "sec_1".to_string(),
            "T1".to_string(),
            "".to_string(),
            "".to_string(),
        ));
        assert_eq!(session.next_section_id(), "sec_2");

        session.sections.push(Section::new(
            "sec_5".to_string(),
            "T5".to_string(),
            "".to_string(),
            "".to_string(),
        ));
        assert_eq!(session.next_section_id(), "sec_6");
    }

    #[test]
    fn test_tag_counts() {
        let mut session = Session::new("test".to_string(), "".to_string());
        session.sections = vec![
            Section::new(
                "s1".to_string(),
                "T1".to_string(),
                "".to_string(),
                "".to_string(),
            ),
            Section::new(
                "s2".to_string(),
                "T2".to_string(),
                "".to_string(),
                "".to_string(),
            ),
            Section::new(
                "s3".to_string(),
                "T3".to_string(),
                "".to_string(),
                "".to_string(),
            ),
        ];
        session.sections[0].tag = Tag::GotIt;
        session.sections[1].tag = Tag::Shaky;
        session.sections[2].tag = Tag::Lost;

        let counts = session.tag_counts();
        assert_eq!(counts.got_it, 1);
        assert_eq!(counts.shaky, 1);
        assert_eq!(counts.lost, 1);
        assert_eq!(counts.untagged, 0);
    }

    #[test]
    fn test_all_tagged() {
        let mut session = Session::new("test".to_string(), "".to_string());
        session.sections = vec![Section::new(
            "s1".to_string(),
            "T1".to_string(),
            "".to_string(),
            "".to_string(),
        )];

        assert!(!session.all_tagged());

        session.sections[0].tag = Tag::GotIt;
        assert!(session.all_tagged());
    }

    #[test]
    fn test_session_serialization() {
        let session = Session::new("my-review".to_string(), "abc123".to_string());
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, session.name);
        assert_eq!(deserialized.base_ref, session.base_ref);
    }

    #[test]
    fn test_empty_sections_tag_counts() {
        let session = Session::new("test".to_string(), "".to_string());
        let counts = session.tag_counts();
        assert_eq!(counts.total(), 0);
        assert_eq!(counts.tagged(), 0);
        assert_eq!(counts.untagged, 0);
    }

    #[test]
    fn test_empty_sections_hypothesis_counts() {
        let session = Session::new("test".to_string(), "".to_string());
        let counts = session.hypothesis_counts();
        assert_eq!(counts.total(), 0);
        assert_eq!(counts.confirmed, 0);
        assert_eq!(counts.corrected, 0);
    }

    #[test]
    fn test_empty_sections_needing_review() {
        let session = Session::new("test".to_string(), "".to_string());
        let needing_review = session.sections_needing_review();
        assert!(needing_review.is_empty());
    }

    #[test]
    fn test_empty_sections_all_tagged() {
        let session = Session::new("test".to_string(), "".to_string());
        assert!(session.all_tagged());
    }

    #[test]
    fn test_empty_sections_deep_review_complete() {
        let session = Session::new("test".to_string(), "".to_string());
        assert!(session.deep_review_complete());
    }
}
