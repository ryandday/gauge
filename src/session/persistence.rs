use std::fs;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::error::{AppError, Result};
use crate::models::Session;

/// Validate a session name: [a-zA-Z0-9_-]{1,64}
pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 64 {
        return Err(AppError::Session(
            "Session name must be 1-64 characters".to_string(),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::Session(
            "Session name must contain only alphanumeric characters, hyphens, and underscores"
                .to_string(),
        ));
    }
    Ok(())
}

/// Get the sherpa directory (~/.sherpa/)
pub fn sherpa_dir() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| AppError::Session("Could not find home directory".to_string()))?;
    Ok(home.join(".sherpa"))
}

/// Get the sessions directory (~/.sherpa/sessions/)
pub fn sessions_dir() -> Result<PathBuf> {
    Ok(sherpa_dir()?.join("sessions"))
}

/// Get the path for a session file
pub fn session_path(name: &str) -> Result<PathBuf> {
    let dir = sessions_dir()?;
    Ok(dir.join(format!("{}.json", name)))
}

/// Get the path for the active session file
fn active_path() -> Result<PathBuf> {
    Ok(sherpa_dir()?.join("active"))
}

/// Write the active session name
pub fn write_active(name: &str) -> Result<()> {
    let path = active_path()?;
    let dir = path.parent().unwrap();
    fs::create_dir_all(dir)
        .map_err(|e| AppError::Session(format!("Failed to create sherpa directory: {}", e)))?;
    fs::write(&path, name)
        .map_err(|e| AppError::Session(format!("Failed to write active session: {}", e)))?;
    Ok(())
}

/// Read the active session name
pub fn read_active() -> Result<Option<String>> {
    let path = active_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let name = fs::read_to_string(&path)
        .map_err(|e| AppError::Session(format!("Failed to read active session: {}", e)))?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Ok(None);
    }
    Ok(Some(name))
}

/// Check if a session exists for the given name
#[allow(dead_code)]
pub fn session_exists(name: &str) -> Result<bool> {
    let path = session_path(name)?;
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
pub fn load_session(name: &str) -> Result<SessionLoadResult> {
    let path = session_path(name)?;

    if !path.exists() {
        return Ok(SessionLoadResult::NotFound);
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| AppError::Session(format!("Failed to read session file: {}", e)))?;

    match serde_json::from_str::<Session>(&content) {
        Ok(session) => {
            if session.name != name {
                return Ok(SessionLoadResult::Corrupted {
                    path,
                    error: "Session name mismatch".to_string(),
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
pub fn save_session(session: &Session) -> Result<PathBuf> {
    let dir = sessions_dir()?;

    fs::create_dir_all(&dir)
        .map_err(|e| AppError::Session(format!("Failed to create sessions directory: {}", e)))?;

    #[cfg(unix)]
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))
        .map_err(|e| AppError::Session(format!("Failed to set directory permissions: {}", e)))?;

    let path = session_path(&session.name)?;
    let tmp_path = path.with_extension("tmp");

    let content = serde_json::to_string_pretty(session)
        .map_err(|e| AppError::Session(format!("Failed to serialize session: {}", e)))?;

    fs::write(&tmp_path, &content)
        .map_err(|e| AppError::Session(format!("Failed to write session file: {}", e)))?;

    fs::rename(&tmp_path, &path)
        .map_err(|e| AppError::Session(format!("Failed to save session file: {}", e)))?;

    Ok(path)
}

/// Delete a session file
pub fn delete_session(name: &str) -> Result<()> {
    let path = session_path(name)?;
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| AppError::Session(format!("Failed to delete session: {}", e)))?;
    }
    Ok(())
}

/// List all session names (reads the sessions directory)
pub fn list_sessions() -> Result<Vec<String>> {
    let dir = sessions_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    let entries = fs::read_dir(&dir)
        .map_err(|e| AppError::Session(format!("Failed to read sessions directory: {}", e)))?;

    for entry in entries {
        let entry =
            entry.map_err(|e| AppError::Session(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                sessions.push(name.to_string());
            }
        }
    }

    sessions.sort();
    Ok(sessions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_valid() {
        assert!(validate_name("my-review").is_ok());
        assert!(validate_name("test_123").is_ok());
        assert!(validate_name("a").is_ok());
        assert!(validate_name("A-b_C-1").is_ok());
    }

    #[test]
    fn test_validate_name_invalid() {
        assert!(validate_name("").is_err());
        assert!(validate_name("has spaces").is_err());
        assert!(validate_name("has:colon").is_err());
        assert!(validate_name("has/slash").is_err());
        assert!(validate_name(&"a".repeat(65)).is_err());
    }

    #[test]
    fn test_session_path() {
        let path = session_path("my-review").unwrap();
        assert!(path.to_string_lossy().ends_with("my-review.json"));
        assert!(path.to_string_lossy().contains(".sherpa"));
    }

    #[test]
    fn test_save_and_load_session() {
        let name = format!("test-{}", std::process::id());
        let session = Session::new(name.clone(), "abc123".to_string());

        let path = save_session(&session).unwrap();
        assert!(path.exists());

        let result = load_session(&name).unwrap();
        match result {
            SessionLoadResult::Loaded(loaded) => {
                assert_eq!(loaded.name, name);
                assert_eq!(loaded.base_ref, "abc123");
            }
            _ => panic!("Expected Loaded result"),
        }

        delete_session(&name).unwrap();
    }

    #[test]
    fn test_load_nonexistent_session() {
        let result = load_session("nonexistent-session-xyz").unwrap();
        assert!(matches!(result, SessionLoadResult::NotFound));
    }

    #[test]
    fn test_corrupted_session_detection() {
        let name = format!("corrupt-{}", std::process::id());
        let path = session_path(&name).unwrap();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        fs::write(&path, "{ invalid json }").unwrap();

        let result = load_session(&name).unwrap();
        match result {
            SessionLoadResult::Corrupted { error, .. } => {
                assert!(error.contains("Invalid JSON"));
            }
            _ => panic!("Expected Corrupted result"),
        }

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_name_mismatch_detection() {
        let name = format!("mismatch-{}", std::process::id());
        let path = session_path(&name).unwrap();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        let session = Session::new("wrong-name".to_string(), "".to_string());
        let content = serde_json::to_string(&session).unwrap();
        fs::write(&path, content).unwrap();

        let result = load_session(&name).unwrap();
        match result {
            SessionLoadResult::Corrupted { error, .. } => {
                assert!(error.contains("mismatch"));
            }
            _ => panic!("Expected Corrupted result for name mismatch"),
        }

        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_write_and_read_active() {
        write_active("test-session").unwrap();
        let active = read_active().unwrap();
        assert_eq!(active, Some("test-session".to_string()));
    }

    #[test]
    fn test_list_sessions() {
        let result = list_sessions();
        assert!(result.is_ok());
        // Just verify it returns a list (may be empty or have test sessions)
    }
}
