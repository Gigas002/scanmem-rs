//! Ratatui shell: terminal lifecycle (raw mode, alternate screen, panic recovery) and the
//! top-level event loop. Panel-specific rendering lives in sibling files (`layout.rs`, ...).

#[cfg(feature = "cheat-list")]
mod cheat_view;
mod help_overlay;
mod hex_view;
mod input;
mod keymap;
mod layout;
mod match_view;
mod process_picker;
mod scan_panel;

use std::io;
use std::process::ExitCode;
use std::time::Duration;

use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

use crate::app::{AppState, Msg};
use crate::settings::Settings;

/// Enters raw mode + the alternate screen on construction, restores the terminal on drop —
/// covers both clean exits and unwinding panics without leaving the terminal broken.
struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

/// Installs a panic hook that restores the terminal *before* the default hook prints the panic,
/// so a panic mid-render never leaves the shell in raw mode / the alternate screen.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));
}

/// Runs the `ratatui` shell: process picker, status bar, quitting cleanly on `Ctrl+Q`.
pub fn run(state: &mut AppState, settings: &Settings) -> ExitCode {
    let _ = settings;
    install_panic_hook();
    crate::app::update(state, Msg::RefreshProcessList);

    let guard = match TerminalGuard::enter() {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("gameconqueror: failed to initialize terminal: {err}");
            return ExitCode::FAILURE;
        }
    };

    let result = run_event_loop(state);
    drop(guard);

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("gameconqueror: {err}");
            ExitCode::FAILURE
        }
    }
}

/// How often the event loop wakes up when no key was pressed, to rewrite frozen cheat-list
/// entries (with the `cheat-list` feature) and to keep redrawing/polling a background scan's
/// progress and completion (`Msg::PollScan`) — without this, a scan running via `Msg::RunScan`
/// would only be checked on the next keypress, defeating the point of it running off the main
/// thread in the first place.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

fn run_event_loop(state: &mut AppState) -> io::Result<()> {
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|frame| layout::render(frame, state))?;

        if event::poll(POLL_INTERVAL)? {
            if let Event::Key(key) = event::read()? {
                input::handle_key(state, key);
            }
        } else {
            #[cfg(feature = "cheat-list")]
            crate::app::update(state, Msg::Tick);
        }

        if state.is_scanning() {
            crate::app::update(state, Msg::PollScan);
        }

        if state.should_quit() {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests;
