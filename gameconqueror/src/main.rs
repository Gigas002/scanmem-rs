//! Slim entry point: parse CLI, resolve settings, init logging, hand off to `app`.

use std::process::ExitCode;

use clap::Parser;
use gameconqueror::{app, cli, logger, settings};

fn main() -> ExitCode {
    let args = cli::CliArgs::parse();
    let settings = settings::resolve(&args);

    logger::init(&settings);
    app::run(settings)
}
