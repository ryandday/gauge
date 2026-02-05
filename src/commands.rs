use std::path::{Path, PathBuf};

use crate::app::App;
use crate::cli::{parse_line_range, CodeAction, DiffAction, SectionAction};
use crate::diff_parser;
use crate::error::{AppError, Result};
use crate::git;
use crate::models::{CodeBlock, CodeSource, ReviewStage, Section, Session};
use crate::session::{
    delete_session, list_sessions, load_session, read_active, save_session, validate_name,
    write_active, SessionLoadResult,
};

fn cwd() -> Result<PathBuf> {
    std::env::current_dir()
        .map_err(|e| AppError::Git(format!("Failed to get current directory: {}", e)))
}

/// Compute a context key from repo root + current branch, so each
/// repo+branch combination gets its own active session pointer.
fn active_context_key() -> Result<String> {
    let dir = cwd()?;
    let repo_root = git::get_repo_root(&dir)?;
    let branch = git::get_current_branch(&dir)?;
    let key = format!("{}:{}", repo_root.display(), branch);
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    Ok(format!("{:016x}", hasher.finish()))
}

/// Load the active session or error
fn load_active_session() -> Result<Session> {
    let name = read_active(&active_context_key()?)?
        .ok_or_else(|| AppError::Session("No active session. Run 'sherpa init <name>' first.".to_string()))?;
    match load_session(&name)? {
        SessionLoadResult::Loaded(session) => Ok(session),
        SessionLoadResult::Corrupted { path, error } => Err(AppError::Session(format!(
            "Session file corrupted ({}): {}",
            path.display(),
            error
        ))),
        SessionLoadResult::NotFound => Err(AppError::Session(format!(
            "Active session '{}' not found. Run 'sherpa init <name>' to create one.",
            name
        ))),
    }
}

/// sherpa init <name> [--base <ref>]
pub fn init(name: &str, base: Option<&str>) -> Result<()> {
    validate_name(name)?;

    // Check if session already exists
    match load_session(name)? {
        SessionLoadResult::Loaded(_) => {
            return Err(AppError::Session(format!(
                "Session '{}' already exists. Delete it first or choose a different name.",
                name
            )));
        }
        SessionLoadResult::Corrupted { .. } => {
            // Delete corrupted session and proceed
            delete_session(name)?;
        }
        SessionLoadResult::NotFound => {}
    }

    let dir = cwd()?;
    let base_ref = match base {
        Some(reference) => git::resolve_ref(&dir, reference)?,
        None => git::compute_merge_base(&dir)?,
    };

    let session = Session::new(name.to_string(), base_ref.clone());
    save_session(&session)?;
    write_active(name, &active_context_key()?)?;

    eprintln!("Session '{}' created (base: {})", name, &base_ref[..12.min(base_ref.len())]);
    Ok(())
}

/// sherpa open <name>
pub fn open(name: &str) -> Result<()> {
    let session = match load_session(name)? {
        SessionLoadResult::Loaded(session) => session,
        SessionLoadResult::Corrupted { path, error } => {
            return Err(AppError::Session(format!(
                "Session file corrupted ({}): {}",
                path.display(),
                error
            )));
        }
        SessionLoadResult::NotFound => {
            return Err(AppError::Session(format!(
                "Session '{}' not found. Run 'sherpa init {}' first.",
                name, name
            )));
        }
    };

    // Set as active
    write_active(name, &active_context_key()?)?;

    let mut app = App::new(session);

    // Go to appropriate screen based on stage
    if !app.session().sections.is_empty() {
        let screen = app.session().stage.into();
        app.state_mut().goto(screen);
    }

    // Save on quit
    let result = app.run();

    if let Err(e) = save_session(app.session()) {
        eprintln!();
        eprintln!("==============================================");
        eprintln!("WARNING: Failed to save session: {}", e);
        eprintln!("Your progress has NOT been saved!");
        eprintln!("Session: {}", app.session().name);
        eprintln!("==============================================");
        eprintln!("Press Enter to acknowledge...");
        let _ = std::io::stdin().read_line(&mut String::new());
    }

    result
}

/// sherpa list
pub fn list() -> Result<()> {
    let sessions = list_sessions()?;
    let active = read_active(&active_context_key()?)?;

    if sessions.is_empty() {
        eprintln!("No sessions found.");
        return Ok(());
    }

    for name in &sessions {
        let marker = if active.as_deref() == Some(name) {
            " *"
        } else {
            ""
        };

        // Try to load session for summary info
        match load_session(name)? {
            SessionLoadResult::Loaded(session) => {
                let stage = match session.stage {
                    ReviewStage::Loading => "loading",
                    ReviewStage::Triage => "triage",
                    ReviewStage::DeepReview => "review",
                    ReviewStage::Summary => "summary",
                };
                println!(
                    "{}{} ({}, {} sections)",
                    name,
                    marker,
                    stage,
                    session.sections.len()
                );
            }
            _ => {
                println!("{}{} (corrupted)", name, marker);
            }
        }
    }

    Ok(())
}

/// sherpa done
pub fn done() -> Result<()> {
    let mut session = load_active_session()?;

    if session.sections.is_empty() {
        return Err(AppError::Session(
            "No sections in session. Add sections before marking done.".to_string(),
        ));
    }

    session.stage = ReviewStage::Triage;
    save_session(&session)?;
    eprintln!(
        "Session '{}' marked ready for triage ({} sections).",
        session.name,
        session.sections.len()
    );
    Ok(())
}

/// sherpa section <action>
pub fn section(action: SectionAction) -> Result<()> {
    match action {
        SectionAction::Add { title, description } => section_add(&title, &description),
        SectionAction::Show { id } => section_show(&id),
        SectionAction::List => section_list(),
        SectionAction::Delete { id } => section_delete(&id),
        SectionAction::Reorder { ids } => section_reorder(&ids),
        SectionAction::Update {
            id,
            title,
            description,
        } => section_update(&id, title.as_deref(), description.as_deref()),
    }
}

fn section_add(title: &str, description: &str) -> Result<()> {
    let mut session = load_active_session()?;
    let id = session.next_section_id();

    let section = Section::new(
        id.clone(),
        title.to_string(),
        description.to_string(),
        String::new(),
    );
    session.sections.push(section);
    save_session(&session)?;

    println!("{}", id);
    Ok(())
}

fn section_show(id: &str) -> Result<()> {
    let session = load_active_session()?;
    let section = session
        .sections
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| AppError::Session(format!("Section '{}' not found", id)))?;

    println!("ID: {}", section.id);
    println!("Title: {}", section.title);
    println!("Description: {}", section.description);
    println!("Tag: {}", section.tag.label());
    println!("Code blocks: {}", section.code_blocks.len());
    println!("Files: {}", section.files.join(", "));
    Ok(())
}

fn section_list() -> Result<()> {
    let session = load_active_session()?;

    if session.sections.is_empty() {
        eprintln!("No sections.");
        return Ok(());
    }

    for section in &session.sections {
        println!(
            "{}: {} ({} code blocks, {})",
            section.id,
            section.title,
            section.code_blocks.len(),
            section.tag.label()
        );
    }
    Ok(())
}

fn section_delete(id: &str) -> Result<()> {
    let mut session = load_active_session()?;
    let idx = session
        .sections
        .iter()
        .position(|s| s.id == id)
        .ok_or_else(|| AppError::Session(format!("Section '{}' not found", id)))?;

    session.sections.remove(idx);
    save_session(&session)?;
    eprintln!("Deleted section '{}'", id);
    Ok(())
}

fn section_reorder(ids: &[String]) -> Result<()> {
    let mut session = load_active_session()?;

    // Validate all IDs exist
    for id in ids {
        if !session.sections.iter().any(|s| s.id == *id) {
            return Err(AppError::Session(format!("Section '{}' not found", id)));
        }
    }

    // Check all sections are mentioned
    if ids.len() != session.sections.len() {
        return Err(AppError::Session(format!(
            "Expected {} section IDs, got {}. All sections must be included.",
            session.sections.len(),
            ids.len()
        )));
    }

    // Reorder
    let mut reordered = Vec::with_capacity(session.sections.len());
    for id in ids {
        let idx = session.sections.iter().position(|s| s.id == *id).unwrap();
        reordered.push(session.sections[idx].clone());
    }
    session.sections = reordered;
    save_session(&session)?;
    eprintln!("Sections reordered.");
    Ok(())
}

fn section_update(id: &str, title: Option<&str>, description: Option<&str>) -> Result<()> {
    let mut session = load_active_session()?;
    let section = session
        .sections
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or_else(|| AppError::Session(format!("Section '{}' not found", id)))?;

    if let Some(t) = title {
        section.title = t.to_string();
    }
    if let Some(d) = description {
        section.description = d.to_string();
    }

    save_session(&session)?;
    eprintln!("Section '{}' updated.", id);
    Ok(())
}

/// sherpa code <action>
pub fn code(action: CodeAction) -> Result<()> {
    match action {
        CodeAction::Add {
            section_id,
            only,
            file,
            hunks,
            lines,
        } => code_add(&section_id, only.as_deref(), file.as_deref(), hunks, lines.as_deref()),
        CodeAction::Show {
            section_id,
            code_id,
        } => code_show(&section_id, &code_id),
        CodeAction::List { section_id } => code_list(&section_id),
        CodeAction::Delete {
            section_id,
            code_id,
        } => code_delete(&section_id, &code_id),
        CodeAction::Reorder { section_id, ids } => code_reorder(&section_id, &ids),
        CodeAction::Update {
            section_id,
            code_id,
            only,
            file,
            hunks,
            lines,
        } => code_update(
            &section_id,
            &code_id,
            only.as_deref(),
            file.as_deref(),
            hunks,
            lines.as_deref(),
        ),
    }
}

/// Resolve content for a code block from diff or file source
fn resolve_content(
    dir: &Path,
    base_ref: &str,
    only: Option<&str>,
    file: Option<&str>,
    hunks: Option<Vec<usize>>,
    lines_str: Option<&str>,
) -> Result<(CodeSource, String)> {
    let lines = match lines_str {
        Some(s) => Some(parse_line_range(s).map_err(AppError::Session)?),
        None => None,
    };

    match (only, file) {
        (Some(path), None) => {
            // Diff mode
            let diff_text = git::diff_file(dir, base_ref, path)?;
            if diff_text.trim().is_empty() {
                return Err(AppError::Session(format!(
                    "No diff found for '{}'. File may not have changes.",
                    path
                )));
            }

            let content = if hunks.is_some() || lines.is_some() {
                let parsed_hunks = diff_parser::parse_hunks(&diff_text);
                if let Some(ref hunk_indices) = hunks {
                    diff_parser::filter_by_hunks(&parsed_hunks, hunk_indices)?
                } else if let Some((start, end)) = lines {
                    diff_parser::filter_by_lines(&parsed_hunks, start, end)
                } else {
                    diff_text.clone()
                }
            } else {
                diff_text
            };

            let source = CodeSource::Diff {
                paths: vec![path.to_string()],
                hunks,
                lines,
            };
            Ok((source, content))
        }
        (None, Some(path)) => {
            // File mode
            let content = git::read_file_content(dir, path, lines)?;
            let source = CodeSource::File {
                path: path.to_string(),
                lines,
            };
            Ok((source, content))
        }
        (None, None) => Err(AppError::Session(
            "Either --only or --file is required".to_string(),
        )),
        (Some(_), Some(_)) => Err(AppError::Session(
            "--only and --file are mutually exclusive".to_string(),
        )),
    }
}

fn code_add(
    section_id: &str,
    only: Option<&str>,
    file: Option<&str>,
    hunks: Option<Vec<usize>>,
    lines: Option<&str>,
) -> Result<()> {
    let mut session = load_active_session()?;

    let section = session
        .sections
        .iter_mut()
        .find(|s| s.id == section_id)
        .ok_or_else(|| AppError::Session(format!("Section '{}' not found", section_id)))?;

    let (source, content) = resolve_content(&cwd()?, &session.base_ref, only, file, hunks, lines)?;
    let code_id = section.next_code_id();

    section.code_blocks.push(CodeBlock {
        id: code_id.clone(),
        source,
        content,
    });
    section.derive_files();

    // Need to clone base_ref before mutable borrow ends, but we already used session immutably.
    // Actually the borrow of session.base_ref ended when we called resolve_content with a slice.
    // But we need to re-find the section for derive_files. Let's restructure.
    save_session(&session)?;
    println!("{}", code_id);
    Ok(())
}

fn code_show(section_id: &str, code_id: &str) -> Result<()> {
    let session = load_active_session()?;
    let section = session
        .sections
        .iter()
        .find(|s| s.id == section_id)
        .ok_or_else(|| AppError::Session(format!("Section '{}' not found", section_id)))?;
    let block = section
        .code_blocks
        .iter()
        .find(|cb| cb.id == code_id)
        .ok_or_else(|| {
            AppError::Session(format!(
                "Code block '{}' not found in section '{}'",
                code_id, section_id
            ))
        })?;

    println!("{}", block.content);
    Ok(())
}

fn code_list(section_id: &str) -> Result<()> {
    let session = load_active_session()?;
    let section = session
        .sections
        .iter()
        .find(|s| s.id == section_id)
        .ok_or_else(|| AppError::Session(format!("Section '{}' not found", section_id)))?;

    if section.code_blocks.is_empty() {
        eprintln!("No code blocks in section '{}'.", section_id);
        return Ok(());
    }

    for block in &section.code_blocks {
        let source_desc = match &block.source {
            CodeSource::Diff { paths, hunks, lines } => {
                let mut desc = format!("diff: {}", paths.join(", "));
                if let Some(h) = hunks {
                    desc.push_str(&format!(
                        " hunks:{}",
                        h.iter()
                            .map(|n| n.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    ));
                }
                if let Some((s, e)) = lines {
                    desc.push_str(&format!(" lines:{}-{}", s, e));
                }
                desc
            }
            CodeSource::File { path, lines } => {
                let mut desc = format!("file: {}", path);
                if let Some((s, e)) = lines {
                    desc.push_str(&format!(" lines:{}-{}", s, e));
                }
                desc
            }
        };
        let lines_count = block.content.lines().count();
        println!("{}: {} ({} lines)", block.id, source_desc, lines_count);
    }
    Ok(())
}

fn code_delete(section_id: &str, code_id: &str) -> Result<()> {
    let mut session = load_active_session()?;
    let section = session
        .sections
        .iter_mut()
        .find(|s| s.id == section_id)
        .ok_or_else(|| AppError::Session(format!("Section '{}' not found", section_id)))?;

    let idx = section
        .code_blocks
        .iter()
        .position(|cb| cb.id == code_id)
        .ok_or_else(|| {
            AppError::Session(format!(
                "Code block '{}' not found in section '{}'",
                code_id, section_id
            ))
        })?;

    section.code_blocks.remove(idx);
    section.derive_files();
    save_session(&session)?;
    eprintln!("Deleted code block '{}' from section '{}'", code_id, section_id);
    Ok(())
}

fn code_reorder(section_id: &str, ids: &[String]) -> Result<()> {
    let mut session = load_active_session()?;
    let section = session
        .sections
        .iter_mut()
        .find(|s| s.id == section_id)
        .ok_or_else(|| AppError::Session(format!("Section '{}' not found", section_id)))?;

    // Validate all IDs exist
    for id in ids {
        if !section.code_blocks.iter().any(|cb| cb.id == *id) {
            return Err(AppError::Session(format!(
                "Code block '{}' not found in section '{}'",
                id, section_id
            )));
        }
    }

    if ids.len() != section.code_blocks.len() {
        return Err(AppError::Session(format!(
            "Expected {} code block IDs, got {}. All code blocks must be included.",
            section.code_blocks.len(),
            ids.len()
        )));
    }

    let mut reordered = Vec::with_capacity(section.code_blocks.len());
    for id in ids {
        let idx = section.code_blocks.iter().position(|cb| cb.id == *id).unwrap();
        reordered.push(section.code_blocks[idx].clone());
    }
    section.code_blocks = reordered;
    save_session(&session)?;
    eprintln!("Code blocks reordered in section '{}'.", section_id);
    Ok(())
}

fn code_update(
    section_id: &str,
    code_id: &str,
    only: Option<&str>,
    file: Option<&str>,
    hunks: Option<Vec<usize>>,
    lines: Option<&str>,
) -> Result<()> {
    let mut session = load_active_session()?;

    let base_ref = session.base_ref.clone();
    let (source, content) = resolve_content(&cwd()?, &base_ref, only, file, hunks, lines)?;

    let section = session
        .sections
        .iter_mut()
        .find(|s| s.id == section_id)
        .ok_or_else(|| AppError::Session(format!("Section '{}' not found", section_id)))?;

    let block = section
        .code_blocks
        .iter_mut()
        .find(|cb| cb.id == code_id)
        .ok_or_else(|| {
            AppError::Session(format!(
                "Code block '{}' not found in section '{}'",
                code_id, section_id
            ))
        })?;

    block.source = source;
    block.content = content;

    section.derive_files();
    save_session(&session)?;
    eprintln!("Code block '{}' updated.", code_id);
    Ok(())
}

/// sherpa diff <action>
pub fn diff(action: DiffAction) -> Result<()> {
    match action {
        DiffAction::Preview { only } => diff_preview(&only),
    }
}

fn diff_preview(path: &str) -> Result<()> {
    let session = load_active_session()?;
    let diff_text = git::diff_file(&cwd()?, &session.base_ref, path)?;

    if diff_text.trim().is_empty() {
        eprintln!("No changes found for '{}'.", path);
        return Ok(());
    }

    let hunks = diff_parser::parse_hunks(&diff_text);

    if hunks.is_empty() {
        eprintln!("No hunks found in diff for '{}'.", path);
        return Ok(());
    }

    // Print full diff with hunk numbers annotated
    println!("{}", diff_parser::format_hunk_preview(&hunks));

    // Also print the full diff for reference
    println!("--- Full diff ---");
    println!("{}", diff_text);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_content_requires_source() {
        let dir = std::env::current_dir().unwrap();
        let result = resolve_content(&dir, "abc", None, None, None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("--only or --file"));
    }

    #[test]
    fn test_parse_line_range_in_resolve() {
        let dir = std::env::current_dir().unwrap();
        // Invalid line range
        let result = resolve_content(&dir, "abc", Some("foo.rs"), None, None, Some("bad"));
        assert!(result.is_err());
    }
}
