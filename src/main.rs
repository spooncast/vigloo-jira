mod acli;
mod app;
mod cache;
mod cli;
mod config;
mod event;
mod model;
mod ui;

use acli::AcliClient;
use app::{App, Mode};
use config::Config;
use event::{handle_events, AppEvent, DataPayload};

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Parser)]
#[command(name = "vj", version, about = "Vigloo Jira TUI & CLI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 활성 스프린트 작업 항목 조회
    Sprint {
        #[arg(long)]
        json: bool,
    },
    /// 스크럼 데이(어제/오늘/내일) 조회
    Scrum {
        #[arg(long)]
        json: bool,
    },
    /// 내일 스크럼 코멘트 작성
    Write,
    /// Jira 이슈를 브라우저에서 열기
    Open {
        /// sprint 또는 scrum (기본: sprint)
        #[arg(default_value = "sprint")]
        mode: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let config = Config::load()?;
    let client = Arc::new(AcliClient::new(config.jira.board_id, config.jira.project.clone()));

    // CLI subcommand mode
    if let Some(command) = args.command {
        let result = match command {
            Commands::Sprint { json } => cli::cmd_sprint(&client, &config.jira.host, json).await,
            Commands::Scrum { json } => cli::cmd_scrum(&client, json).await,
            Commands::Write => cli::cmd_write(&client).await,
            Commands::Open { mode } => cli::cmd_open(&client, &config.jira.host, &mode).await,
        };
        if let Err(e) = result {
            eprintln!("Error: {:#}", e);
            std::process::exit(1);
        }
        return Ok(());
    }

    // TUI mode (no subcommand)
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, client, &config.jira.host).await;

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
            AppEvent::WriteScrum => {
                // Get today's comment and tomorrow's issue key
                let today_comment = app.today_scrum().and_then(|d| d.my_comment.as_ref());
                let tomorrow_key = app.tomorrow_scrum().map(|d| d.key.clone());

                match (today_comment, tomorrow_key) {
                    (Some(comment), Some(key)) if !key.is_empty() => {
                        if let Some(adf) = comment.build_tomorrow_adf() {
                            let c = client.clone();
                            let tx2 = tx.clone();
                            app.loading = true;
                            tokio::spawn(async move {
                                match c.create_comment(&key, &adf).await {
                                    Ok(()) => {
                                        // Refresh scrum data after writing
                                        let payload = match c.fetch_scrum_data(true).await {
                                            Ok((days, warnings)) => {
                                                DataPayload::Scrum { days, warnings }
                                            }
                                            Err(e) => DataPayload::Error(format!("{:#}", e)),
                                        };
                                        let _ = tx2.send(payload);
                                    }
                                    Err(e) => {
                                        let _ = tx2.send(DataPayload::Error(
                                            format!("Failed to write scrum: {:#}", e),
                                        ));
                                    }
                                }
                            });
                        } else {
                            app.set_error("오늘 코멘트에서 테이블을 찾을 수 없습니다".to_string());
                        }
                    }
                    (None, _) => {
                        app.set_error("오늘 스크럼 코멘트가 없습니다".to_string());
                    }
                    (_, None) | (_, Some(_)) => {
                        app.set_error("내일 스크럼 이슈를 찾을 수 없습니다".to_string());
                    }
                }
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
