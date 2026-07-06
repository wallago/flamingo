use std::rc::Rc;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
};
use tui_big_text::{BigTextBuilder, PixelSize};

use crate::tui::state::State;

fn create_layout(chunk: Rect) -> Rc<[Rect]> {
    Layout::new(
        Direction::Vertical,
        [
            Constraint::Percentage(5),
            Constraint::Length(7),
            Constraint::Percentage(100),
        ],
    )
    .margin(1)
    .split(chunk)
}

fn create_banner_layout(chunk: Rect, width: u16) -> Rc<[Rect]> {
    Layout::new(
        Direction::Horizontal,
        [
            Constraint::Length((chunk.width.checked_sub(width)).unwrap_or_default() / 2),
            Constraint::Min(width),
            Constraint::Length((chunk.width.checked_sub(width)).unwrap_or_default() / 2),
        ],
    )
    .split(chunk)
}

fn render_line<'a>(accent_color: Color, key: &'a str, value: &'a str) -> Line<'a> {
    Line::from(vec![
        key.cyan(),
        Span::raw(": ").fg(Color::Gray),
        value.fg(accent_color),
    ])
}

/// Renders the general info tab.
pub fn render_general(state: &mut State, frame: &mut Frame, chunk: Rect) {
    frame.render_widget(Block::bordered(), chunk);
    let chunks = create_layout(chunk);

    let banner = BigTextBuilder::default()
        .pixel_size(PixelSize::Sextant)
        .lines([format!("{}.", env!("CARGO_PKG_NAME")).into()])
        .build();
    let banner_chunks = create_banner_layout(chunks[1], 34);
    frame.render_widget(banner, banner_chunks[1]);
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
                    .fg(Color::Gray),
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
        banner_chunks[1],
    );

    let lines = {
        let nixos_added = state
            .flake
            .nixos_modules
            .iter()
            .filter(|module| module.added)
            .count();
        let home_added = state
            .flake
            .home_modules
            .iter()
            .filter(|module| module.added)
            .count();
        vec![
            render_line(state.accent_color, "Source", &state.flake.source),
            render_line(
                state.accent_color,
                "Nixos Modules",
                &(state.flake.nixos_modules.len()).to_string(),
            ),
            render_line(
                state.accent_color,
                "Home Manager Modules",
                &(state.flake.home_modules.len()).to_string(),
            ),
            render_line(
                state.accent_color,
                "Added Modules",
                &(nixos_added + home_added).to_string(),
            ),
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
