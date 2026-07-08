use std::rc::Rc;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Padding, Paragraph, Wrap},
};
use tui_big_text::{BigTextBuilder, PixelSize};

use crate::tui::state::State;

/// Width of the big-text banner and its taglines.
const BANNER_WIDTH: u16 = 34;

fn create_layout(chunk: Rect) -> Rc<[Rect]> {
    Layout::vertical([
        Constraint::Percentage(5),   // chunks[0]: top spacer
        Constraint::Length(7),       // chunks[1]: banner
        Constraint::Percentage(100), // chunks[2]: info panel
    ])
    .margin(1)
    .split(chunk)
}

/// Returns a rect of at most `width` x `height`, horizontally centered
/// and pinned to the top of `chunk`.
fn centered(chunk: Rect, width: u16, height: u16) -> Rect {
    let [area] = Layout::vertical([Constraint::Max(height)]).areas(chunk);
    let [area] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);
    area
}

fn render_lines<'a>(accent_color: Color, lines: Vec<(&'a str, &'a str)>) -> Vec<Line<'a>> {
    lines
        .iter()
        .map(|(key, value)| {
            Line::from(vec![
                key.cyan(),
                ": ".fg(Color::Gray),
                value.fg(accent_color),
            ])
        })
        .collect()
}

/// Renders the big-text logo with the tagline and repository links below it.
fn render_banner(state: &State, frame: &mut Frame, chunk: Rect) {
    let area = centered(chunk, BANNER_WIDTH, chunk.height);
    frame.render_widget(
        BigTextBuilder::default()
            .pixel_size(PixelSize::Sextant)
            .lines([format!("{}.", env!("CARGO_PKG_NAME")).into()])
            .build(),
        area,
    );
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
                "♥ ".cyan(),
                "by ".into(),
                "@wallago".cyan(),
                "]".fg(Color::Rgb(100, 100, 100)),
            ]),
        ]))
        .centered(),
        area,
    );
}

/// Renders the flake summary in a centered box.
fn render_info(state: &State, frame: &mut Frame, chunk: Rect) {
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
    let nixos_total = state.flake.nixos_modules.len();
    let home_total = state.flake.home_modules.len();
    let nixos_added = format!(
        "{} ({}%)",
        nixos_added,
        if nixos_total == 0 {
            0
        } else {
            nixos_added * 100 / nixos_total
        }
    );
    let home_added = format!(
        "{} ({}%)",
        home_added,
        if home_total == 0 {
            0
        } else {
            home_added * 100 / home_total
        }
    );
    let nixos_total = nixos_total.to_string();
    let home_total = home_total.to_string();

    let input_count = state.flake.input_count.to_string();
    let commit = state.flake.commit.as_ref().map(|commit| {
        if state.flake.dirty {
            format!("{commit} (dirty)")
        } else {
            commit.clone()
        }
    });
    let mut flake_info = vec![("Source", state.flake.source.as_str())];
    if let Some(description) = &state.flake.description {
        flake_info.push(("Description", description.as_str()));
    }
    flake_info.push(("Inputs", &input_count));
    if let Some(branch) = &state.flake.branch {
        flake_info.push(("Branch", branch.as_str()));
    }
    if let Some(commit) = &commit {
        flake_info.push(("Commit", commit.as_str()));
    }
    let sections = [
        render_lines(state.accent_color, flake_info),
        render_lines(
            state.accent_color,
            vec![
                ("Nixos Modules", &nixos_total),
                ("Added Nixos Modules", &nixos_added),
            ],
        ),
        render_lines(
            state.accent_color,
            vec![
                ("Home Manager Modules", &home_total),
                ("Added Home Manager Modules", &home_added),
            ],
        ),
    ];
    let inner_width = sections
        .iter()
        .flatten()
        .map(|v| v.width())
        .max()
        .unwrap_or_default();
    let separator = Line::from(
        ratatui::symbols::line::HORIZONTAL
            .repeat(inner_width)
            .fg(Color::Gray),
    );
    let mut lines = Vec::new();
    for (i, section) in sections.into_iter().enumerate() {
        if i > 0 {
            lines.push(separator.clone());
        }
        lines.extend(section);
    }
    // Borders (2) + horizontal padding (2); height adds the top padding.
    let info_width = inner_width as u16 + 4;
    let info_area = centered(
        chunk.inner(Margin {
            horizontal: 0,
            vertical: 1,
        }),
        info_width,
        lines.len() as u16 + 3,
    );
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::bordered()
                    .title(Line::from(vec![
                        "|".fg(Color::Gray),
                        "Nixos Config".cyan(),
                        "|".fg(Color::Gray),
                    ]))
                    .title_alignment(Alignment::Center)
                    .border_style(Style::default().fg(Color::Gray))
                    .padding(Padding::new(1, 1, 1, 0)),
            )
            .wrap(Wrap { trim: true }),
        info_area,
    );
}

/// Renders the general info tab.
pub fn render_general(state: &mut State, frame: &mut Frame, chunk: Rect) {
    frame.render_widget(Block::bordered(), chunk);
    let chunks = create_layout(chunk);
    render_banner(state, frame, chunks[1]);
    render_info(state, frame, chunks[2]);
}
