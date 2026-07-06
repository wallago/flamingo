use std::rc::Rc;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::{state::State, ui::get_input_line};

fn create_layout(chunk: Rect) -> Rc<[Rect]> {
    Layout::vertical([
        Constraint::Percentage(100), // chunks[0]: spacer, pushes the bar down
        Constraint::Min(1),          // chunks[1]: one-row key-bindings bar
    ])
    .split(chunk)
}

/// Renders the key bindings.
pub fn render_key_bindings(state: &mut State, frame: &mut Frame, chunk: Rect) {
    let chunks = create_layout(chunk);
    let key_bindings = state.get_key_bindings();
    let line = Line::from(
        key_bindings
            .iter()
            .enumerate()
            .flat_map(|(i, (keys, desc))| {
                vec![
                    "[".fg(Color::Gray),
                    keys.clone().yellow(),
                    "→ ".fg(Color::Gray),
                    Span::from(*desc),
                    "]".fg(Color::Gray),
                    if i != key_bindings.len() - 1 { " " } else { "" }.into(),
                ]
            })
            .collect::<Vec<Span>>(),
    );
    if line.width() as u16 > chunks[1].width.saturating_sub(25)
        && get_input_line(state).width() != 0
    {
        return;
    }
    frame.render_widget(Paragraph::new(line.alignment(Alignment::Center)), chunks[1]);
}
