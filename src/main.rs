mod ai;
mod app;
mod cli;
mod error;
mod git;
mod models;
mod screens;
mod session;

use std::process::ExitCode;

use ai::{AiClient, ChunkingResult, ClaudeClient};
use app::App;
use cli::Args;
use error::{AppError, Result};
use models::{Screen, Session};
use session::{delete_session, load_session, save_session, SessionLoadResult};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(AppError::Git(msg)) if msg == "No changes to review" => {
            // Clean exit for empty diff - not an error condition
            eprintln!("No changes to review.");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args = Args::parse_args();
    let identifier = args.session_identifier();

    // Handle session loading/creation
    let session = if args.new {
        // --new flag: force fresh session
        // delete_session returns Ok(()) for non-existent files, so any error is real
        if let Err(e) = delete_session(&identifier) {
            eprintln!("Warning: Could not delete old session: {}", e);
        }
        create_new_session(&args)?
    } else {
        // Try to load existing session
        match load_session(&identifier)? {
            SessionLoadResult::Loaded(session) => {
                eprintln!("Resuming previous session...");
                session
            }
            SessionLoadResult::Corrupted { path, error } => {
                eprintln!("Session file corrupted: {}", error);
                eprintln!("Path: {}", path.display());
                eprintln!();

                // Offer fresh start
                if confirm_fresh_start()? {
                    delete_session(&identifier)?;
                    create_new_session(&args)?
                } else {
                    return Ok(());
                }
            }
            SessionLoadResult::NotFound => create_new_session(&args)?,
        }
    };

    // Create and run the app
    let mut app = App::new(session);

    // If this is a new session with diff, start loading/chunking with real AI
    if app.session().sections.is_empty() && !app.session().diff_text.is_empty() {
        // Use real Claude client for AI chunking
        let mut client = ClaudeClient::new();
        match client.chunk_diff(&app.session().diff_text) {
            ChunkingResult::Success(sections) => {
                if sections.is_empty() {
                    return Err(AppError::Ai("AI returned no sections. The diff may be too small or unclear.".to_string()));
                }
                app.session_mut().sections = sections;
                app.state_mut().goto(Screen::Triage);
            }
            ChunkingResult::Error(e) => {
                // Show error in loading screen
                app.state_mut().ui.set_error(e.message);
                // App will stay on Loading screen with error displayed
            }
        }
    } else if !app.session().sections.is_empty() {
        // Resuming session - go to appropriate screen based on stage
        let screen = app.session().stage.into();
        app.state_mut().goto(screen);
    }

    // Run the main event loop
    let result = app.run();

    // Save session on quit (whether normal or error)
    // Make save failures very visible since user may lose progress
    if let Err(e) = save_session(app.session()) {
        eprintln!();
        eprintln!("==============================================");
        eprintln!("WARNING: Failed to save session: {}", e);
        eprintln!("Your progress has NOT been saved!");
        eprintln!("Session: {}", app.session().identifier);
        eprintln!("==============================================");
        eprintln!("Press Enter to acknowledge...");
        let _ = std::io::stdin().read_line(&mut String::new());
    }

    result
}

fn create_new_session(args: &Args) -> Result<Session> {
    let identifier = args.session_identifier();

    // Read the git diff
    let diff_text = git::read_diff(args.commits)?;

    if diff_text.trim().is_empty() {
        // Return an error instead of exiting - allows clean exit through normal flow
        return Err(AppError::Git("No changes to review".to_string()));
    }

    Ok(Session::new(identifier, diff_text))
}

fn confirm_fresh_start() -> Result<bool> {
    use std::io::{self, Write};

    eprint!("Start fresh session? [y/N] ");
    io::stderr().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("y"))
}
