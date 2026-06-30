use std::time::Instant;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::Widget,
};

// Pixel-art flamingo (one glyph = one cell). Leading spaces are transparent,
// so column alignment matters — do NOT re-center rows.
//   l = light pink (head/neck)   p = pink (body)
//   b = black beak               k = legs
const ART: [&str; HEIGHT as usize] = [
    "      lll",
    "     lllll",
    "   bblll l",
    "     lll",
    "      ll",
    "      lll",
    "     ppppp",
    "   ppppppppp",
    "   pppppppppp",
    "    ppppppp",
    "      k k",
    "      k k",
];
const WIDTH: u16 = 20;
const HEIGHT: u16 = 12;

fn color_for(ch: char) -> Option<Color> {
    match ch {
        'l' => Some(Color::Rgb(250, 180, 200)), // light pink
        'p' => Some(Color::Rgb(244, 138, 170)), // body pink
        'b' => Some(Color::Rgb(30, 30, 30)),    // beak
        'k' => Some(Color::Rgb(224, 120, 150)), // legs
        _ => None,
    }
}

#[derive(Debug)]
pub struct Logo {
    pub init_time: Instant,
    pub is_rendered: bool,
}

impl Default for Logo {
    fn default() -> Self {
        Self {
            init_time: Instant::now(),
            is_rendered: false,
        }
    }
}

impl Widget for &Logo {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines: Vec<Line> = ART
            .iter()
            .map(|row| {
                let mut spans = Vec::with_capacity(row.chars().count());
                for ch in row.chars() {
                    match color_for(ch) {
                        Some(color) => spans.push(Span::styled(" ", Style::default().bg(color))),
                        None => spans.push(Span::raw(" ")),
                    }
                }
                Line::from(spans)
            })
            .collect();
        Text::from(lines).render(area, buf);

        let message = "Loading...";
        buf.set_string(
            WIDTH / 2 - message.len() as u16 / 2 + area.x,
            HEIGHT + area.y,
            message,
            Style::default().fg(Color::Rgb(248, 190, 117)).italic(),
        );
    }
}

impl Logo {
    /// Returns the size of the logo.
    pub fn get_size(&self) -> (u16, u16) {
        (WIDTH, HEIGHT)
    }
}
