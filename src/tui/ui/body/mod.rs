use ratatui::{Frame, layout::Rect};

use crate::tui::{
    state::State,
    ui::{
        body::{
            general::render_general, home_modules::render_home_modules,
            key_bindings::render_key_bindings, nixos_modules::render_nixos_modules,
        },
        tab::Tab,
    },
};

mod general;
mod home_modules;
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
