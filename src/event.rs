use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::app::App;

pub enum AppEvent {
    Quit,
    Refresh,
    OpenLink(String),
    None,
}

pub fn handle_events(app: &mut App) -> anyhow::Result<AppEvent> {
    if event::poll(std::time::Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(AppEvent::None);
            }
            match key.code {
                KeyCode::Char('q') => return Ok(AppEvent::Quit),
                KeyCode::Char('r') => return Ok(AppEvent::Refresh),
                KeyCode::Up => app.move_up(),
                KeyCode::Down => app.move_down(),
                KeyCode::Tab => app.toggle_panel(),
                KeyCode::Esc => app.go_back(),
                KeyCode::Enter => {
                    if let Some(event) = app.handle_enter() {
                        return Ok(event);
                    }
                }
                _ => {}
            }
        }
    }
    Ok(AppEvent::None)
}
