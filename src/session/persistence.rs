use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::error::{AppError, Result};
use crate::models::Session;

/// Get the sessions directory (~/.sherpa/sessions/)
pub fn sessions_dir() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| AppError::Session("Could not find home directory".to_string()))?;

    let dir = home.join(".sherpa").join("sessions");
    Ok(dir)
}

/// Hash the identifier to create a safe filename.
///
/// This handles special characters like ':' in identifiers (e.g., "commits:5")
/// that aren't allowed in filenames on some systems (Windows).
///
/// Uses hashing rather than character escaping to ensure uniform filename length
/// and avoid edge cases with very long identifiers. Trade-off: filenames are not
/// human-readable, but collision risk is negligible for typical usage patterns.
fn hash_identifier(identifier: &str) -> String {
    let mut hasher = DefaultHasher::new();
    identifier.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Get the path for a session file
pub fn session_path(identifier: &str) -> Result<PathBuf> {
    let dir = sessions_dir()?;
    let hash = hash_identifier(identifier);
    Ok(dir.join(format!("{}.json", hash)))
}

/// Check if a session exists for the given identifier
#[allow(dead_code)] // Utility function for PHASE-4
pub fn session_exists(identifier: &str) -> Result<bool> {
    let path = session_path(identifier)?;
    Ok(path.exists())
}

/// Result of loading a session
pub enum SessionLoadResult {
    /// Successfully loaded an existing session
    Loaded(Session),
    /// Session file was corrupted, offering fresh start
    Corrupted { path: PathBuf, error: String },
    /// No existing session found
    NotFound,
}

/// Load a session from disk
pub fn load_session(identifier: &str) -> Result<SessionLoadResult> {
    let path = session_path(identifier)?;

    if !path.exists() {
        return Ok(SessionLoadResult::NotFound);
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| AppError::Session(format!("Failed to read session file: {}", e)))?;

    match serde_json::from_str::<Session>(&content) {
        Ok(session) => {
            // Verify the identifier matches
            if session.identifier != identifier {
                return Ok(SessionLoadResult::Corrupted {
                    path,
                    error: "Session identifier mismatch".to_string(),
                });
            }
            Ok(SessionLoadResult::Loaded(session))
        }
        Err(e) => Ok(SessionLoadResult::Corrupted {
            path,
            error: format!("Invalid JSON: {}", e),
        }),
    }
}

/// Save a session to disk atomically
///
/// Uses a temporary file and atomic rename to prevent corruption
/// on unexpected quit (Ctrl+C, kill, power loss).
pub fn save_session(session: &Session) -> Result<PathBuf> {
    let dir = sessions_dir()?;

    // Ensure directory exists with restrictive permissions (0700)
    fs::create_dir_all(&dir)
        .map_err(|e| AppError::Session(format!("Failed to create sessions directory: {}", e)))?;

    // Set restrictive permissions on the sessions directory (owner-only access)
    #[cfg(unix)]
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))
        .map_err(|e| AppError::Session(format!("Failed to set directory permissions: {}", e)))?;

    let path = session_path(&session.identifier)?;
    let tmp_path = path.with_extension("tmp");

    // Write to temporary file
    let content = serde_json::to_string_pretty(session)
        .map_err(|e| AppError::Session(format!("Failed to serialize session: {}", e)))?;

    fs::write(&tmp_path, &content)
        .map_err(|e| AppError::Session(format!("Failed to write session file: {}", e)))?;

    // Atomic rename (POSIX guarantees atomicity)
    fs::rename(&tmp_path, &path)
        .map_err(|e| AppError::Session(format!("Failed to save session file: {}", e)))?;

    Ok(path)
}

/// Delete a corrupted session file to allow fresh start
pub fn delete_session(identifier: &str) -> Result<()> {
    let path = session_path(identifier)?;
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| AppError::Session(format!("Failed to delete session: {}", e)))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_identifier() {
        let hash1 = hash_identifier("commits:5");
        let hash2 = hash_identifier("commits:5");
        let hash3 = hash_identifier("commits:6");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 16);
    }

    #[test]
    fn test_session_path() {
        let path = session_path("commits:5").unwrap();
        assert!(path.to_string_lossy().ends_with(".json"));
        assert!(path.to_string_lossy().contains(".sherpa"));
    }

    #[test]
    fn test_save_and_load_session() {
        // Use a unique identifier to avoid conflicts with real sessions
        let identifier = format!("test:{}", std::process::id());
        let mut session = Session::new(identifier.clone(), "test diff".to_string());
        session.sections = vec![];

        // Save
        let path = save_session(&session).unwrap();
        assert!(path.exists());

        // Load
        let result = load_session(&identifier).unwrap();
        match result {
            SessionLoadResult::Loaded(loaded) => {
                assert_eq!(loaded.identifier, identifier);
                assert_eq!(loaded.diff_text, "test diff");
            }
            _ => panic!("Expected Loaded result"),
        }

        // Cleanup
        delete_session(&identifier).unwrap();
    }

    #[test]
    fn test_load_nonexistent_session() {
        let result = load_session("nonexistent:session").unwrap();
        assert!(matches!(result, SessionLoadResult::NotFound));
    }

    #[test]
    fn test_corrupted_session_detection() {
        let identifier = format!("corrupt:{}", std::process::id());
        let path = session_path(&identifier).unwrap();

        // Create parent directory
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        // Write invalid JSON
        fs::write(&path, "{ invalid json }").unwrap();

        let result = load_session(&identifier).unwrap();
        match result {
            SessionLoadResult::Corrupted { error, .. } => {
                assert!(error.contains("Invalid JSON"));
            }
            _ => panic!("Expected Corrupted result"),
        }

        // Cleanup
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_truncated_json_detection() {
        let identifier = format!("truncated:{}", std::process::id());
        let path = session_path(&identifier).unwrap();

        // Create parent directory
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        // Write truncated JSON (simulating partial write)
        fs::write(&path, r#"{"version":1,"identifier":"truncated","diff_text":""#).unwrap();

        let result = load_session(&identifier).unwrap();
        match result {
            SessionLoadResult::Corrupted { error, .. } => {
                assert!(error.contains("Invalid JSON"));
            }
            _ => panic!("Expected Corrupted result for truncated JSON"),
        }

        // Cleanup
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_valid_json_wrong_schema() {
        let identifier = format!("wrongschema:{}", std::process::id());
        let path = session_path(&identifier).unwrap();

        // Create parent directory
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        // Write valid JSON but wrong schema (missing required fields)
        fs::write(&path, r#"{"foo": "bar", "baz": 123}"#).unwrap();

        let result = load_session(&identifier).unwrap();
        match result {
            SessionLoadResult::Corrupted { error, .. } => {
                assert!(error.contains("Invalid JSON"));
            }
            _ => panic!("Expected Corrupted result for wrong schema"),
        }

        // Cleanup
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_identifier_mismatch_detection() {
        let identifier = format!("mismatch:{}", std::process::id());
        let wrong_identifier = format!("wrong:{}", std::process::id());
        let path = session_path(&identifier).unwrap();

        // Create parent directory
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        // Create a valid session with mismatched identifier
        let session = Session::new(wrong_identifier, "test diff".to_string());
        let content = serde_json::to_string(&session).unwrap();
        fs::write(&path, content).unwrap();

        let result = load_session(&identifier).unwrap();
        match result {
            SessionLoadResult::Corrupted { error, .. } => {
                assert!(error.contains("mismatch"));
            }
            _ => panic!("Expected Corrupted result for identifier mismatch"),
        }

        // Cleanup
        fs::remove_file(&path).ok();
    }
}
