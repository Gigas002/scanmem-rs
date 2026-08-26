//! Ratatui shell: terminal lifecycle (raw mode, alternate screen, panic recovery) and the
//! top-level event loop. Panel-specific rendering lives in sibling files (`layout.rs`, ...).

#[cfg(feature = "cheat-list")]
mod cheat_view;
mod error_dialog;
mod help_overlay;
#[cfg(feature = "hex-view")]
mod hex_view;
mod input;
mod keymap;
mod layout;
mod match_view;
mod process_picker;
mod scan_panel;
mod theme;

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
use ratatui::style::Style;

use crate::app::{AppState, Msg};
use crate::settings::Settings;

/// The border style every panel renderer applies to its `Block`: [`theme::theme`]'s
/// `focused_border` when `focused` (the grid's currently focused tile, or the sole panel shown
/// while expanded), plain otherwise — the only visual cue distinguishing panels in the
/// always-visible grid `ui/layout.rs` renders.
pub(crate) fn panel_border_style(focused: bool) -> Style {
    if focused {
        theme::theme().focused_border
    } else {
        Style::default()
    }
}

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
/// so a panic mid-render never leaves the shell in raw mode / the alternate screen. Also logs the
/// panic through `tracing` — the default hook only prints to stderr, which is easy to lose if the
/// terminal that ran gameconqueror closed before anyone read it, but `tracing`'s sink is a file
/// that survives that.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        tracing::error!(%info, "panic");
        default_hook(info);
    }));
}

/// Runs the `ratatui` shell: process picker, status bar, quitting cleanly on `Ctrl+Q`.
pub fn run(state: &mut AppState, settings: &Settings) -> ExitCode {
    install_panic_hook();
    crate::app::update(state, Msg::RefreshProcessList);

    #[cfg(feature = "config")]
    let resolved_theme = theme::Theme::resolve(Some(&settings.theme));
    #[cfg(not(feature = "config"))]
    let resolved_theme = theme::Theme::resolve();
    match resolved_theme {
        Ok(resolved) => theme::init(resolved),
        Err(err) => {
            eprintln!("gameconqueror: {err}");
            return ExitCode::FAILURE;
        }
    }

    let guard = match TerminalGuard::enter() {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("gameconqueror: failed to initialize terminal: {err}");
            return ExitCode::FAILURE;
        }
    };

    let poll_interval = Duration::from_millis(settings.poll_interval_ms);
    let result = run_event_loop(state, poll_interval);
    drop(guard);

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("gameconqueror: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_event_loop(state: &mut AppState, poll_interval: Duration) -> io::Result<()> {
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|frame| layout::render(frame, state))?;

        if event::poll(poll_interval)? {
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
