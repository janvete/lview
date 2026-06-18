use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, LogViewer, Screen};

pub fn draw(f: &mut Frame, app: &App) {
    match app.screen {
        Screen::Picker | Screen::PickerSearch => draw_picker(f, app),
        Screen::Viewer | Screen::Search => draw_viewer(f, app),
    }
}

fn draw_picker(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(8),
            Constraint::Length(1),
        ])
        .split(f.area());

    draw_log_list(f, app, chunks[0]);
    draw_preview(f, app, chunks[1]);
    draw_status(f, app, chunks[2]);

    if app.screen == Screen::PickerSearch {
        draw_search_popup(f, app);
    }
}

fn draw_viewer(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(5), Constraint::Length(1)])
        .split(f.area());

    if let Some(viewer) = &app.viewer {
        draw_log_buffer(f, viewer, chunks[0]);
    } else {
        let p = Paragraph::new("No log selected.").block(default_block(" Viewer "));
        f.render_widget(p, chunks[0]);
    }
    draw_viewer_status(f, app, chunks[1]);

    if app.screen == Screen::Search {
        draw_search_popup(f, app);
    }
}

fn draw_log_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(i, &idx)| {
            let log = &app.logs[idx];
            let style = if i == app.selected_index {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("[{}] {}", log.category(), log.display_name())).style(style)
        })
        .collect();

    let title = format!(" Logs ({}/{}) ", app.filtered_indices.len(), app.logs.len());
    let list = List::new(items).block(default_block(&title));
    let mut state = ListState::default().with_selected(Some(app.selected_index));
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_preview(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let text = match app.selected_log() {
        Some(log) => Text::from(vec![
            Line::from(vec![
                Span::styled("Source: ", Style::default().fg(Color::Yellow)),
                Span::raw(log.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Category: ", Style::default().fg(Color::Yellow)),
                Span::raw(log.category()),
            ]),
        ]),
        None => Text::from("No log selected."),
    };
    let paragraph = Paragraph::new(text)
        .block(default_block(" Preview "))
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn draw_log_buffer(f: &mut Frame, viewer: &LogViewer, area: ratatui::layout::Rect) {
    let live_style = if viewer.live {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    };
    let title = Line::from(vec![
        Span::raw(format!(" {} — ", viewer.source)),
        Span::styled(if viewer.live { "LIVE" } else { "PAUSED" }, live_style),
        Span::raw(" "),
    ]);
    let visible = viewer.visible_lines();

    // Compute the viewport offset so the current line stays in view.
    let current_visible_idx = visible
        .iter()
        .position(|(i, _)| *i == viewer.scroll)
        .unwrap_or(visible.len().saturating_sub(1));
    let scroll_offset = current_visible_idx
        .saturating_sub(area.height as usize / 2)
        .min(visible.len().saturating_sub(1));

    let lines: Vec<Line> = visible
        .iter()
        .map(|(global_idx, line)| {
            let is_current = *global_idx == viewer.scroll;
            let is_match = viewer
                .search_cursor
                .map(|c| c == *global_idx)
                .unwrap_or(false);

            let mut style = Style::default();
            if is_current {
                style = style
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD);
            }
            if is_match {
                style = style.bg(Color::Yellow).fg(Color::Black);
            }
            let dot_color = log_level_color(line);
            Line::from(vec![
                Span::styled("● ", Style::default().fg(dot_color)),
                Span::styled(line.as_str(), style),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset as u16, 0));
    f.render_widget(paragraph, area);
}

fn draw_status(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let help = "j/k: move | Enter: open | /: filter | r: reload | q: quit";
    let target = app.session.target.host();
    let text = if app.loading {
        Text::from("Loading...")
    } else {
        match &app.message {
            Some(msg) => Text::from(format!("{} | {}", target, msg)),
            None => Text::from(format!("{} | {}", target, help)),
        }
    };
    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Left);
    f.render_widget(paragraph, area);
}

fn draw_viewer_status(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let help = "j/k: scroll | g/G: top/bottom | l/Space: live | /: search | n/N: next/prev | s: save | q: back";
    let lines = match (&app.viewer, &app.message) {
        (Some(v), None) => {
            let live_style = if v.live {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            };
            Line::from(vec![
                Span::styled(if v.live { "LIVE" } else { "PAUSED" }, live_style),
                Span::styled(
                    format!(" | lines: {} | scroll: {} | {}", v.buffer.len(), v.scroll, help),
                    Style::default().fg(Color::Gray),
                ),
            ])
        }
        _ => Line::from(Span::styled(help, Style::default().fg(Color::Gray))),
    };
    let paragraph = Paragraph::new(Text::from(lines));
    f.render_widget(paragraph, area);
}

fn draw_search_popup(f: &mut Frame, app: &App) {
    let area = f.area();
    let popup_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(3),
            Constraint::Percentage(70),
        ])
        .split(area)[1];
    let popup_area = popup_area.inner(Margin {
        horizontal: area.width / 4,
        vertical: 0,
    });

    let query = app.current_filter_query();
    let title = if app.screen == Screen::PickerSearch {
        " Filter logs "
    } else {
        " Search in log "
    };
    let input = Paragraph::new(query)
        .block(default_block(title).border_style(Style::default().fg(Color::Yellow)))
        .wrap(Wrap { trim: true });
    f.render_widget(Clear, popup_area);
    f.render_widget(input, popup_area);
}

fn default_block(title: &str) -> Block<'_> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
}

/// Determine a color for the log-level indicator dot based on the line content.
fn log_level_color(line: &str) -> Color {
    let lower = line.to_lowercase();
    if lower.contains("error")
        || lower.contains("critical")
        || lower.contains("fatal")
        || lower.contains("panic")
        || lower.contains("emergency")
    {
        Color::Red
    } else if lower.contains("warn") || lower.contains("warning") {
        Color::Yellow
    } else if lower.contains("info") || lower.contains("notice") {
        Color::Cyan
    } else if lower.contains("debug") {
        Color::Gray
    } else if lower.contains("trace") {
        Color::DarkGray
    } else {
        Color::White
    }
}
