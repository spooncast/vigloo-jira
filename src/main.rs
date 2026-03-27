mod acli;
mod app;
mod config;
mod event;
mod model;
mod ui;

use acli::AcliClient;
use app::App;
use config::Config;
use event::{handle_events, AppEvent};

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;
    let client = AcliClient::new(config.jira.board_id);

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

    let result = run_app(&mut terminal, &client, &config.jira.host).await;

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
    client: &AcliClient,
    jira_host: &str,
) -> Result<()> {
    let mut app = App::new(jira_host.to_string());

    // Initial data load
    load_data(&mut app, client).await;

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        match handle_events(&mut app)? {
            AppEvent::Quit => break,
            AppEvent::Refresh => {
                app.loading = true;
                terminal.draw(|frame| ui::render(frame, &app))?;
                load_data(&mut app, client).await;
            }
            AppEvent::OpenLink(url) => {
                let _ = open::that(&url);
            }
            AppEvent::None => {}
        }
    }

    Ok(())
}

async fn load_data(app: &mut App, client: &AcliClient) {
    match client.fetch_all_data().await {
        Ok((sprint, work_items, warnings)) => {
            app.set_data(sprint, work_items);
            for w in warnings {
                app.add_warning(w);
            }
        }
        Err(e) => app.set_error(format!("{:#}", e)),
    }
}
