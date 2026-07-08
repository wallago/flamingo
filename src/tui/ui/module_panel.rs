use std::rc::Rc;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table},
};

use crate::tui::{
    state::State,
    ui::{highlight::highlight_nix, tab::Tab},
};

/// Focusable module panels.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ModulePanel {
    /// Module list panel.
    #[default]
    Modules,
    /// Module content panel.
    Content,
}

impl ModulePanel {
    pub fn create_layout(chunk: Rect) -> Rc<[Rect]> {
        Layout::horizontal([
            Constraint::Percentage(25), // chunks[0]: module tree
            Constraint::Percentage(75), // chunks[1]: module content
        ])
        .horizontal_margin(4)
        .vertical_margin(1)
        .spacing(4)
        .split(chunk.inner(Margin {
            horizontal: 1,
            vertical: 1,
        }))
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

    fn panel_border_style(current_panel: ModulePanel, panel: ModulePanel) -> Style {
        Style::default().fg(if current_panel == panel {
            Color::Green
        } else {
            Color::Gray
        })
    }

    pub fn render_module_tree(state: &mut State, frame: &mut Frame, rect: Rect) {
        let selected_index = state.list.state.selected().unwrap_or_default();
        let items_len = state.list.items.len();

        let modules = state
            .list
            .items
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                let added = state
                    .module_rows
                    .get(row_index)
                    .and_then(|(_, index)| state.focused_modules()?.get(*index))
                    .is_some_and(|module| module.added);
                Row::new(vec![Line::from(vec![
                    Span::raw(" "),
                    Self::tree_prefix(&state.module_rows, row_index).fg(Color::Gray),
                    if added {
                        "󰱒 ".green()
                    } else {
                        "󰄱 ".fg(Color::Gray)
                    },
                    Span::raw(row.first().cloned().unwrap_or_default()),
                ])])
            })
            .collect::<Vec<Row>>();

        let Some(module_tab) = state.focused_module_tab() else {
            return;
        };

        frame.render_stateful_widget(
            Table::new(modules.clone(), &[Constraint::Percentage(100)])
                .header(Row::new(vec!["Module".bold()]))
                .block(
                    Block::bordered()
                        .title(vec![
                            "|".fg(Color::Gray),
                            "Modules".fg(state.accent_color).bold(),
                            "|".fg(Color::Gray),
                        ])
                        .title_alignment(Alignment::Center)
                        .border_style(Self::panel_border_style(
                            module_tab.panel,
                            ModulePanel::Modules,
                        ))
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
            rect,
            &mut state.list.state,
        );
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            rect.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut ScrollbarState::new(modules.len())
                .position(state.list.state.selected().unwrap_or_default()),
        );
    }

    pub fn render_module_content(state: &mut State, frame: &mut Frame, rect: Rect) {
        let Some(modules) = state.focused_modules() else {
            return;
        };

        let (module_name, module_content) = state
            .list
            .state
            .selected()
            .and_then(|row| state.module_rows.get(row))
            .and_then(|(_, index)| modules.get(*index))
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

        let accent_color = state.accent_color;
        let Some(module_tab) = state.focused_module_tab_mut() else {
            return;
        };

        let content_height = module_content.lines().count();
        let viewport_height = rect.height.saturating_sub(2) as usize;
        let max_scroll = content_height.saturating_sub(viewport_height);
        if module_tab.scroll_index > max_scroll {
            module_tab.scroll_index = max_scroll;
        }

        frame.render_widget(
            Paragraph::new(highlight_nix(&module_content))
                .block(
                    Block::bordered()
                        .title(vec![
                            "|".fg(Color::Gray),
                            module_name.fg(accent_color).bold(),
                            "|".fg(Color::Gray),
                        ])
                        .title_alignment(Alignment::Center)
                        .border_style(Self::panel_border_style(
                            module_tab.panel,
                            ModulePanel::Content,
                        )),
                )
                .scroll((module_tab.scroll_index as u16, 0)),
            rect,
        );
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            rect.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut ScrollbarState::new(max_scroll).position(module_tab.scroll_index),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_tree_prefix() {
        // desktop > (compositor > niri), desktopShell; server standalone.
        let rows = vec![(0, 0), (1, 1), (2, 2), (1, 3), (0, 4)];
        assert_eq!(ModulePanel::tree_prefix(&rows, 0), "");
        assert_eq!(ModulePanel::tree_prefix(&rows, 1), "├─ ");
        assert_eq!(ModulePanel::tree_prefix(&rows, 2), "│  └─ ");
        assert_eq!(ModulePanel::tree_prefix(&rows, 3), "└─ ");
        assert_eq!(ModulePanel::tree_prefix(&rows, 4), "");
    }
}
