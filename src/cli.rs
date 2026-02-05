use clap::Parser;

fn parse_commit_count(s: &str) -> Result<usize, String> {
    let n: usize = s.parse().map_err(|_| format!("'{}' is not a valid number", s))?;
    if !(1..1000).contains(&n) {
        return Err(format!("commit count must be between 1 and 999, got {}", n));
    }
    Ok(n)
}

#[derive(Parser, Debug, Clone)]
#[command(name = "sherpa")]
#[command(about = "Interactive code review TUI for understanding diffs")]
#[command(version)]
pub struct Args {
    /// Number of commits to review (e.g., 5 for HEAD~5..HEAD)
    /// If not provided, compares current branch against main/master
    #[arg(value_parser = parse_commit_count)]
    pub commits: Option<usize>,

    /// Force a fresh session instead of resuming a previous one
    #[arg(long)]
    pub new: bool,
}

impl Args {
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Generate a session identifier based on the input mode
    pub fn session_identifier(&self) -> String {
        match self.commits {
            Some(n) => format!("commits:{}", n),
            None => "branch".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_parsing_commit_count() {
        let args = Args::try_parse_from(["sherpa", "5"]).unwrap();
        assert_eq!(args.commits, Some(5));
        assert!(!args.new);
    }

    #[test]
    fn test_cli_parsing_no_args() {
        let args = Args::try_parse_from(["sherpa"]).unwrap();
        assert_eq!(args.commits, None);
        assert!(!args.new);
    }

    #[test]
    fn test_cli_parsing_new_flag() {
        let args = Args::try_parse_from(["sherpa", "--new"]).unwrap();
        assert_eq!(args.commits, None);
        assert!(args.new);
    }

    #[test]
    fn test_cli_parsing_new_flag_with_commits() {
        let args = Args::try_parse_from(["sherpa", "--new", "3"]).unwrap();
        assert_eq!(args.commits, Some(3));
        assert!(args.new);
    }

    #[test]
    fn test_cli_invalid_flag_shows_error() {
        let result = Args::try_parse_from(["sherpa", "--bad-flag"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_session_identifier_with_commits() {
        let args = Args {
            commits: Some(5),
            new: false,
        };
        assert_eq!(args.session_identifier(), "commits:5");
    }

    #[test]
    fn test_session_identifier_branch_mode() {
        let args = Args {
            commits: None,
            new: false,
        };
        assert_eq!(args.session_identifier(), "branch");
    }

    #[test]
    fn verify_cli() {
        Args::command().debug_assert();
    }

    #[test]
    fn test_parse_commit_count_zero_rejected() {
        let result = parse_commit_count("0");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("between 1 and 999"));
    }

    #[test]
    fn test_parse_commit_count_one_accepted() {
        let result = parse_commit_count("1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_parse_commit_count_999_accepted() {
        let result = parse_commit_count("999");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 999);
    }

    #[test]
    fn test_parse_commit_count_1000_rejected() {
        let result = parse_commit_count("1000");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("between 1 and 999"));
    }

    #[test]
    fn test_parse_commit_count_invalid_string() {
        let result = parse_commit_count("abc");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a valid number"));
    }
}
