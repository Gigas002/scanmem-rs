use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::install_panic_hook;
use super::layout::render;
use crate::app::AppState;

#[test]
fn installing_the_panic_hook_does_not_panic() {
    install_panic_hook();
}

#[test]
fn layout_renders_without_panicking() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState::default();

    terminal.draw(|frame| render(frame, &state)).unwrap();
}
