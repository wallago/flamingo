use std::rc::Rc;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, Paragraph, Tabs},
};

use crate::tui::{
    state::State,
    ui::{MAIN_TABS, get_input_line},
};

fn create_layout(chunk: Rect) -> Rc<[Rect]> {
    Layout::new(
        Direction::Horizontal,
        [
            Constraint::Percentage(50), // chunks[0]: tabs
            Constraint::Percentage(50), // chunks[1]: infos
        ],
    )
    .margin(1)
    .split(chunk)
}

pub fn render_header(chunk: Rect, frame: &mut Frame, state: &State) {
    frame.render_widget(
        Block::bordered()
            .title(vec![
                "|".fg(Color::Gray),
                env!("CARGO_PKG_NAME").bold(),
                "-".fg(Color::Rgb(100, 100, 100)),
                env!("CARGO_PKG_VERSION").into(),
                "|".fg(Color::Rgb(100, 100, 100)),
            ])
            .title_alignment(Alignment::Center),
        chunk,
    );
    let chunks = create_layout(chunk);
    let tabs = Tabs::new(MAIN_TABS.iter().map(|v| Line::from(*v)))
        .select(state.tab as usize)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(state.accent_color),
        );
    frame.render_widget(tabs, chunks[0]);
    frame.render_widget(
        Paragraph::new(get_input_line(state)).alignment(Alignment::Right),
        chunks[1],
    );
}
