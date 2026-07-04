use flamingo::app::Config;
use flamingo::tui::state::State;
use flamingo::tui::ui;
use ratatui::{Terminal, backend::TestBackend};

#[test]
fn first_render_does_not_panic() {
    // Initialilized config
    let config = Config::new("https://github.com/wallago/nix-config").unwrap();
    // Initialilized state
    let mut state = State::new(None, config).unwrap();
    // skip the splash screen
    state.logo.is_rendered = true;
    // Initialilized backend
    let backend = TestBackend::new(120, 40);
    // Initialilized terminal
    let mut terminal = Terminal::new(backend).unwrap();
    // Render first frames
    terminal
        .draw(|frame| ui::render(&mut state, frame))
        .unwrap();
}
