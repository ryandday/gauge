// @task(P1-T4) Session data model for persistence
use serde::{Deserialize, Serialize};

use super::section::Section;
use super::state::ReviewStage;

/// Persistent session data that survives app restarts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Version for schema migration
    pub version: u32,

    /// Session identifier (commit count or branch mode)
    pub identifier: String,

    /// Raw diff text (for potential re-chunking)
    pub diff_text: String,

    /// Sections chunked by AI (or empty if not yet chunked)
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
    pub const CURRENT_VERSION: u32 = 1;

    /// Creates a new session with the given identifier
    pub fn new(identifier: String, diff_text: String) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            identifier,
            diff_text,
            sections: Vec::new(),
            stage: ReviewStage::Loading,
            current_section_index: 0,
            draft_hypothesis: None,
        }
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
            if section.assessment.is_some() {
                let assessment = section.assessment.as_ref().unwrap();
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
    pub fn sections_needing_review(&self) -> Vec<&Section> {
        self.sections.iter().filter(|s| s.needs_review()).collect()
    }

    /// Get all unreviewed sections that need review
    #[allow(dead_code)] // Used in PHASE-3 (DeepReviewScreen)
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
    pub fn can_proceed_from_triage(&self) -> bool {
        self.all_tagged()
    }

    /// Check if all deep reviews are complete
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
    #[allow(dead_code)] // Used in PHASE-3 (SummaryScreen)
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
        let session = Session::new("commits:5".to_string(), "diff text".to_string());
        assert_eq!(session.identifier, "commits:5");
        assert_eq!(session.version, Session::CURRENT_VERSION);
        assert!(session.sections.is_empty());
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
        let session = Session::new("commits:3".to_string(), "diff".to_string());
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.identifier, session.identifier);
    }
}
