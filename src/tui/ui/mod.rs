use std::rc::Rc;

use crate::ExportStage;
use crate::tui::state::{Panel, State};
use crate::tui::ui::body::render_body;
use crate::tui::ui::header::render_header;
use crate::tui::ui::splash_screen::render_splash_screen;
use crate::tui::ui::tab::Tab;
use ratatui::widgets::Clear;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, Tabs, Wrap,
    },
};
use tui_big_text::{BigTextBuilder, PixelSize};
use tui_input::Input;

mod body;
mod header;
mod module_panel;
mod splash_screen;
mod tab;

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
    render_splash_screen(state, frame);
    let chunks = create_layout(frame);
    render_header(chunks[0], frame, state);
    render_body(chunks[1], frame, state);
    render_export_popup(state, frame);
}

/// Builds the tree guide prefix (`│  ├─ └─`) for a module row.
fn tree_prefix(rows: &[(usize, usize)], row: usize) -> String {
    let Some(&(depth, _)) = rows.get(row) else {
        return String::new();
    };
    if depth == 0 {
        return String::new();
    }
    let mut prefix = String::new();
    // Continuation bar for each ancestor level that has more siblings below.
    for level in 1..depth {
        let continues = rows[row + 1..]
            .iter()
            .take_while(|(next_depth, _)| *next_depth >= level)
            .any(|(next_depth, _)| *next_depth == level);
        prefix.push_str(if continues { "│  " } else { "   " });
    }
    let last = !rows[row + 1..]
        .iter()
        .take_while(|(next_depth, _)| *next_depth >= depth)
        .any(|(next_depth, _)| *next_depth == depth);
    prefix.push_str(if last { "└─ " } else { "├─ " });
    prefix
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

/// Returns the line with the search result highlighted.
fn highlight_search_result<'a>(line: Line<'a>, input: &'a Input) -> Vec<Span<'a>> {
    let line_str = line.to_string();
    if line_str.contains(input.value()) && !input.value().is_empty() {
        let splits = line_str.split(input.value());
        let chunks = splits.into_iter().map(|c| Span::from(c.to_owned()));
        let pattern = Span::styled(
            input.value(),
            Style::new().bg(Color::Yellow).fg(Color::Black),
        );
        itertools::intersperse(chunks, pattern).collect::<Vec<Span>>()
    } else {
        line.spans.clone()
    }
}

/// Renders the export popup: input prompts while the flow is running,
/// then the report or error once it finished.
fn render_export_popup(state: &State, frame: &mut Frame) {
    let (title, lines) = match &state.export_stage {
        ExportStage::Idle => return,
        ExportStage::Hostname => ("Export 1/2", export_prompt_lines("hostname: ", state)),
        ExportStage::OutputDir { .. } => ("Export 2/2", export_prompt_lines("output dir: ", state)),
        ExportStage::Done(result) => ("Export", export_report_lines(result, state)),
    };
    let area = frame.area();
    let width = (area.width / 2)
        .max(40)
        .min(area.width.saturating_sub(4))
        .max(1);
    let height = (lines.len() as u16 + 2)
        .min(area.height.saturating_sub(2))
        .max(1);
    let popup = Rect::new(
        area.width.saturating_sub(width) / 2,
        area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }).block(
            Block::bordered()
                .title(vec![
                    "|".fg(Color::Rgb(100, 100, 100)),
                    title.fg(state.accent_color).bold(),
                    "|".fg(Color::Rgb(100, 100, 100)),
                ])
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(state.accent_color)),
        ),
        popup,
    );
}

/// Builds the lines of an export input prompt.
fn export_prompt_lines<'a>(label: &'a str, state: &'a State) -> Vec<Line<'a>> {
    vec![
        Line::from(vec![
            label.yellow(),
            state.input.value().fg(state.accent_color),
            "█".fg(Color::Rgb(100, 100, 100)),
        ]),
        Line::default(),
        Line::from(vec![
            "[⏎ confirm]".fg(Color::Rgb(100, 100, 100)),
            " ".into(),
            "[esc cancel]".fg(Color::Rgb(100, 100, 100)),
        ]),
    ]
}

/// Builds the lines of the export result.
fn export_report_lines<'a>(
    result: &'a std::result::Result<crate::trim::ExportReport, String>,
    _state: &State,
) -> Vec<Line<'a>> {
    let mut lines = match result {
        Ok(report) => {
            let mut lines = vec![
                Line::from(vec![
                    "Exported to ".into(),
                    report.output_dir.display().to_string().green(),
                ]),
                Line::from(format!("{} files written", report.files_written.len())),
            ];
            if !report.warnings.is_empty() {
                lines.push(Line::from(
                    format!("{} warning(s):", report.warnings.len()).yellow(),
                ));
                lines.extend(
                    report
                        .warnings
                        .iter()
                        .take(8)
                        .map(|warning| Line::from(format!("- {warning}"))),
                );
                if report.warnings.len() > 8 {
                    lines.push(Line::from(format!(
                        "… and {} more",
                        report.warnings.len() - 8
                    )));
                }
            }
            lines
        }
        Err(error) => vec![
            Line::from("Export failed".red()),
            Line::from(error.as_str()),
        ],
    };
    lines.push(Line::from(
        "press any key to close".fg(Color::Rgb(100, 100, 100)),
    ));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_tree_prefix() {
        // desktop > (compositor > niri), desktopShell; server standalone.
        let rows = vec![(0, 0), (1, 1), (2, 2), (1, 3), (0, 4)];
        assert_eq!(tree_prefix(&rows, 0), "");
        assert_eq!(tree_prefix(&rows, 1), "├─ ");
        assert_eq!(tree_prefix(&rows, 2), "│  └─ ");
        assert_eq!(tree_prefix(&rows, 3), "└─ ");
        assert_eq!(tree_prefix(&rows, 4), "");
    }

    #[test]
    fn test_highlight_search_string() {
        let line: Line = "onetwothree".into();
        let query = Input::new("two".into());
        let highlighted = highlight_search_result(line, &query);
        assert_eq!(
            vec![
                Span::raw("one"),
                Span::styled("two", Style::new().bg(Color::Yellow).fg(Color::Black)),
                Span::raw("three")
            ],
            highlighted
        );
    }

    // This test is not passing.
    //
    // See this Discord message for more info:
    // <https://discord.com/channels/1070692720437383208/1072907135664529508/1275922734291095734>
    #[test]
    #[ignore]
    fn test_highlight_search_line() {
        let line: Line = vec![
            Span::raw("one"),
            Span::styled("two", Style::new().bg(Color::Blue).fg(Color::Black)),
            Span::raw("three"),
        ]
        .into();
        let query = Input::new("one".into());
        let highlighted = highlight_search_result(line, &query);
        assert_eq!(
            vec![
                Span::styled("one", Style::new().bg(Color::Yellow).fg(Color::Black)),
                Span::styled("two", Style::new().bg(Color::Blue).fg(Color::Black)),
                Span::raw("three")
            ],
            highlighted
        );
    }
}
