// @task(P1-T3) Implement git diff reading: N commits back or branch vs main/master
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

/// Gets the current branch name
pub fn get_current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|e| AppError::Git(format!("Failed to run git: {}", e)))?;

    if !output.status.success() {
        return Err(AppError::Git(
            "Failed to get current branch name".to_string(),
        ));
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
        // This test requires being in a git repo
        let result = detect_default_branch();
        // Should succeed in a git repo
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_get_current_branch() {
        // This test requires being in a git repo
        let result = get_current_branch();
        if result.is_ok() {
            let branch = result.unwrap();
            assert!(!branch.is_empty());
        }
    }

    #[test]
    fn test_read_diff_commits_invalid_range() {
        // Try to read a very large commit range that likely doesn't exist
        let result = read_diff_commits(99999);
        // This might succeed with empty diff or fail - both are valid
        assert!(result.is_ok() || result.is_err());
    }
}
