mod acli;
mod app;
mod cache;
mod config;
mod event;
mod model;
mod ui;

use acli::AcliClient;
use app::{App, Mode};
use config::Config;
use event::{handle_events, AppEvent, DataPayload};

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;
    let client = Arc::new(AcliClient::new(config.jira.board_id, config.jira.project));

    // Install panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, client, &config.jira.host).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    client: Arc<AcliClient>,
    jira_host: &str,
) -> Result<()> {
    let mut app = App::new(jira_host.to_string());
    let (tx, mut rx) = mpsc::unbounded_channel::<DataPayload>();

    // Spawn initial load (non-blocking, uses cache)
    spawn_load(client.clone(), Mode::Sprint, false, tx.clone());

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        // Check for async data (non-blocking)
        if let Ok(payload) = rx.try_recv() {
            match payload {
                DataPayload::Sprint { sprint, work_items, warnings } => {
                    app.set_data(sprint, work_items);
                    for w in warnings {
                        app.add_warning(w);
                    }
                }
                DataPayload::Scrum { days, warnings } => {
                    app.set_scrum_data(days);
                    for w in warnings {
                        app.add_warning(w);
                    }
                }
                DataPayload::Error(e) => app.set_error(e),
            }
        }

        match handle_events(&mut app)? {
            AppEvent::Quit => break,
            AppEvent::Refresh => {
                app.loading = true;
                spawn_load(client.clone(), app.mode, true, tx.clone());
            }
            AppEvent::SwitchMode(mode) => {
                app.loading = true;
                spawn_load(client.clone(), mode, false, tx.clone());
            }
            AppEvent::OpenLink(url) => {
                let _ = open::that(&url);
            }
            AppEvent::None => {}
        }
    }

    Ok(())
}

fn spawn_load(
    client: Arc<AcliClient>,
    mode: Mode,
    force: bool,
    tx: mpsc::UnboundedSender<DataPayload>,
) {
    tokio::spawn(async move {
        let payload = match mode {
            Mode::Sprint => match client.fetch_all_data(force).await {
                Ok((sprint, work_items, warnings)) => {
                    DataPayload::Sprint { sprint, work_items, warnings }
                }
                Err(e) => DataPayload::Error(format!("{:#}", e)),
            },
            Mode::Scrum => match client.fetch_scrum_data(force).await {
                Ok((days, warnings)) => {
                    DataPayload::Scrum { days, warnings }
                }
                Err(e) => DataPayload::Error(format!("{:#}", e)),
            },
        };
        let _ = tx.send(payload);
    });
}
