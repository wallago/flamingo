use crate::tui::state::{ExportStage, State};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Clear, Paragraph, WidgetRef, Wrap},
};

/// Renders the export popup: input prompts while the flow is running,
/// then the report or error once it finished.
pub fn render_export_popup(state: &State, frame: &mut Frame) {
    let (title, lines) = match &state.export_stage {
        ExportStage::Idle => return,
        ExportStage::Hostname => ("Export 1/2", export_prompt_lines("hostname: ", state)),
        ExportStage::OutputDir { .. } => return render_explorer_popup(state, frame),
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
                    "|".fg(Color::Gray),
                    title.fg(state.accent_color).bold(),
                    "|".fg(Color::Gray),
                ])
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(state.accent_color)),
        ),
        popup,
    );
}

/// Renders the output-dir stage: a directory picker in a centered popup.
/// Confirming exports into the directory currently open in the picker.
fn render_explorer_popup(state: &State, frame: &mut Frame) {
    let Some(explorer) = &state.explorer else {
        return;
    };
    let area = frame.area();
    let width = (area.width / 2)
        .max(40)
        .min(area.width.saturating_sub(4))
        .max(1);
    let height = (area.height * 2 / 3)
        .max(10)
        .min(area.height.saturating_sub(2))
        .max(1);
    let popup = Rect::new(
        area.width.saturating_sub(width) / 2,
        area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    let block = Block::bordered()
        .title(vec![
            "|".fg(Color::Gray),
            "Export 2/2".fg(state.accent_color).bold(),
            "|".fg(Color::Gray),
        ])
        .title_alignment(Alignment::Center)
        .title_bottom(
            Line::from(vec![
                "[←/→ navigate]".fg(Color::Gray),
                " ".into(),
                "[⏎ export to selected dir]".fg(Color::Gray),
                " ".into(),
                "[esc cancel]".fg(Color::Gray),
            ])
            .centered(),
        )
        .border_style(Style::default().fg(state.accent_color));
    let inner = block.inner(popup);
    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);
    explorer.widget().render_ref(inner, frame.buffer_mut());
}

/// Builds the lines of an export input prompt.
fn export_prompt_lines<'a>(label: &'a str, state: &'a State) -> Vec<Line<'a>> {
    vec![
        Line::from(vec![
            label.yellow(),
            state.input.value().fg(state.accent_color),
            "█".fg(Color::Gray),
        ]),
        Line::default(),
        Line::from(vec![
            "[⏎ confirm]".fg(Color::Gray),
            " ".into(),
            "[esc cancel]".fg(Color::Gray),
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
    lines.push(Line::from("press any key to close".fg(Color::Gray)));
    lines
}
