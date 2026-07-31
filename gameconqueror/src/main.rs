//! Slim entry point: parse CLI, resolve settings, init logging, hand off to `app`.

mod app;
mod cli;
mod logger;
mod settings;

#[cfg(feature = "tui")]
mod ui;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let args = cli::CliArgs::parse();
    let settings = settings::resolve(&args);

    logger::init(&settings);
    app::run(settings)
}
