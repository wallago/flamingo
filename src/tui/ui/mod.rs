use std::rc::Rc;

use crate::ExportStage;
use crate::tui::state::State;
use crate::tui::ui::body::render_body;
use crate::tui::ui::header::render_header;
use crate::tui::ui::popup::render_export_popup;
use crate::tui::ui::splash_screen::render_splash_screen;
use prelude::*;
use ratatui::widgets::Clear;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
};
use tui_input::Input;

mod body;
mod header;
mod highlight;
mod module_panel;
mod popup;
mod splash_screen;
mod tab;

pub mod prelude {
    pub use super::module_panel::ModulePanel;
    pub use super::tab::{ModuleTabState, Tab};
}

/// Titles of the main tabs.
pub const MAIN_TABS: &[&str] = Tab::get_headers();

/// Maximum number of elements to show in table/list.
const LIST_LIMIT: usize = 100;

fn create_layout(frame: &mut Frame) -> Rc<[Rect]> {
    Layout::new(
        Direction::Vertical,
        [
            Constraint::Length(3), // chunks[0]: header
            Constraint::Min(0),    // chunks[1]: body
            Constraint::Length(1), // chunks[2]: footer
        ],
    )
    .split(frame.area())
}

/// Renders the user interface widgets.
pub fn render(state: &mut State, frame: &mut Frame) {
    if render_splash_screen(state, frame) {
        return;
    };
    let chunks = create_layout(frame);
    render_header(chunks[0], frame, state);
    render_body(chunks[1], frame, state);
    render_export_popup(state, frame);
}

/// Returns the input line.
fn get_input_line<'a>(state: &'a State) -> Line<'a> {
    if let Some(status) = &state.status {
        return Line::from(vec![
            "|".fg(Color::Rgb(100, 100, 100)),
            status.clone().yellow(),
            "|".fg(Color::Rgb(100, 100, 100)),
        ]);
    }
    // Export prompts render in their own popup, not in the header slot.
    if !matches!(state.export_stage, ExportStage::Idle) {
        return Line::default();
    }
    let label = "search: ";
    if !state.input.value().is_empty() || state.input_mode {
        Line::from(vec![
            "|".fg(Color::Rgb(100, 100, 100)),
            label.yellow(),
            state.input.value().fg(state.accent_color),
            if state.input_mode { " " } else { "" }.into(),
            "|".fg(Color::Rgb(100, 100, 100)),
        ])
    } else {
        Line::default()
    }
}

// /// Returns the line with the search result highlighted.
// fn highlight_search_result<'a>(line: Line<'a>, input: &'a Input) -> Vec<Span<'a>> {
//     let line_str = line.to_string();
//     if line_str.contains(input.value()) && !input.value().is_empty() {
//         let splits = line_str.split(input.value());
//         let chunks = splits.into_iter().map(|c| Span::from(c.to_owned()));
//         let pattern = Span::styled(
//             input.value(),
//             Style::new().bg(Color::Yellow).fg(Color::Black),
//         );
//         itertools::intersperse(chunks, pattern).collect::<Vec<Span>>()
//     } else {
//         line.spans.clone()
//     }
// }
//
// /// Builds the lines of an export input prompt.
// fn export_prompt_lines<'a>(label: &'a str, state: &'a State) -> Vec<Line<'a>> {
//     vec![
//         Line::from(vec![
//             label.yellow(),
//             state.input.value().fg(state.accent_color),
//             "█".fg(Color::Rgb(100, 100, 100)),
//         ]),
//         Line::default(),
//         Line::from(vec![
//             "[⏎ confirm]".fg(Color::Rgb(100, 100, 100)),
//             " ".into(),
//             "[esc cancel]".fg(Color::Rgb(100, 100, 100)),
//         ]),
//     ]
// }
//

// TODO: revive the highlight tests together with `highlight_search_result`.
