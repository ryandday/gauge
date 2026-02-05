use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{AppError, Result};

/// Detects the default branch name (main or master)
pub fn detect_default_branch(dir: &Path) -> Result<String> {
    // Try to get the default branch from git config
    let output = Command::new("git")
        .current_dir(dir)
        .args(["config", "--get", "init.defaultBranch"])
        .output()
        .map_err(|e| AppError::Git(format!("Failed to run git: {}", e)))?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !branch.is_empty() {
            return Ok(branch);
        }
    }

    // Check if 'main' branch exists
    let output = Command::new("git")
        .current_dir(dir)
        .args(["rev-parse", "--verify", "main"])
        .output()
        .map_err(|e| AppError::Git(format!("Failed to run git: {}", e)))?;

    if output.status.success() {
        return Ok("main".to_string());
    }

    // Check if 'master' branch exists
    let output = Command::new("git")
        .current_dir(dir)
        .args(["rev-parse", "--verify", "master"])
        .output()
        .map_err(|e| AppError::Git(format!("Failed to run git: {}", e)))?;

    if output.status.success() {
        return Ok("master".to_string());
    }

    Err(AppError::Git(
        "Could not detect default branch (neither 'main' nor 'master' found)".to_string(),
    ))
}

/// Gets the repository root directory
pub fn get_repo_root(dir: &Path) -> Result<PathBuf> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| AppError::Git(format!("Failed to run git: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Git(format!(
            "Failed to get repo root: {}",
            stderr.trim()
        )));
    }

    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(path_str))
}

/// Gets the current branch name
pub fn get_current_branch(dir: &Path) -> Result<String> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|e| AppError::Git(format!("Failed to run git: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Git(format!(
            "Failed to get current branch: {}",
            stderr.trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Compute the merge-base between the default branch and HEAD.
/// Returns the commit hash.
pub fn compute_merge_base(dir: &Path) -> Result<String> {
    let default_branch = detect_default_branch(dir)?;

    let output = Command::new("git")
        .current_dir(dir)
        .args(["merge-base", &default_branch, "HEAD"])
        .output()
        .map_err(|e| AppError::Git(format!("Failed to find merge base: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Git(format!(
            "Failed to find merge base: {}",
            stderr.trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Resolve an arbitrary ref (branch, tag, HEAD~2, hash) to a full commit hash
pub fn resolve_ref(dir: &Path, reference: &str) -> Result<String> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(["rev-parse", reference])
        .output()
        .map_err(|e| AppError::Git(format!("Failed to run git: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Git(format!(
            "Failed to resolve ref '{}': {}",
            reference,
            stderr.trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run `git diff <base_ref>..HEAD -- <path>` and return the diff text
pub fn diff_file(dir: &Path, base_ref: &str, path: &str) -> Result<String> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(["diff", &format!("{}..HEAD", base_ref), "--", path])
        .output()
        .map_err(|e| AppError::Git(format!("Failed to run git diff: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Git(format!("git diff failed: {}", stderr.trim())));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Read a file's content directly (for --file mode)
pub fn read_file_content(dir: &Path, path: &str, lines: Option<(usize, usize)>) -> Result<String> {
    let full_path = dir.join(path);
    let content = std::fs::read_to_string(&full_path)
        .map_err(|e| AppError::Git(format!("Failed to read file '{}': {}", full_path.display(), e)))?;

    match lines {
        Some((start, end)) => {
            let selected: Vec<&str> = content
                .lines()
                .enumerate()
                .filter(|(i, _)| {
                    let line_num = i + 1; // 1-based
                    line_num >= start && line_num <= end
                })
                .map(|(_, line)| line)
                .collect();
            Ok(selected.join("\n"))
        }
        None => Ok(content),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_default_branch() {
        let dir = std::env::current_dir().unwrap();
        let result = detect_default_branch(&dir);
        assert!(result.is_ok(), "detect_default_branch should succeed in a git repo");
        let branch = result.unwrap();
        assert!(!branch.is_empty(), "branch name should not be empty");
        assert!(
            branch.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '/'),
            "branch name should contain only valid characters, got: {}",
            branch
        );
    }

    #[test]
    fn test_get_current_branch() {
        let dir = std::env::current_dir().unwrap();
        let result = get_current_branch(&dir);
        assert!(result.is_ok(), "get_current_branch should succeed in a git repo");
        let branch = result.unwrap();
        assert!(!branch.is_empty(), "branch name should not be empty");
        assert!(
            branch.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '/'),
            "branch name should contain only valid characters, got: {}",
            branch
        );
    }

    #[test]
    fn test_compute_merge_base() {
        let dir = std::env::current_dir().unwrap();
        let result = compute_merge_base(&dir);
        // This may fail if on the default branch itself, which is OK for the test
        if let Ok(hash) = result {
            assert!(!hash.is_empty());
            // SHA hashes are hex strings
            assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn test_diff_file_nonexistent() {
        let dir = std::env::current_dir().unwrap();
        // Diffing a non-existent file should return empty diff, not error
        let result = diff_file(&dir, "HEAD", "nonexistent_file_xyz.rs");
        // git diff returns success with empty output for non-existent files
        assert!(result.is_ok());
    }

    #[test]
    fn test_read_file_content() {
        let dir = std::env::current_dir().unwrap();
        // Read our own Cargo.toml as a test
        let result = read_file_content(&dir, "Cargo.toml", None);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("[package]"));
    }

    #[test]
    fn test_read_file_content_with_lines() {
        let dir = std::env::current_dir().unwrap();
        let result = read_file_content(&dir, "Cargo.toml", Some((1, 3)));
        assert!(result.is_ok());
        let content = result.unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert!(lines.len() <= 3);
    }

    #[test]
    fn test_read_file_content_nonexistent() {
        let dir = std::env::current_dir().unwrap();
        let result = read_file_content(&dir, "nonexistent_xyz.rs", None);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::process::Command;
    use tempfile::tempdir;

    /// Create a temp git repo with a main branch, a commit, then a feature branch with changes.
    /// Returns (tempdir, main_commit_hash).
    fn setup_temp_repo() -> (tempfile::TempDir, String) {
        let tmp = tempdir().unwrap();
        let dir = tmp.path();

        // git init -b main
        let status = Command::new("git")
            .current_dir(dir)
            .args(["init", "-b", "main"])
            .output()
            .unwrap();
        assert!(status.status.success(), "git init failed");

        // Configure user for commits
        Command::new("git")
            .current_dir(dir)
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(dir)
            .args(["config", "user.name", "Test User"])
            .output()
            .unwrap();

        // Create hello.rs and commit on main
        std::fs::write(dir.join("hello.rs"), "fn main() {\n    println!(\"hello\");\n}\n").unwrap();
        Command::new("git")
            .current_dir(dir)
            .args(["add", "hello.rs"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(dir)
            .args(["commit", "-m", "initial commit"])
            .output()
            .unwrap();

        // Record main commit hash
        let out = Command::new("git")
            .current_dir(dir)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        let main_hash = String::from_utf8_lossy(&out.stdout).trim().to_string();

        // Create feature branch, modify hello.rs, add new_file.rs
        Command::new("git")
            .current_dir(dir)
            .args(["checkout", "-b", "feature"])
            .output()
            .unwrap();
        std::fs::write(
            dir.join("hello.rs"),
            "fn main() {\n    println!(\"hello\");\n    println!(\"world\");\n}\n",
        )
        .unwrap();
        std::fs::write(dir.join("new_file.rs"), "fn new() {}\n").unwrap();
        Command::new("git")
            .current_dir(dir)
            .args(["add", "hello.rs", "new_file.rs"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(dir)
            .args(["commit", "-m", "feature changes"])
            .output()
            .unwrap();

        (tmp, main_hash)
    }

    #[test]
    fn test_detect_default_branch_in_temp_repo() {
        let (tmp, _) = setup_temp_repo();
        let result = detect_default_branch(tmp.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "main");
    }

    #[test]
    fn test_get_repo_root_in_temp_repo() {
        let (tmp, _) = setup_temp_repo();
        let result = get_repo_root(tmp.path());
        assert!(result.is_ok());
        // On macOS /var -> /private/var, so canonicalize both
        let expected = tmp.path().canonicalize().unwrap();
        let actual = result.unwrap().canonicalize().unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_get_current_branch_in_temp_repo() {
        let (tmp, _) = setup_temp_repo();
        let result = get_current_branch(tmp.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "feature");
    }

    #[test]
    fn test_compute_merge_base_in_temp_repo() {
        let (tmp, main_hash) = setup_temp_repo();
        let result = compute_merge_base(tmp.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), main_hash);
    }

    #[test]
    fn test_diff_file_shows_changes() {
        let (tmp, main_hash) = setup_temp_repo();
        let result = diff_file(tmp.path(), &main_hash, "hello.rs");
        assert!(result.is_ok());
        let diff = result.unwrap();
        assert!(diff.contains("world"), "diff should contain the added line");
    }

    #[test]
    fn test_diff_file_new_file() {
        let (tmp, main_hash) = setup_temp_repo();
        let result = diff_file(tmp.path(), &main_hash, "new_file.rs");
        assert!(result.is_ok());
        let diff = result.unwrap();
        assert!(diff.contains("fn new()"), "diff should contain new file content");
    }

    #[test]
    fn test_diff_file_no_changes() {
        let (tmp, _) = setup_temp_repo();
        // Diff HEAD against itself — no changes
        let result = diff_file(tmp.path(), "HEAD", "hello.rs");
        assert!(result.is_ok());
        assert!(result.unwrap().trim().is_empty(), "no diff expected for HEAD..HEAD");
    }

    #[test]
    fn test_read_file_content_from_temp_repo() {
        let (tmp, _) = setup_temp_repo();
        let result = read_file_content(tmp.path(), "hello.rs", None);
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("println!(\"hello\")"));
        assert!(content.contains("println!(\"world\")"));
    }

    #[test]
    fn test_read_file_content_with_line_range() {
        let (tmp, _) = setup_temp_repo();
        // hello.rs has 4 lines; read only lines 2-3
        let result = read_file_content(tmp.path(), "hello.rs", Some((2, 3)));
        assert!(result.is_ok());
        let content = result.unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("println!(\"hello\")"));
        assert!(lines[1].contains("println!(\"world\")"));
    }
}
