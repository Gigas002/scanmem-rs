//! Slim entry point: parse CLI, resolve settings, init logging, hand off to `app`.

mod app;
mod cli;
mod commands;
mod logger;
mod settings;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let args = cli::CliArgs::parse();
    let settings = settings::resolve(&args);
    logger::init(&settings);
    app::run(settings)
}
