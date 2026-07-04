use crate::tui::state::State;
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

/// Titles of the main tabs.
pub const MAIN_TABS: &[&str] = Tab::get_headers();

/// Header for the strings table.
const STRINGS_HEADERS: &[&str] = &["Location", "String"];

/// Maximum number of elements to show in table/list.
const LIST_LIMIT: usize = 100;

/// Application tab.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default, clap::ValueEnum)]
pub enum Tab {
    /// General information.
    #[default]
    General = 0,
}

impl Tab {
    /// Returns the available tabs.
    const fn get_headers() -> &'static [&'static str] {
        &["General"]
    }
}

impl From<usize> for Tab {
    fn from(v: usize) -> Self {
        match v {
            0 => Self::General,
            _ => Self::default(),
        }
    }
}

/// Renders the user interface widgets.
pub fn render(state: &mut State, frame: &mut Frame) {
    if !state.logo.is_rendered {
        let area = frame.area();
        let (logo_width, logo_height) = state.logo.get_size();
        if logo_width < area.width && logo_height < area.height {
            frame.render_widget(
                &state.logo,
                Rect::new(
                    area.width / 2 - logo_width / 2,
                    area.height / 2 - logo_height / 2,
                    logo_width,
                    logo_height,
                ),
            );
            state.logo.is_rendered = state.logo.init_time.elapsed().as_millis() > 500;
            return;
        }
    }
    let chunks = Layout::new(
        Direction::Vertical,
        [Constraint::Length(3), Constraint::Min(0)],
    )
    .direction(Direction::Vertical)
    .margin(1)
    .split(frame.area());

    {
        frame.render_widget(
            Block::bordered()
                .title(vec![
                    "|".fg(Color::Rgb(100, 100, 100)),
                    env!("CARGO_PKG_NAME").bold(),
                    "-".fg(Color::Rgb(100, 100, 100)),
                    env!("CARGO_PKG_VERSION").into(),
                    "|".fg(Color::Rgb(100, 100, 100)),
                ])
                .title_alignment(Alignment::Center),
            chunks[0],
        );
        let chunks = Layout::new(
            Direction::Horizontal,
            [Constraint::Percentage(50), Constraint::Percentage(50)],
        )
        .margin(1)
        .split(chunks[0]);
        let tabs = Tabs::new(MAIN_TABS.iter().map(|v| Line::from(*v)))
            .select(state.tab as usize)
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(state.accent_color),
            );
        frame.render_widget(tabs, chunks[0]);
        let mut files = Vec::new();
        // TODO
        // Add options here to select ?
        files.push(" ".into());
        frame.render_widget(
            Paragraph::new(Line::from(files)).alignment(Alignment::Right),
            chunks[1],
        )
    }
    match state.tab {
        Tab::General => {
            render_general_info(state, frame, chunks[1]);
        }
    }
    render_key_bindings(state, frame, chunks[1]);
}

/// Renders the key bindings.
pub fn render_key_bindings(state: &mut State, frame: &mut Frame, rect: Rect) {
    let chunks = Layout::vertical([Constraint::Percentage(100), Constraint::Min(1)]).split(rect);
    let key_bindings = state.get_key_bindings();
    let line = Line::from(
        key_bindings
            .iter()
            .enumerate()
            .flat_map(|(i, (keys, desc))| {
                vec![
                    "[".fg(Color::Rgb(100, 100, 100)),
                    keys.yellow(),
                    "→ ".fg(Color::Rgb(100, 100, 100)),
                    Span::from(*desc),
                    "]".fg(Color::Rgb(100, 100, 100)),
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

/// Renders the general info tab.
pub fn render_general_info(state: &mut State, frame: &mut Frame, rect: Rect) {
    let selected_index = state.list.state.selected().unwrap_or_default();
    let items_len = state.list.items.len();

    frame.render_widget(Block::bordered(), rect);
    let area = Layout::new(
        Direction::Vertical,
        [
            Constraint::Percentage(5),
            Constraint::Length(7),
            Constraint::Percentage(100),
        ],
    )
    .margin(1)
    .split(rect);

    let banner = BigTextBuilder::default()
        .pixel_size(PixelSize::Sextant)
        .lines([format!("{}.", env!("CARGO_PKG_NAME")).into()])
        .build();
    let banner_width = 34;
    let banner_area = Layout::new(
        Direction::Horizontal,
        [
            Constraint::Length((area[1].width.checked_sub(banner_width)).unwrap_or_default() / 2),
            Constraint::Min(banner_width),
            Constraint::Length((area[1].width.checked_sub(banner_width)).unwrap_or_default() / 2),
        ],
    )
    .split(area[1]);
    frame.render_widget(banner, banner_area[1]);
    frame.render_widget(
        Paragraph::new(Text::from(vec![
            Line::default(),
            Line::default(),
            Line::default(),
            Line::from(vec![
                "Trim Nix config ".fg(state.accent_color),
                "like a pruner.".yellow().italic(),
            ]),
            Line::from(
                ratatui::symbols::line::HORIZONTAL
                    .repeat(33)
                    .fg(Color::Rgb(100, 100, 100)),
            ),
            Line::from(env!("CARGO_PKG_REPOSITORY").italic()),
            Line::from(vec![
                "[".fg(Color::Rgb(100, 100, 100)),
                "with ".into(),
                "♥".cyan(),
                " by ".into(),
                "@wallago".cyan(),
                "]".fg(Color::Rgb(100, 100, 100)),
            ]),
        ]))
        .centered(),
        banner_area[1],
    );

    let lines = if cfg!(target_os = "windows") {
        vec![
            Line::from("This feature is not implemented!"),
            Line::from("See <https://github.com/orhun/binsider/issues/35>"),
        ]
    } else {
        vec![Line::from(vec![
            "Available Modules".cyan(),
            Span::raw(": ").fg(Color::Rgb(100, 100, 100)),
            // TODO
            // Get the number of options
            (state
                .config
                .available_modules
                .as_deref()
                .unwrap_or_default()
                .len())
            .to_string()
            .fg(state.accent_color),
        ])]
    };

    let info_width = lines.iter().map(|v| v.width()).max().unwrap_or_default() as u16 + 2;
    let rect = area[2].inner(Margin {
        horizontal: 0,
        vertical: 1,
    });
    let area = Layout::new(
        Direction::Vertical,
        if state.list.items.is_empty() {
            vec![Constraint::Max(lines.len() as u16 + 2)]
        } else if (lines.len() as u16).saturating_sub(2) < rect.height / 2 {
            vec![
                Constraint::Min(lines.len() as u16 + 2),
                Constraint::Percentage(100),
            ]
        } else {
            vec![Constraint::Percentage(50), Constraint::Percentage(50)]
        },
    )
    .split(rect);

    let info_area = Layout::new(
        Direction::Horizontal,
        [
            Constraint::Length((area[0].width.checked_sub(info_width)).unwrap_or_default() / 2),
            Constraint::Min(info_width),
            Constraint::Length((area[0].width.checked_sub(info_width)).unwrap_or_default() / 2),
        ],
    )
    .split(area[0])[1];

    let max_height = lines.len().saturating_sub(info_area.height as usize);
    if max_height + 2 < state.available_option_scroll_index {
        state.available_option_scroll_index = max_height + 2;
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::bordered()
                    .title(Line::from(vec![
                        "|".fg(Color::Rgb(100, 100, 100)),
                        "Nixos Config".cyan(),
                        "|".fg(Color::Rgb(100, 100, 100)),
                    ]))
                    .title_alignment(Alignment::Center)
                    .border_style(Style::default().fg(Color::Rgb(100, 100, 100))),
            )
            .scroll((state.available_option_scroll_index as u16, 0))
            .wrap(Wrap { trim: true }),
        info_area,
    );
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        info_area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut ScrollbarState::new(max_height).position(state.available_option_scroll_index),
    );

    if state.list.items.is_empty() {
        return;
    }

    let max_row_width = state
        .list
        .items
        .iter()
        .map(|v| v.join(" ").len())
        .max()
        .unwrap_or_default() as u16
        + 5;

    let table_area = Layout::new(
        Direction::Horizontal,
        [
            Constraint::Length((area[1].width.checked_sub(max_row_width)).unwrap_or_default() / 2),
            Constraint::Min(max_row_width),
            Constraint::Length((area[1].width.checked_sub(max_row_width)).unwrap_or_default() / 2),
        ],
    )
    .split(area[1]);

    let table_area = Layout::new(
        Direction::Vertical,
        [
            Constraint::Min(state.list.items.len() as u16 + 3),
            Constraint::Percentage(100),
        ],
    )
    .split(table_area[1])[0];
    let items = state
        .list
        .items
        .clone()
        .into_iter()
        .map(Row::new)
        .collect::<Vec<Row>>();

    frame.render_stateful_widget(
        Table::new(
            items.clone(),
            &[
                Constraint::Min(
                    state
                        .list
                        .items
                        .iter()
                        .map(|v| v[0].len())
                        .max()
                        .unwrap_or_default() as u16
                        + 1,
                ),
                Constraint::Percentage(100),
            ],
        )
        .header(Row::new(vec!["Library".bold(), "Path".bold()]))
        .block(
            Block::bordered()
                .title(vec![
                    "|".fg(Color::Rgb(100, 100, 100)),
                    "Dependencies".fg(state.accent_color).bold(),
                    "|".fg(Color::Rgb(100, 100, 100)),
                ])
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(Color::Rgb(100, 100, 100)))
                .title_bottom(
                    if items_len != 0 {
                        Line::from(vec![
                            "|".fg(Color::Rgb(100, 100, 100)),
                            format!("{}/{}", selected_index.saturating_add(1), items_len)
                                .fg(state.accent_color)
                                .bold(),
                            "|".fg(Color::Rgb(100, 100, 100)),
                        ])
                    } else {
                        Line::default()
                    }
                    .right_aligned(),
                ),
        )
        .row_highlight_style(Style::default().fg(Color::Green)),
        table_area,
        &mut state.list.state,
    );
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        table_area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut ScrollbarState::new(items.len())
            .position(state.list.state.selected().unwrap_or_default()),
    );
}

/// Returns the input line.
fn get_input_line<'a>(state: &'a State) -> Line<'a> {
    if !state.input.value().is_empty() || state.input_mode {
        Line::from(vec![
            "|".fg(Color::Rgb(100, 100, 100)),
            "search: ".yellow(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

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
