use ratatui::{
    Frame,
    layout::{Margin, Rect},
    widgets::{Block, Paragraph},
};

use crate::tui::{state::State, ui::module_panel::ModulePanel};

/// Renders the Home Manager modules tab: module tree and content panels.
pub fn render_home_modules(state: &mut State, frame: &mut Frame, chunk: Rect) {
    frame.render_widget(Block::bordered(), chunk);

    if state.list.items.is_empty() {
        frame.render_widget(
            Paragraph::new("no modules found").centered(),
            chunk.inner(Margin {
                horizontal: 1,
                vertical: 1,
            }),
        );
        return;
    }

    let chunks = ModulePanel::create_layout(chunk);
    ModulePanel::render_module_tree(state, frame, chunks[0]);
    ModulePanel::render_module_content(state, frame, chunks[1]);
}
