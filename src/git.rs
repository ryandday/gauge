use std::path::PathBuf;
use std::process::Command;

use crate::error::{AppError, Result};

/// Detects the default branch name (main or master)
pub fn detect_default_branch() -> Result<String> {
    // Try to get the default branch from git config
    let output = Command::new("git")
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
        .args(["rev-parse", "--verify", "main"])
        .output()
        .map_err(|e| AppError::Git(format!("Failed to run git: {}", e)))?;

    if output.status.success() {
        return Ok("main".to_string());
    }

    // Check if 'master' branch exists
    let output = Command::new("git")
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
pub fn get_repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
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
pub fn get_current_branch() -> Result<String> {
    let output = Command::new("git")
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

/// Reads a git diff for the specified number of commits back
pub fn read_diff_commits(commit_count: usize) -> Result<String> {
    let range = format!("HEAD~{}..HEAD", commit_count);

    let output = Command::new("git")
        .args(["diff", &range])
        .output()
        .map_err(|e| AppError::Git(format!("Failed to run git diff: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Git(format!("git diff failed: {}", stderr)));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Reads a git diff comparing current branch against main/master
pub fn read_diff_branch() -> Result<String> {
    let default_branch = detect_default_branch()?;
    let current_branch = get_current_branch()?;

    if current_branch == default_branch {
        return Err(AppError::Git(format!(
            "Already on {} branch. Use 'sherpa N' to review last N commits instead.",
            default_branch
        )));
    }

    // Find the merge base to get only changes specific to this branch
    let output = Command::new("git")
        .args(["merge-base", &default_branch, "HEAD"])
        .output()
        .map_err(|e| AppError::Git(format!("Failed to find merge base: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Git(format!(
            "Failed to find merge base: {}",
            stderr
        )));
    }

    let merge_base = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let output = Command::new("git")
        .args(["diff", &merge_base, "HEAD"])
        .output()
        .map_err(|e| AppError::Git(format!("Failed to run git diff: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Git(format!("git diff failed: {}", stderr)));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Reads the appropriate diff based on CLI arguments
pub fn read_diff(commits: Option<usize>) -> Result<String> {
    match commits {
        Some(n) => read_diff_commits(n),
        None => read_diff_branch(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_default_branch() {
        // This test runs in a git repo (the sherpa2 repo itself)
        // so detect_default_branch should succeed and return a valid branch name
        let result = detect_default_branch();
        assert!(result.is_ok(), "detect_default_branch should succeed in a git repo");
        let branch = result.unwrap();
        assert!(!branch.is_empty(), "branch name should not be empty");
        // Verify the branch name is a valid git branch name
        // We don't assert specific values like "main" or "master" because:
        // 1. The function works correctly for any configured default branch
        // 2. Asserting specific values tests repo config, not function logic
        assert!(
            branch.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '/'),
            "branch name should contain only valid characters, got: {}",
            branch
        );
    }

    #[test]
    fn test_get_current_branch() {
        // This test runs in a git repo (the sherpa2 repo itself)
        // so get_current_branch should succeed and return a valid branch name
        let result = get_current_branch();
        assert!(result.is_ok(), "get_current_branch should succeed in a git repo");
        let branch = result.unwrap();
        assert!(!branch.is_empty(), "branch name should not be empty");
        // Branch name should be a valid git branch name (alphanumeric, hyphens, underscores, slashes)
        assert!(
            branch.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '/'),
            "branch name should contain only valid characters, got: {}",
            branch
        );
    }

    #[test]
    fn test_read_diff_commits_valid_range() {
        // This test runs in a git repo with at least one commit
        // Reading diff for HEAD~1..HEAD should succeed (may be empty if no recent changes)
        let result = read_diff_commits(1);
        assert!(result.is_ok(), "read_diff_commits(1) should succeed in a git repo with commits");
        // The diff content is a string (may be empty if HEAD~1 and HEAD are the same)
        // unwrap succeeds because we already asserted is_ok() above
        let _diff = result.unwrap();
    }
}
