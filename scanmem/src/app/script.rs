//! One-shot script runner: same `Command`/`Session` path as the REPL, driven by `--exec` instead
//! of `rustyline`.

use std::process::ExitCode;

use super::AppState;
use super::repl::dispatch;
use crate::commands::{Command, formatter, parser};

/// Runs every `;`-separated command in `script` against `state`, printing each result the same
/// way the REPL does, and returns the process exit code — [`ExitCode::FAILURE`] if any command
/// failed to parse or dispatch.
pub fn run(mut state: AppState, script: &str) -> ExitCode {
    let mut had_error = false;

    for command_text in script.split(';') {
        let trimmed = command_text.trim();
        if trimmed.is_empty() {
            continue;
        }

        match parser::parse(trimmed) {
            Ok(Command::Quit) => break,
            Ok(command) => {
                let output = dispatch(&mut state, command);
                had_error |= formatter::is_error(&output);
                println!("{output}");
            }
            Err(err) => {
                eprintln!("{}", formatter::error(err));
                had_error = true;
            }
        }
    }

    if had_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
