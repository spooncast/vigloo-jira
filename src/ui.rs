use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, Panel};

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
        render_body(frame, app, chunks[1]);
    }

    render_footer(frame, chunks[2]);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = match &app.sprint {
        Some(sprint) => format!(
            " {} | Work Items: {} ",
            sprint.name,
            app.work_items.len()
        ),
        None => " vigloo-jira ".to_string(),
    };

    let header = Paragraph::new(title)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, area);
}

fn render_body(frame: &mut Frame, app: &App, area: Rect) {
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_work_items(frame, app, panels[0]);
    render_subtasks(frame, app, panels[1]);
}

fn render_work_items(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.active_panel == Panel::WorkItems {
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

            let style = if i == app.selected_work_item && app.active_panel == Panel::WorkItems {
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
    let border_style = if app.active_panel == Panel::Subtasks {
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

            let style = if i == app.selected_subtask && app.active_panel == Panel::Subtasks {
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

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Cyan)),
        Span::raw(": Move  "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(": Select/Open  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(": Back  "),
        Span::styled("Tab", Style::default().fg(Color::Cyan)),
        Span::raw(": Switch Panel  "),
        Span::styled("r", Style::default().fg(Color::Cyan)),
        Span::raw(": Refresh  "),
        Span::styled("q", Style::default().fg(Color::Cyan)),
        Span::raw(": Quit"),
    ]));
    frame.render_widget(footer, area);
}

fn status_color(status: &str) -> Color {
    match status {
        "진행 중" => Color::Yellow,
        "검토 중" => Color::Blue,
        "완료" => Color::Green,
        _ => Color::Gray, // "해야 할 일" and others
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
