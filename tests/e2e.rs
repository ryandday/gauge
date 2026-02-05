//! End-to-end tests for the Code Review TUI
//!
//! These tests verify complete workflows including:
//! - Complete review flow
//! - Session resume functionality
//! - Markdown export

use sherpa::models::{Assessment, CodeBlock, CodeSource, ReviewStage, Section, Session, Tag};
use sherpa::session::{delete_session, load_session, save_session, SessionLoadResult};

/// Helper to create a test session with sections
fn create_test_session(name: &str) -> Session {
    let mut session = Session::new(name.to_string(), "abc123def456".to_string());
    let mut sec1 = Section::new(
        "sec_1".to_string(),
        "Core Models".to_string(),
        "Data model definitions".to_string(),
        String::new(),
    );
    sec1.code_blocks = vec![CodeBlock {
        id: "code_1".to_string(),
        source: CodeSource::Diff {
            paths: vec!["src/models.rs".to_string()],
            hunks: None,
            lines: None,
        },
        content: "+fn new() -> Self".to_string(),
    }];

    let mut sec2 = Section::new(
        "sec_2".to_string(),
        "API Handlers".to_string(),
        "HTTP endpoint handlers".to_string(),
        String::new(),
    );
    sec2.code_blocks = vec![CodeBlock {
        id: "code_1".to_string(),
        source: CodeSource::Diff {
            paths: vec!["src/handlers.rs".to_string()],
            hunks: None,
            lines: None,
        },
        content: "+async fn handle()".to_string(),
    }];

    session.sections = vec![sec1, sec2];
    session
}

/// Test complete review flow: triage -> deep review -> summary
#[test]
fn test_complete_review_flow() {
    let session = create_test_session("e2e-flow-test");

    // Start in loading stage
    assert_eq!(session.stage, ReviewStage::Loading);

    // Simulate triage: tag sections
    let mut session = session;
    session.sections[0].tag = Tag::GotIt;
    session.sections[1].tag = Tag::Shaky;
    session.stage = ReviewStage::Triage;

    // Verify triage state
    assert!(session.all_tagged());
    assert!(session.can_proceed_from_triage());

    // Section needing review
    let needs_review = session.sections_needing_review();
    assert_eq!(needs_review.len(), 1);
    assert_eq!(needs_review[0].title, "API Handlers");

    // Simulate deep review: add assessment
    session.sections[1].hypothesis = Some("Handles HTTP requests".to_string());
    session.sections[1].assessment = Some(Assessment {
        correct: vec!["Correctly identified HTTP handling".to_string()],
        diverges: vec![],
        missed: vec!["Also handles WebSocket".to_string()],
    });
    session.stage = ReviewStage::DeepReview;

    // Verify review completion
    assert!(session.sections[1].is_reviewed());

    // Transition to summary
    session.stage = ReviewStage::Summary;

    // Verify hypothesis counts
    let counts = session.hypothesis_counts();
    assert_eq!(counts.corrected, 1);
}

/// Test all sections "got it" skips to summary
#[test]
fn test_all_got_it_skips_deep_review() {
    let mut session = create_test_session("e2e-all-got-it");

    // Tag all as "got it"
    session.sections[0].tag = Tag::GotIt;
    session.sections[1].tag = Tag::GotIt;

    // Verify no sections need review
    assert!(session.sections_needing_review().is_empty());
    assert!(session.can_proceed_from_triage());

    // Flow should skip directly to summary
    session.stage = ReviewStage::Summary;
    assert_eq!(session.stage, ReviewStage::Summary);
}

/// Test session persistence and resume
#[test]
fn test_session_resume() {
    let name = format!("e2e-resume-{}", std::process::id());

    // Create and save session
    let mut session = create_test_session(&name);
    session.name = name.clone();
    session.sections[0].tag = Tag::Shaky;
    session.stage = ReviewStage::Triage;
    session.draft_hypothesis = Some("partial draft".to_string());

    save_session(&session).expect("Failed to save session");

    // Load and verify
    let result = load_session(&name).expect("Failed to load session");
    match result {
        SessionLoadResult::Loaded(loaded) => {
            assert_eq!(loaded.name, name);
            assert_eq!(loaded.stage, ReviewStage::Triage);
            assert_eq!(loaded.sections[0].tag, Tag::Shaky);
            assert_eq!(loaded.draft_hypothesis, Some("partial draft".to_string()));
        }
        _ => panic!("Expected Loaded result"),
    }

    // Cleanup
    delete_session(&name).ok();
}

/// Test session resume with progress preserved
#[test]
fn test_session_resume_with_review_progress() {
    let name = format!("e2e-resume-progress-{}", std::process::id());

    // Create session with review progress
    let mut session = create_test_session(&name);
    session.name = name.clone();
    session.sections[0].tag = Tag::Lost;
    session.sections[1].tag = Tag::GotIt;
    session.sections[0].hypothesis = Some("My analysis".to_string());
    session.sections[0].assessment = Some(Assessment {
        correct: vec!["Good point".to_string()],
        diverges: vec![],
        missed: vec![],
    });
    session.stage = ReviewStage::DeepReview;
    session.current_section_index = 1;

    save_session(&session).expect("Failed to save");

    // Reload and verify
    let result = load_session(&name).expect("Failed to load");
    match result {
        SessionLoadResult::Loaded(loaded) => {
            assert_eq!(loaded.stage, ReviewStage::DeepReview);
            assert_eq!(loaded.current_section_index, 1);
            assert!(loaded.sections[0].is_reviewed());
            assert_eq!(
                loaded.sections[0].hypothesis,
                Some("My analysis".to_string())
            );
            assert!(loaded.sections[0].assessment.is_some());
        }
        _ => panic!("Expected Loaded result"),
    }

    // Cleanup
    delete_session(&name).ok();
}

/// Test tag counts and hypothesis counts
#[test]
fn test_statistics_calculation() {
    let mut session = create_test_session("e2e-stats");

    // Tag sections
    session.sections[0].tag = Tag::GotIt;
    session.sections[1].tag = Tag::Lost;

    // Verify tag counts
    let tag_counts = session.tag_counts();
    assert_eq!(tag_counts.got_it, 1);
    assert_eq!(tag_counts.lost, 1);
    assert_eq!(tag_counts.total(), 2);

    // Add assessments
    session.sections[1].hypothesis = Some("hypothesis".to_string());
    session.sections[1].assessment = Some(Assessment {
        correct: vec!["correct".to_string()],
        diverges: vec![],
        missed: vec![],
    });

    // This should count as confirmed (no diverges or missed)
    let hyp_counts = session.hypothesis_counts();
    assert_eq!(hyp_counts.confirmed, 1);
    assert_eq!(hyp_counts.corrected, 0);

    // Add another with corrections needed
    session.sections[0].hypothesis = Some("hypothesis 2".to_string());
    session.sections[0].assessment = Some(Assessment {
        correct: vec![],
        diverges: vec!["wrong".to_string()],
        missed: vec![],
    });

    let hyp_counts = session.hypothesis_counts();
    assert_eq!(hyp_counts.confirmed, 1);
    assert_eq!(hyp_counts.corrected, 1);
}

/// Test draft hypothesis preservation
#[test]
fn test_draft_hypothesis_preservation() {
    let name = format!("e2e-draft-{}", std::process::id());

    let mut session = create_test_session(&name);
    session.name = name.clone();
    session.sections[0].tag = Tag::Shaky;
    session.stage = ReviewStage::DeepReview;

    // Simulate typing - draft is saved
    session.draft_hypothesis = Some("partial thought...".to_string());

    // Save (simulating quit mid-typing)
    save_session(&session).expect("Failed to save");

    // Reload and verify draft preserved
    let result = load_session(&name).expect("Failed to load");
    match result {
        SessionLoadResult::Loaded(loaded) => {
            assert_eq!(
                loaded.draft_hypothesis,
                Some("partial thought...".to_string())
            );
        }
        _ => panic!("Expected Loaded result"),
    }

    // Cleanup
    delete_session(&name).ok();
}

/// Test code block concatenation
#[test]
fn test_code_block_concatenation() {
    let mut section = Section::new(
        "sec_1".to_string(),
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
            content: "+line 1\n+line 2".to_string(),
        },
        CodeBlock {
            id: "code_2".to_string(),
            source: CodeSource::File {
                path: "b.rs".to_string(),
                lines: Some((1, 10)),
            },
            content: "fn main() {}".to_string(),
        },
    ];

    let code = section.code();
    assert!(code.contains("+line 1"));
    assert!(code.contains("fn main()"));
}
