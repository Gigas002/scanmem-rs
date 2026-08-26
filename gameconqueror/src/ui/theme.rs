//! Resolves the user's (optional) `theme.toml` — plain color strings carried unparsed through
//! `settings::Settings` — into the concrete `ratatui::style::Style`s every panel renders with.
//! The only place `ratatui::style::Color` parsing happens, so `config/`/`settings/` stay
//! toolkit-independent. Also folds in `NO_COLOR` handling: once [`Theme::resolve`] returns, every
//! `Style` on it is already `NO_COLOR`-appropriate, so panels never check it themselves.

use std::io::IsTerminal;
#[cfg(feature = "config")]
use std::str::FromStr;
use std::sync::OnceLock;

use ratatui::style::{Color, Modifier, Style};
use thiserror::Error;

#[cfg(feature = "config")]
use crate::theme::FileTheme;

/// Every `Style` a panel might render with — resolved once at startup and read thereafter via
/// [`theme`]. Field names match `theme.toml`'s keys 1:1 (kebab-case on disk).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    /// Border of whichever panel currently has focus.
    pub focused_border: Style,
    /// Status bar while nothing is attached (and no error is showing).
    pub status_bar: Style,
    /// Status bar while a process is attached (and no error is showing) — the main "did my
    /// attach actually take" visual cue, distinct from [`Self::status_bar`].
    pub status_bar_attached: Style,
    /// Status bar while showing an error status — takes priority over both of the above
    /// regardless of attach state.
    pub status_bar_error: Style,
    /// Scan Panel's progress gauge fill.
    pub scan_progress: Style,
    /// Selected row, in every table (Process Picker, Match View, Cheat View) and the Hex View's
    /// cursor cell.
    pub selection: Style,
    /// Match View's value cell for a match whose value changed on the most recent scan/refresh.
    pub match_changed: Style,
    /// Cheat View's Frozen column for a currently-frozen cheat.
    pub frozen: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            focused_border: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            status_bar: Style::default().fg(Color::Black).bg(Color::Gray),
            status_bar_attached: Style::default().fg(Color::Black).bg(Color::Green),
            status_bar_error: Style::default().fg(Color::White).bg(Color::Red),
            scan_progress: Style::default().fg(Color::Cyan),
            selection: Style::default().add_modifier(Modifier::REVERSED),
            match_changed: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            frozen: Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        }
    }
}

/// Failure parsing a `theme.toml` color string — `field` is the `theme.toml` key it came from.
#[derive(Debug, Error, PartialEq, Eq)]
#[error(
    "invalid color {value:?} for theme.{field} (expected a name like \"cyan\"/\"light red\", \
     \"#rrggbb\" hex, or a 0-255 palette index)"
)]
pub struct ThemeError {
    field: &'static str,
    value: String,
}

impl Theme {
    /// Resolves the final `Theme`: [`Theme::default`], overridden field-by-field by `file` (if
    /// given — absent fields keep the default), then stripped of every `Color` (keeping
    /// structural modifiers like `BOLD`/`REVERSED`, which work on monochrome terminals too) when
    /// [`color_enabled`] is `false`.
    #[cfg(feature = "config")]
    pub fn resolve(file: Option<&FileTheme>) -> Result<Self, ThemeError> {
        Self::resolve_with_color(file, color_enabled())
    }

    /// [`Self::resolve`] with the color-vs-`NO_COLOR` decision passed in explicitly instead of
    /// read live from [`color_enabled`] — the seam that makes both branches deterministically
    /// testable regardless of whether the test process's `stdout` happens to be a real terminal.
    #[cfg(feature = "config")]
    pub(super) fn resolve_with_color(
        file: Option<&FileTheme>,
        use_color: bool,
    ) -> Result<Self, ThemeError> {
        let mut theme = Self::default();

        if let Some(file) = file {
            if let Some(v) = &file.focused_border {
                theme.focused_border = theme.focused_border.fg(parse_color("focused-border", v)?);
            }
            if let Some(v) = &file.status_bar_fg {
                theme.status_bar = theme.status_bar.fg(parse_color("status-bar-fg", v)?);
            }
            if let Some(v) = &file.status_bar_bg {
                theme.status_bar = theme.status_bar.bg(parse_color("status-bar-bg", v)?);
            }
            if let Some(v) = &file.status_bar_attached_fg {
                theme.status_bar_attached = theme
                    .status_bar_attached
                    .fg(parse_color("status-bar-attached-fg", v)?);
            }
            if let Some(v) = &file.status_bar_attached_bg {
                theme.status_bar_attached = theme
                    .status_bar_attached
                    .bg(parse_color("status-bar-attached-bg", v)?);
            }
            if let Some(v) = &file.status_bar_error_fg {
                theme.status_bar_error = theme
                    .status_bar_error
                    .fg(parse_color("status-bar-error-fg", v)?);
            }
            if let Some(v) = &file.status_bar_error_bg {
                theme.status_bar_error = theme
                    .status_bar_error
                    .bg(parse_color("status-bar-error-bg", v)?);
            }
            if let Some(v) = &file.scan_progress {
                theme.scan_progress = theme.scan_progress.fg(parse_color("scan-progress", v)?);
            }
            if let Some(v) = &file.selection {
                // An explicit color replaces the default reverse-video convention outright,
                // rather than layering a background under it.
                theme.selection = Style::default().bg(parse_color("selection", v)?);
            }
            if let Some(v) = &file.match_changed {
                theme.match_changed = theme.match_changed.fg(parse_color("match-changed", v)?);
            }
            if let Some(v) = &file.frozen {
                theme.frozen = theme.frozen.fg(parse_color("frozen", v)?);
            }
        }

        if !use_color {
            theme = strip_colors(theme);
        }

        Ok(theme)
    }

    /// Same as the `config`-feature [`Theme::resolve`], minus reading a theme file (there is
    /// none to read without `config`) — still applies [`color_enabled`].
    #[cfg(not(feature = "config"))]
    pub fn resolve() -> Result<Self, ThemeError> {
        Ok(Self::resolve_with_color(color_enabled()))
    }

    /// See the `config`-feature [`Self::resolve_with_color`] — same testable seam.
    #[cfg(not(feature = "config"))]
    pub(super) fn resolve_with_color(use_color: bool) -> Self {
        let mut theme = Self::default();
        if !use_color {
            theme = strip_colors(theme);
        }
        theme
    }
}

#[cfg(feature = "config")]
fn parse_color(field: &'static str, value: &str) -> Result<Color, ThemeError> {
    Color::from_str(value).map_err(|_| ThemeError {
        field,
        value: value.to_owned(),
    })
}

/// Clears every `Style`'s colors, keeping only structural modifiers (`BOLD`/`REVERSED`/...) —
/// plus a modifier fallback for the handful of styles whose only differentiator is otherwise a
/// color, so they don't become visually identical to plain text once colors are gone.
fn strip_colors(theme: Theme) -> Theme {
    fn strip(style: Style) -> Style {
        Style {
            fg: None,
            bg: None,
            ..style
        }
    }
    Theme {
        focused_border: strip(theme.focused_border),
        status_bar: strip(theme.status_bar),
        status_bar_attached: strip(theme.status_bar_attached).add_modifier(Modifier::BOLD),
        status_bar_error: strip(theme.status_bar_error)
            .add_modifier(Modifier::REVERSED | Modifier::BOLD),
        scan_progress: strip(theme.scan_progress).add_modifier(Modifier::REVERSED),
        selection: strip(theme.selection).add_modifier(Modifier::REVERSED),
        match_changed: strip(theme.match_changed).add_modifier(Modifier::BOLD),
        frozen: strip(theme.frozen).add_modifier(Modifier::BOLD),
    }
}

/// Whether `ui/` should use ANSI colors, honoring `NO_COLOR` — the same convention (and the same
/// `NO_COLOR`-then-`IsTerminal` check) as `scanmem`'s CLI `commands::formatter::color_enabled`,
/// reused here rather than inventing a second color-detection policy. `stdout` is the relevant
/// stream since that's what `CrosstermBackend` renders to.
fn color_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}

static THEME: OnceLock<Theme> = OnceLock::new();

/// Stores `theme` as the process-wide resolved theme — called exactly once, in `ui::run`, before
/// the first frame renders.
pub(crate) fn init(theme: Theme) {
    let _ = THEME.set(theme);
}

/// The process-wide resolved theme, set by [`init`]. Every panel reads styling through this
/// rather than threading a `&Theme` through every render function's signature.
pub(crate) fn theme() -> &'static Theme {
    THEME.get_or_init(Theme::default)
}
