use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::app::{App, Mode};
use crate::model::{ScrumDay, Sprint, WorkItem};

pub enum AppEvent {
    Quit,
    Refresh,
    OpenLink(String),
    SwitchMode(Mode),
    None,
}

pub enum DataPayload {
    Sprint {
        sprint: Sprint,
        work_items: Vec<WorkItem>,
        warnings: Vec<String>,
    },
    Scrum {
        days: Vec<ScrumDay>,
        warnings: Vec<String>,
    },
    Error(String),
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
                KeyCode::Char('1') => {
                    if app.switch_mode(Mode::Sprint) {
                        return Ok(AppEvent::SwitchMode(Mode::Sprint));
                    }
                }
                KeyCode::Char('2') => {
                    if app.switch_mode(Mode::Scrum) {
                        return Ok(AppEvent::SwitchMode(Mode::Scrum));
                    }
                }
                KeyCode::Up => app.move_up(),
                KeyCode::Down => app.move_down(),
                KeyCode::Left => app.move_left(),
                KeyCode::Right => app.move_right(),
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
