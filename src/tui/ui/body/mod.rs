use ratatui::{Frame, layout::Rect};

use crate::tui::{
    state::State,
    ui::{body::key_bindings::render_key_bindings, tab::Tab},
};

mod general;
mod key_bindings;
mod nixos_modules;

pub fn render_body(chunk: Rect, frame: &mut Frame, state: &mut State) {
    match state.tab {
        Tab::General => {
            render_general(state, frame, chunk);
        }
        Tab::NixosModules => {
            render_nixos_modules(state, frame, chunk);
        }
        Tab::HomeManagerModules => {
            render_home_modules(state, frame, chunk);
        }
    }
    render_key_bindings(state, frame, chunk);
}
