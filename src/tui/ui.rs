use crate::ExportStage;
use crate::tui::state::{Panel, State};
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
    /// Nixos config modules.
    Config = 1,
}

impl Tab {
    /// Returns the available tabs.
    const fn get_headers() -> &'static [&'static str] {
        &["General", "Config"]
    }
}

impl From<usize> for Tab {
    fn from(v: usize) -> Self {
        match v {
            0 => Self::General,
            1 => Self::Config,
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
        frame.render_widget(
            Paragraph::new(get_input_line(state)).alignment(Alignment::Right),
            chunks[1],
        );
    }
    match state.tab {
        Tab::General => {
            render_general_info(state, frame, chunks[1]);
        }
        Tab::Config => {
            render_config(state, frame, chunks[1]);
        }
    }
    render_key_bindings(state, frame, chunks[1]);
    render_export_popup(state, frame);
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
        let added = state
            .config
            .modules
            .iter()
            .filter(|module| module.added)
            .count();
        vec![
            Line::from(vec![
                "Source".cyan(),
                Span::raw(": ").fg(Color::Rgb(100, 100, 100)),
                state.config.source.clone().fg(state.accent_color),
            ]),
            Line::from(vec![
                "Available Modules".cyan(),
                Span::raw(": ").fg(Color::Rgb(100, 100, 100)),
                (state.config.modules.len())
                    .to_string()
                    .fg(state.accent_color),
            ]),
            Line::from(vec![
                "Added Modules".cyan(),
                Span::raw(": ").fg(Color::Rgb(100, 100, 100)),
                added.to_string().fg(state.accent_color),
            ]),
        ]
    };

    let info_width = lines.iter().map(|v| v.width()).max().unwrap_or_default() as u16 + 2;
    let rect = area[2].inner(Margin {
        horizontal: 0,
        vertical: 1,
    });
    let area = Layout::new(
        Direction::Vertical,
        [Constraint::Max(lines.len() as u16 + 2)],
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

    // Module list.
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
}

/// Renders the config tab: module tree and content panels.
pub fn render_config(state: &mut State, frame: &mut Frame, rect: Rect) {
    let selected_index = state.list.state.selected().unwrap_or_default();
    let items_len = state.list.items.len();

    frame.render_widget(Block::bordered(), rect);

    if state.list.items.is_empty() {
        frame.render_widget(
            Paragraph::new("no modules found").centered(),
            rect.inner(Margin {
                horizontal: 1,
                vertical: 1,
            }),
        );
        return;
    }

    let panels = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(25), Constraint::Percentage(75)],
    )
    .horizontal_margin(4)
    .vertical_margin(1)
    .spacing(4)
    .split(rect.inner(Margin {
        horizontal: 1,
        vertical: 1,
    }));
    let table_area = panels[0];

    let panel_border_style = |panel: Panel| {
        Style::default().fg(if state.focused_panel == panel {
            Color::Green
        } else {
            Color::Rgb(100, 100, 100)
        })
    };

    let items = state
        .list
        .items
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            let added = state
                .module_rows
                .get(row_index)
                .and_then(|(_, index)| state.config.modules.get(*index))
                .is_some_and(|module| module.added);
            Row::new(vec![Line::from(vec![
                Span::raw(" "),
                tree_prefix(&state.module_rows, row_index).fg(Color::Rgb(100, 100, 100)),
                if added {
                    "󰱒 ".green()
                } else {
                    "󰄱 ".fg(Color::Rgb(100, 100, 100))
                },
                Span::raw(row.first().cloned().unwrap_or_default()),
            ])])
        })
        .collect::<Vec<Row>>();

    frame.render_stateful_widget(
        Table::new(items.clone(), &[Constraint::Percentage(100)])
            .header(Row::new(vec!["Module".bold()]))
        .block(
            Block::bordered()
                .title(vec![
                    "|".fg(Color::Rgb(100, 100, 100)),
                    "Modules".fg(state.accent_color).bold(),
                    "|".fg(Color::Rgb(100, 100, 100)),
                ])
                .title_alignment(Alignment::Center)
                .border_style(panel_border_style(Panel::Modules))
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

    let (module_name, module_content) = state
        .list
        .state
        .selected()
        .and_then(|row| state.module_rows.get(row))
        .and_then(|(_, index)| state.config.modules.get(*index))
        .map(|module| {
            (
                module.name.clone(),
                module
                    .content
                    .clone()
                    .unwrap_or_else(|| "source not available".to_string()),
            )
        })
        .unwrap_or_default();

    let content_height = module_content.lines().count();
    let viewport_height = panels[1].height.saturating_sub(2) as usize;
    let max_scroll = content_height.saturating_sub(viewport_height);
    if state.selected_option_scroll_index > max_scroll {
        state.selected_option_scroll_index = max_scroll;
    }

    frame.render_widget(
        Paragraph::new(module_content)
            .block(
                Block::bordered()
                    .title(vec![
                        "|".fg(Color::Rgb(100, 100, 100)),
                        module_name.fg(state.accent_color).bold(),
                        "|".fg(Color::Rgb(100, 100, 100)),
                    ])
                    .title_alignment(Alignment::Center)
                    .border_style(panel_border_style(Panel::Content)),
            )
            .scroll((state.selected_option_scroll_index as u16, 0)),
        panels[1],
    );
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓")),
        panels[1].inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut ScrollbarState::new(max_scroll).position(state.selected_option_scroll_index),
    );
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
        ExportStage::Hostname => (
            "Export 1/2",
            export_prompt_lines("hostname: ", state),
        ),
        ExportStage::OutputDir { .. } => (
            "Export 2/2",
            export_prompt_lines("output dir: ", state),
        ),
        ExportStage::Done(result) => ("Export", export_report_lines(result, state)),
    };
    let area = frame.area();
    let width = (area.width / 2)
        .max(40)
        .min(area.width.saturating_sub(4))
        .max(1);
    let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(2)).max(1);
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
