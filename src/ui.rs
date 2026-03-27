use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table},
    Frame,
};

use crate::app::{App, Mode, Panel};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(0),   // body
            Constraint::Length(1), // footer
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);

    if app.loading {
        let loading = Paragraph::new("Loading data from Jira...")
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(loading, chunks[1]);
    } else if let Some(ref err) = app.error {
        let error = Paragraph::new(format!("Error: {}", err))
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

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let sprint_tab = if app.mode == Mode::Sprint {
        Span::styled(" [1:Sprint] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("  1:Sprint  ", Style::default().fg(Color::DarkGray))
    };
    let scrum_tab = if app.mode == Mode::Scrum {
        Span::styled(" [2:Scrum] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("  2:Scrum  ", Style::default().fg(Color::DarkGray))
    };

    let info = match app.mode {
        Mode::Sprint => match &app.sprint {
            Some(sprint) => format!(" | {} | Items: {}", sprint.name, app.work_items.len()),
            None => String::new(),
        },
        Mode::Scrum => " | Daily Scrum".to_string(),
    };

    let header = Paragraph::new(Line::from(vec![
        sprint_tab,
        scrum_tab,
        Span::styled(info, Style::default().fg(Color::White)),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, area);
}

// -- Sprint mode rendering --

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
                Span::styled(&item.key, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled(
                    truncate(&item.summary, 30),
                    Style::default().fg(Color::Gray),
                ),
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
        let empty = Paragraph::new("No subtasks")
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
                Span::styled(&sub.key, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled(
                    truncate(&sub.summary, 35),
                    Style::default().fg(Color::Gray),
                ),
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

// -- Scrum mode rendering --

fn render_scrum_body(frame: &mut Frame, app: &App, area: Rect) {
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(25), Constraint::Min(0)])
        .split(area);

    render_scrum_days(frame, app, panels[0]);
    render_scrum_comment(frame, app, panels[1]);
}

fn render_scrum_days(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.active_panel == Panel::Left {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = app
        .scrum_days
        .iter()
        .enumerate()
        .map(|(i, day)| {
            let marker = if i == app.selected_scrum_day { "▸ " } else { "  " };
            let status_color = status_color(&day.status);
            let has_comment = if day.my_comment.is_some() { " ✓" } else { "" };
            let line = Line::from(vec![
                Span::raw(marker),
                Span::styled(
                    format!("{} ({})", day.label, &day.date[5..]),
                    Style::default().fg(Color::White),
                ),
                Span::styled(has_comment, Style::default().fg(Color::Green)),
                Span::raw(" "),
                Span::styled(&day.status, Style::default().fg(status_color).add_modifier(Modifier::DIM)),
            ]);

            let style = if i == app.selected_scrum_day && app.active_panel == Panel::Left {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Daily Scrum ")
            .borders(Borders::ALL)
            .border_style(border_style),
    );
    frame.render_widget(list, area);
}

fn render_scrum_comment(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.active_panel == Panel::Right {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let day = app.scrum_days.get(app.selected_scrum_day);
    let title = day
        .map(|d| format!(" My Comment ({}) ", d.date))
        .unwrap_or_else(|| " My Comment ".to_string());

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    match app.current_scrum_comment() {
        Some(comment) => {
            let table_data = &comment.table;
            if table_data.headers.is_empty() {
                let empty = Paragraph::new("Empty comment").style(Style::default().fg(Color::DarkGray)).block(block);
                frame.render_widget(empty, area);
                return;
            }

            // Calculate column widths: distribute evenly
            let col_count = table_data.headers.len();
            let constraints: Vec<Constraint> = (0..col_count)
                .map(|_| Constraint::Percentage((100 / col_count as u16).max(1)))
                .collect();

            // Header row
            let header_cells: Vec<Cell> = table_data.headers.iter().map(|h| {
                Cell::from(Line::from(Span::styled(
                    h.as_str(),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )))
            }).collect();
            let header = Row::new(header_cells)
                .style(Style::default())
                .height(1)
                .bottom_margin(1);

            // Calculate available width per column (area minus borders and spacing)
            let inner_width = area.width.saturating_sub(2); // borders
            let col_width = inner_width.saturating_sub(col_count as u16) / col_count as u16;

            // Data rows — wrap long lines to fit column width
            let rows: Vec<Row> = table_data.rows.iter().map(|row| {
                let cells: Vec<Vec<Line>> = row.iter().map(|cell_text| {
                    let mut lines = Vec::new();
                    for line in cell_text.lines() {
                        wrap_text_into(line, col_width as usize, &mut lines);
                    }
                    if lines.is_empty() {
                        lines.push(Line::from(""));
                    }
                    lines
                }).collect();
                let max_lines = cells.iter().map(|c| c.len()).max().unwrap_or(1);
                let row_cells: Vec<Cell> = cells.into_iter().map(|lines| Cell::from(lines)).collect();
                Row::new(row_cells).height(max_lines as u16)
            }).collect();

            let table = Table::new(rows, &constraints)
                .header(header)
                .block(block)
                .column_spacing(1);

            frame.render_widget(table, area);
        }
        None => {
            let msg = if day.map(|d| d.key.is_empty()).unwrap_or(true) {
                "No scrum issue found for this date"
            } else {
                "No comment found"
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
        Span::styled("↑↓", Style::default().fg(Color::Cyan)),
        Span::raw(": Move  "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(": Select/Open  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(": Back  "),
        Span::styled("Tab", Style::default().fg(Color::Cyan)),
        Span::raw(": Panel  "),
    ];

    if app.mode == Mode::Sprint {
        spans.push(Span::styled("2", Style::default().fg(Color::Yellow)));
        spans.push(Span::raw(": Scrum  "));
    } else {
        spans.push(Span::styled("1", Style::default().fg(Color::Yellow)));
        spans.push(Span::raw(": Sprint  "));
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
        lines.push(Line::from(Span::styled(text, Style::default().fg(Color::White))));
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
        lines.push(Line::from(Span::styled(chunk, Style::default().fg(Color::White))));
        start = end;
    }
}
