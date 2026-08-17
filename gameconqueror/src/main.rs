//! Slim entry point: parse CLI, resolve settings, init logging, hand off to `app`.

use std::process::ExitCode;

use clap::Parser;
use gameconqueror::{app, cli, logger, settings};

fn main() -> ExitCode {
    let args = cli::CliArgs::parse();
    let settings = match settings::resolve(&args) {
        Ok(settings) => settings,
        Err(err) => {
            eprintln!("gameconqueror: {err}");
            return ExitCode::FAILURE;
        }
    };

    logger::init(&settings);
    app::run(settings)
}
