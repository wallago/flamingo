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
