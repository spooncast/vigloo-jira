use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame,
};

use crate::app::{App, Mode, Panel};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header (mode tabs)
            Constraint::Min(0),   // body
            Constraint::Length(1), // footer
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);

    if app.loading {
        let loading = Paragraph::new("  Loading data from Jira...")
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(loading, chunks[1]);
    } else if let Some(ref err) = app.error {
        let error = Paragraph::new(format!("  Error: {}", err))
            .style(Style::default().fg(Color::Red));
        frame.render_widget(error, chunks[1]);
    } else {
        match app.mode {
            Mode::Sprint => render_sprint_body(frame, app, chunks[1]),
            Mode::Scrum => render_scrum_body(frame, app, chunks[1]),
        }
    }

    render_footer(frame, app, chunks[2]);
}

// -- Header --

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let titles = vec![
        Line::from(" 1: Sprint "),
        Line::from(" 2: Scrum "),
    ];

    let selected = match app.mode {
        Mode::Sprint => 0,
        Mode::Scrum => 1,
    };

    let info = match app.mode {
        Mode::Sprint => match &app.sprint {
            Some(sprint) => format!("  {} | Items: {}", sprint.name, app.work_items.len()),
            None => String::new(),
        },
        Mode::Scrum => "  Daily Scrum".to_string(),
    };

    let tabs = Tabs::new(titles)
        .select(selected)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::raw(" "))
        .block(
            Block::default()
                .title(Span::styled(
                    info,
                    Style::default().fg(Color::Cyan),
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        );

    frame.render_widget(tabs, area);
}

// -- Sprint mode --

fn render_sprint_body(frame: &mut Frame, app: &App, area: Rect) {
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_work_items(frame, app, panels[0]);
    render_subtasks(frame, app, panels[1]);
}

fn render_work_items(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.active_panel == Panel::Left {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app
        .work_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let marker = if i == app.selected_work_item { "▸ " } else { "  " };
            let status_color = status_color(&item.status);
            let line = Line::from(vec![
                Span::raw(marker),
                Span::styled(
                    &item.key,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(truncate(&item.summary, 30), Style::default().fg(Color::Gray)),
                Span::raw(" "),
                Span::styled(&item.status, Style::default().fg(status_color)),
            ]);

            let style = if i == app.selected_work_item && app.active_panel == Panel::Left {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Work Items ")
            .borders(Borders::ALL)
            .border_style(border_style),
    );
    frame.render_widget(list, area);
}

fn render_subtasks(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.active_panel == Panel::Right {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let current_key = app
        .work_items
        .get(app.selected_work_item)
        .map(|w| w.key.as_str())
        .unwrap_or("");

    let title = format!(" Subtasks ({}) ", current_key);

    let subtasks = app.current_subtasks();

    if subtasks.is_empty() && !app.work_items.is_empty() {
        let empty = Paragraph::new("  No subtasks")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            );
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = subtasks
        .iter()
        .enumerate()
        .map(|(i, sub)| {
            let status_color = status_color(&sub.status);
            let line = Line::from(vec![
                Span::styled(
                    format!("[{}]", sub.status),
                    Style::default().fg(status_color),
                ),
                Span::raw(" "),
                Span::styled(
                    &sub.key,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(truncate(&sub.summary, 35), Style::default().fg(Color::Gray)),
                Span::raw(" "),
                Span::styled(&sub.assignee, Style::default().fg(Color::Blue)),
            ]);

            let style = if i == app.selected_subtask && app.active_panel == Panel::Right {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style),
    );
    frame.render_widget(list, area);
}

// -- Scrum mode (D layout: date tabs on top + full-width table) --

fn render_scrum_body(frame: &mut Frame, app: &App, area: Rect) {
    let has_today_comment = app.today_scrum().and_then(|d| d.my_comment.as_ref()).is_some();
    let has_tomorrow_key = app.tomorrow_scrum().map(|d| !d.key.is_empty()).unwrap_or(false);
    let has_action = has_today_comment && has_tomorrow_key;
    let show_bar = has_action || (has_today_comment && !has_tomorrow_key);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),                          // date tabs
            Constraint::Min(0),                            // comment table
            Constraint::Length(if show_bar { 3 } else { 0 }), // action bar
        ])
        .split(area);

    render_scrum_date_tabs(frame, app, chunks[0]);
    render_scrum_comment(frame, app, chunks[1]);

    if has_action {
        render_scrum_action_bar(frame, app, chunks[2]);
    } else if has_today_comment && !has_tomorrow_key {
        let tomorrow_date = app.tomorrow_scrum().map(|d| d.date.as_str()).unwrap_or("");
        let msg = format!("  내일({}) 스크럼 이슈가 아직 생성되지 않아 자동 작성을 할 수 없습니다.", &tomorrow_date[5..]);
        let bar = Paragraph::new(msg)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)));
        frame.render_widget(bar, chunks[2]);
    }
}

fn render_scrum_action_bar(frame: &mut Frame, app: &App, area: Rect) {
    let (msg, style) = if app.confirm_write {
        (
            " 오늘의 '오늘 할 것'을 내일 스크럼에 작성합니다. 확인: Enter / 취소: Esc ",
            Style::default().fg(Color::White).bg(Color::Red).add_modifier(Modifier::BOLD),
        )
    } else {
        let tomorrow = app.tomorrow_scrum().map(|d| d.date.as_str()).unwrap_or("");
        (
            "",
            Style::default(),
        );
        let line = Line::from(vec![
            Span::raw("  "),
            Span::styled(
                " w ",
                Style::default().fg(Color::White).bg(Color::Green).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!("오늘의 '오늘 할 것' → 내일({}) 스크럼에 작성", &tomorrow[5..]),
                Style::default().fg(Color::White),
            ),
        ]);
        let bar = Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        );
        frame.render_widget(bar, area);
        return;
    };

    let bar = Paragraph::new(msg)
        .style(style)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Red)));
    frame.render_widget(bar, area);
}

fn render_scrum_date_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = app
        .scrum_days
        .iter()
        .map(|day| {
            let check = if day.my_comment.is_some() { " ✓" } else { "" };
            Line::from(format!(" {} ({}){} ", day.label, &day.date[5..], check))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .select(app.selected_scrum_day)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::raw(" │ "))
        .block(
            Block::default()
                .title(" Daily Scrum ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        );

    frame.render_widget(tabs, area);
}

fn render_scrum_comment(frame: &mut Frame, app: &App, area: Rect) {
    let day = app.scrum_days.get(app.selected_scrum_day);

    match app.current_scrum_comment() {
        Some(comment) => {
            let table_data = &comment.table;
            if table_data.headers.is_empty() {
                let block = Block::default()
                    .title(" My Comment ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray));
                let empty = Paragraph::new("  Empty comment")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(block);
                frame.render_widget(empty, area);
                return;
            }

            let col_count = table_data.headers.len();

            // Split area into columns
            let constraints: Vec<Constraint> = (0..col_count)
                .map(|_| Constraint::Ratio(1, col_count as u32))
                .collect();
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(constraints)
                .split(area);

            // Merge all row data per column
            for (col_idx, col_area) in columns.iter().enumerate() {
                let header = table_data.headers.get(col_idx).map(|s| s.as_str()).unwrap_or("");

                // Collect all cell texts for this column across rows
                let mut lines: Vec<Line> = Vec::new();
                for row in &table_data.rows {
                    let cell_text = row.get(col_idx).map(|s| s.as_str()).unwrap_or("");
                    if cell_text.is_empty() || cell_text == "\u{a0}" {
                        continue;
                    }
                    for text_line in cell_text.lines() {
                        if text_line.is_empty() || text_line == "\u{a0}" {
                            continue;
                        }
                        let col_inner = col_area.width.saturating_sub(2) as usize;
                        wrap_text_into(text_line, col_inner, &mut lines);
                    }
                }

                let paragraph = Paragraph::new(lines)
                    .style(Style::default().fg(Color::White))
                    .block(
                        Block::default()
                            .title(Span::styled(
                                format!(" {} ", header),
                                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                            ))
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::DarkGray)),
                    )
                    .scroll((app.scrum_scroll, 0));
                frame.render_widget(paragraph, *col_area);
            }
        }
        None => {
            let block = Block::default()
                .title(" My Comment ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));
            let msg = if day.map(|d| d.key.is_empty()).unwrap_or(true) {
                "  No scrum issue found for this date"
            } else {
                "  No comment found"
            };
            let empty = Paragraph::new(msg)
                .style(Style::default().fg(Color::DarkGray))
                .block(block);
            frame.render_widget(empty, area);
        }
    }
}

// -- Footer --

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![
        Span::styled(" ↑↓", Style::default().fg(Color::Cyan)),
        Span::raw(": Move  "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(": Select/Open  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(": Back  "),
    ];

    match app.mode {
        Mode::Sprint => {
            spans.push(Span::styled("Tab", Style::default().fg(Color::Cyan)));
            spans.push(Span::raw(": Panel  "));
            spans.push(Span::styled("2", Style::default().fg(Color::Yellow)));
            spans.push(Span::raw(": Scrum  "));
        }
        Mode::Scrum => {
            spans.push(Span::styled("←→", Style::default().fg(Color::Cyan)));
            spans.push(Span::raw(": Day  "));
            spans.push(Span::styled("1", Style::default().fg(Color::Yellow)));
            spans.push(Span::raw(": Sprint  "));
        }
    }

    spans.push(Span::styled("r", Style::default().fg(Color::Cyan)));
    spans.push(Span::raw(": Refresh  "));
    spans.push(Span::styled("q", Style::default().fg(Color::Cyan)));
    spans.push(Span::raw(": Quit"));

    let footer = Paragraph::new(Line::from(spans));
    frame.render_widget(footer, area);
}

// -- Helpers --

fn status_color(status: &str) -> Color {
    match status {
        "진행 중" => Color::Yellow,
        "검토 중" => Color::Blue,
        "완료" => Color::Green,
        _ => Color::Gray,
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        let truncated: String = s.chars().take(max_len - 2).collect();
        format!("{}..", truncated)
    } else {
        s.to_string()
    }
}

fn wrap_text_into<'a>(text: &'a str, width: usize, lines: &mut Vec<Line<'a>>) {
    if width == 0 {
        lines.push(Line::from(Span::styled(
            text,
            Style::default().fg(Color::White),
        )));
        return;
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return;
    }
    let mut start = 0;
    while start < chars.len() {
        let end = (start + width).min(chars.len());
        let chunk: String = chars[start..end].iter().collect();
        lines.push(Line::from(Span::styled(
            chunk,
            Style::default().fg(Color::White),
        )));
        start = end;
    }
}
