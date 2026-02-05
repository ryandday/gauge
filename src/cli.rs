use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "gauge")]
#[command(about = "Interactive code review TUI for understanding diffs")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create a new review session
    Init {
        /// Session name (alphanumeric, hyphens, underscores)
        name: String,
        /// Base git ref (branch, tag, commit, HEAD~N). Defaults to merge-base with default branch.
        #[arg(long)]
        base: Option<String>,
    },
    /// Launch TUI for a session
    Open {
        /// Session name
        name: String,
    },
    /// List all sessions
    List,
    /// Mark active session ready for triage
    Done,
    /// Manage sections
    Section {
        #[command(subcommand)]
        action: SectionAction,
    },
    /// Manage code blocks within sections
    Code {
        #[command(subcommand)]
        action: CodeAction,
    },
    /// Diff utilities
    Diff {
        #[command(subcommand)]
        action: DiffAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum SectionAction {
    /// Add a new section
    Add {
        /// Section title
        #[arg(long)]
        title: String,
        /// Section description
        #[arg(long)]
        description: String,
    },
    /// Show a section's details
    Show {
        /// Section ID (e.g., sec_1)
        id: String,
    },
    /// List all sections
    List,
    /// Delete a section
    Delete {
        /// Section ID (e.g., sec_1)
        id: String,
    },
    /// Reorder sections
    Reorder {
        /// Section IDs in desired order
        ids: Vec<String>,
    },
    /// Update a section's title or description
    Update {
        /// Section ID (e.g., sec_1)
        id: String,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New description
        #[arg(long)]
        description: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum CodeAction {
    /// Add a code block to a section
    Add {
        /// Section ID (e.g., sec_1)
        section_id: String,
        /// File path for diff mode (git diff from base_ref)
        #[arg(long, conflicts_with = "file")]
        only: Option<String>,
        /// File path for direct file read mode
        #[arg(long, conflicts_with = "only")]
        file: Option<String>,
        /// Hunk indices to include (1-based, comma-separated)
        #[arg(long, value_delimiter = ',')]
        hunks: Option<Vec<usize>>,
        /// Line range in format start-end
        #[arg(long)]
        lines: Option<String>,
    },
    /// Show a code block
    Show {
        /// Section ID
        section_id: String,
        /// Code block ID (e.g., code_1)
        code_id: String,
    },
    /// List code blocks in a section
    List {
        /// Section ID
        section_id: String,
    },
    /// Delete a code block
    Delete {
        /// Section ID
        section_id: String,
        /// Code block ID
        code_id: String,
    },
    /// Move a code block to another section
    Move {
        /// Source section ID
        section_id: String,
        /// Code block ID to move
        code_id: String,
        /// Target section ID
        target_section_id: String,
    },
    /// Reorder code blocks within a section
    Reorder {
        /// Section ID
        section_id: String,
        /// Code block IDs in desired order
        ids: Vec<String>,
    },
    /// Update a code block's content
    Update {
        /// Section ID
        section_id: String,
        /// Code block ID
        code_id: String,
        /// File path for diff mode
        #[arg(long, conflicts_with = "file")]
        only: Option<String>,
        /// File path for direct file read mode
        #[arg(long, conflicts_with = "only")]
        file: Option<String>,
        /// Hunk indices to include (1-based, comma-separated)
        #[arg(long, value_delimiter = ',')]
        hunks: Option<Vec<usize>>,
        /// Line range in format start-end
        #[arg(long)]
        lines: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum DiffAction {
    /// Preview diff for a file with numbered hunks
    Preview {
        /// File path to preview
        #[arg(long)]
        only: String,
    },
}

/// Parse a line range string like "10-25" into (start, end)
pub fn parse_line_range(s: &str) -> Result<(usize, usize), String> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid line range '{}', expected format: start-end", s));
    }
    let start: usize = parts[0]
        .parse()
        .map_err(|_| format!("Invalid start line '{}'", parts[0]))?;
    let end: usize = parts[1]
        .parse()
        .map_err(|_| format!("Invalid end line '{}'", parts[1]))?;
    if start == 0 || end == 0 {
        return Err("Line numbers must be >= 1".to_string());
    }
    if start > end {
        return Err(format!("Start line {} is greater than end line {}", start, end));
    }
    Ok((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }

    #[test]
    fn test_parse_init() {
        let cli = Cli::try_parse_from(["gauge", "init", "my-review"]).unwrap();
        match cli.command {
            Command::Init { name, base } => {
                assert_eq!(name, "my-review");
                assert!(base.is_none());
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_parse_init_with_base() {
        let cli = Cli::try_parse_from(["gauge", "init", "my-review", "--base", "HEAD~2"]).unwrap();
        match cli.command {
            Command::Init { name, base } => {
                assert_eq!(name, "my-review");
                assert_eq!(base, Some("HEAD~2".to_string()));
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_parse_open() {
        let cli = Cli::try_parse_from(["gauge", "open", "my-review"]).unwrap();
        match cli.command {
            Command::Open { name } => assert_eq!(name, "my-review"),
            _ => panic!("Expected Open command"),
        }
    }

    #[test]
    fn test_parse_list() {
        let cli = Cli::try_parse_from(["gauge", "list"]).unwrap();
        assert!(matches!(cli.command, Command::List));
    }

    #[test]
    fn test_parse_done() {
        let cli = Cli::try_parse_from(["gauge", "done"]).unwrap();
        assert!(matches!(cli.command, Command::Done));
    }

    #[test]
    fn test_parse_section_add() {
        let cli = Cli::try_parse_from([
            "gauge", "section", "add",
            "--title", "Models",
            "--description", "Core types",
        ]).unwrap();
        match cli.command {
            Command::Section { action: SectionAction::Add { title, description } } => {
                assert_eq!(title, "Models");
                assert_eq!(description, "Core types");
            }
            _ => panic!("Expected Section Add"),
        }
    }

    #[test]
    fn test_parse_section_list() {
        let cli = Cli::try_parse_from(["gauge", "section", "list"]).unwrap();
        assert!(matches!(cli.command, Command::Section { action: SectionAction::List }));
    }

    #[test]
    fn test_parse_code_add_only() {
        let cli = Cli::try_parse_from([
            "gauge", "code", "add", "sec_1",
            "--only", "src/main.rs",
            "--hunks", "1,2",
        ]).unwrap();
        match cli.command {
            Command::Code { action: CodeAction::Add { section_id, only, file, hunks, lines } } => {
                assert_eq!(section_id, "sec_1");
                assert_eq!(only, Some("src/main.rs".to_string()));
                assert!(file.is_none());
                assert_eq!(hunks, Some(vec![1, 2]));
                assert!(lines.is_none());
            }
            _ => panic!("Expected Code Add"),
        }
    }

    #[test]
    fn test_parse_code_add_file() {
        let cli = Cli::try_parse_from([
            "gauge", "code", "add", "sec_1",
            "--file", "src/main.rs",
            "--lines", "5-20",
        ]).unwrap();
        match cli.command {
            Command::Code { action: CodeAction::Add { section_id, only, file, hunks, lines } } => {
                assert_eq!(section_id, "sec_1");
                assert!(only.is_none());
                assert_eq!(file, Some("src/main.rs".to_string()));
                assert!(hunks.is_none());
                assert_eq!(lines, Some("5-20".to_string()));
            }
            _ => panic!("Expected Code Add"),
        }
    }

    #[test]
    fn test_parse_diff_preview() {
        let cli = Cli::try_parse_from([
            "gauge", "diff", "preview", "--only", "src/main.rs",
        ]).unwrap();
        match cli.command {
            Command::Diff { action: DiffAction::Preview { only } } => {
                assert_eq!(only, "src/main.rs");
            }
            _ => panic!("Expected Diff Preview"),
        }
    }

    #[test]
    fn test_parse_line_range_valid() {
        assert_eq!(parse_line_range("10-25"), Ok((10, 25)));
        assert_eq!(parse_line_range("1-1"), Ok((1, 1)));
    }

    #[test]
    fn test_parse_line_range_invalid() {
        assert!(parse_line_range("abc").is_err());
        assert!(parse_line_range("10").is_err());
        assert!(parse_line_range("25-10").is_err());
        assert!(parse_line_range("0-5").is_err());
    }

    #[test]
    fn test_parse_section_reorder() {
        let cli = Cli::try_parse_from([
            "gauge", "section", "reorder", "sec_2", "sec_1",
        ]).unwrap();
        match cli.command {
            Command::Section { action: SectionAction::Reorder { ids } } => {
                assert_eq!(ids, vec!["sec_2", "sec_1"]);
            }
            _ => panic!("Expected Section Reorder"),
        }
    }

    #[test]
    fn test_parse_code_move() {
        let cli = Cli::try_parse_from([
            "gauge", "code", "move", "sec_1", "code_2", "sec_3",
        ]).unwrap();
        match cli.command {
            Command::Code { action: CodeAction::Move { section_id, code_id, target_section_id } } => {
                assert_eq!(section_id, "sec_1");
                assert_eq!(code_id, "code_2");
                assert_eq!(target_section_id, "sec_3");
            }
            _ => panic!("Expected Code Move"),
        }
    }

    #[test]
    fn test_parse_code_delete() {
        let cli = Cli::try_parse_from([
            "gauge", "code", "delete", "sec_1", "code_2",
        ]).unwrap();
        match cli.command {
            Command::Code { action: CodeAction::Delete { section_id, code_id } } => {
                assert_eq!(section_id, "sec_1");
                assert_eq!(code_id, "code_2");
            }
            _ => panic!("Expected Code Delete"),
        }
    }
}
