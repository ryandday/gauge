mod ai;
mod app;
mod cli;
mod commands;
mod diff_parser;
mod error;
mod git;
mod models;
mod screens;
mod session;

use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, Command};
use error::Result;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init { name } => commands::init(&name),
        Command::Open { name } => commands::open(&name),
        Command::List => commands::list(),
        Command::Done => commands::done(),
        Command::Section { action } => commands::section(action),
        Command::Code { action } => commands::code(action),
        Command::Diff { action } => commands::diff(action),
    }
}
