# vigloo-jira TUI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust TUI that shows the current user's Jira work items and subtasks for the active sprint via acli.

**Architecture:** Rust binary using Ratatui + crossterm for the TUI, tokio for async acli subprocess calls, serde for JSON parsing. The app shells out to `acli` for all Jira data, keeping auth concerns out of our code. Left-right split panel layout: work items on the left, subtasks on the right.

**Tech Stack:** Rust, ratatui, crossterm, tokio, serde/serde_json, toml, dirs

---

## File Structure

```
vigloo-jira/
├── Cargo.toml            — project manifest with all dependencies
├── src/
│   ├── main.rs           — entrypoint: terminal setup, app init, event loop
│   ├── model.rs          — Sprint, WorkItem, Subtask structs + JSON deserialization
│   ├── config.rs         — Config struct, TOML loading with defaults
│   ├── acli.rs           — async acli subprocess calls, returns domain models
│   ├── app.rs            — App state: sprint, work items, selection, panel focus
│   ├── event.rs          — keyboard event handling → App state mutations
│   └── ui.rs             — ratatui rendering: header, left panel, right panel, footer
```

> Note: `config.rs` was split from the spec's architecture (which had config in app.rs) because config loading is a distinct responsibility with its own struct and file I/O.

---

### Task 1: Project Scaffold & Data Models

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/model.rs`

- [ ] **Step 1: Initialize Cargo project**

```bash
cd /Users/raymond/Developer/vigloo-jira
cargo init --name vigloo-jira
```

- [ ] **Step 2: Set up Cargo.toml with all dependencies**

Replace the generated `Cargo.toml` with:

```toml
[package]
name = "vigloo-jira"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.29"
crossterm = "0.28"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
dirs = "6"
anyhow = "1"
```

- [ ] **Step 3: Create data models in `src/model.rs`**

These structs map to the acli JSON output. The `Deserialize` impls use `#[serde(rename_all)]` and field aliases to handle the nested Jira JSON structure.

```rust
use serde::Deserialize;

// -- acli response wrappers --

#[derive(Debug, Deserialize)]
pub struct SprintListResponse {
    pub sprints: Vec<SprintRaw>,
}

#[derive(Debug, Deserialize)]
pub struct IssueSearchResponse {
    pub issues: Vec<IssueRaw>,
}

// -- raw JSON shapes from acli --

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SprintRaw {
    pub id: u64,
    pub name: String,
    pub state: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub goal: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IssueRaw {
    pub key: String,
    pub fields: IssueFields,
}

#[derive(Debug, Deserialize)]
pub struct IssueFields {
    pub summary: String,
    pub status: StatusField,
    pub assignee: Option<AssigneeField>,
    pub issuetype: IssueTypeField,
    pub priority: Option<PriorityField>,
}

#[derive(Debug, Deserialize)]
pub struct StatusField {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssigneeField {
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct IssueTypeField {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct PriorityField {
    pub name: String,
}

// -- domain models --

#[derive(Debug, Clone)]
pub struct Sprint {
    pub id: u64,
    pub name: String,
    pub state: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Clone)]
pub struct WorkItem {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub issue_type: String,
    pub assignee: String,
    pub priority: String,
    pub subtasks: Vec<Subtask>,
}

#[derive(Debug, Clone)]
pub struct Subtask {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub assignee: String,
    pub priority: String,
}

// -- conversions --

impl From<SprintRaw> for Sprint {
    fn from(raw: SprintRaw) -> Self {
        Self {
            id: raw.id,
            name: raw.name,
            state: raw.state,
            start_date: raw.start_date.unwrap_or_default(),
            end_date: raw.end_date.unwrap_or_default(),
        }
    }
}

impl From<&IssueRaw> for WorkItem {
    fn from(raw: &IssueRaw) -> Self {
        Self {
            key: raw.key.clone(),
            summary: raw.fields.summary.clone(),
            status: raw.fields.status.name.clone(),
            issue_type: raw.fields.issuetype.name.clone(),
            assignee: raw
                .fields
                .assignee
                .as_ref()
                .map(|a| a.display_name.clone())
                .unwrap_or_else(|| "Unassigned".to_string()),
            priority: raw
                .fields
                .priority
                .as_ref()
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "None".to_string()),
            subtasks: Vec::new(),
        }
    }
}

impl From<&IssueRaw> for Subtask {
    fn from(raw: &IssueRaw) -> Self {
        Self {
            key: raw.key.clone(),
            summary: raw.fields.summary.clone(),
            status: raw.fields.status.name.clone(),
            assignee: raw
                .fields
                .assignee
                .as_ref()
                .map(|a| a.display_name.clone())
                .unwrap_or_else(|| "Unassigned".to_string()),
            priority: raw
                .fields
                .priority
                .as_ref()
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "None".to_string()),
        }
    }
}
```

- [ ] **Step 4: Minimal main.rs that compiles**

```rust
mod model;

fn main() {
    println!("vigloo-jira");
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build`
Expected: compiles with no errors

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs src/model.rs
git commit -m "feat: scaffold project with data models"
```

---

### Task 2: Configuration

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/config.rs`**

```rust
use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_jira")]
    pub jira: JiraConfig,
}

#[derive(Debug, Deserialize)]
pub struct JiraConfig {
    #[serde(default = "default_board_id")]
    pub board_id: u64,
}

fn default_board_id() -> u64 {
    272
}

fn default_jira() -> JiraConfig {
    JiraConfig {
        board_id: default_board_id(),
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            jira: default_jira(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("vigloo-jira")
            .join("config.toml")
    }
}
```

- [ ] **Step 2: Update `src/main.rs` to load config**

```rust
mod config;
mod model;

use config::Config;

fn main() {
    let config = Config::load().expect("Failed to load config");
    println!("Board ID: {}", config.jira.board_id);
}
```

- [ ] **Step 3: Verify it compiles and runs**

Run: `cargo run`
Expected: prints `Board ID: 272`

- [ ] **Step 4: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "feat: add TOML config with defaults"
```

---

### Task 3: acli Client

**Files:**
- Create: `src/acli.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/acli.rs`**

This module shells out to `acli` and returns domain models. All three acli commands are here.

```rust
use anyhow::{Context, Result};
use tokio::process::Command;

use crate::model::*;

pub struct AcliClient {
    board_id: u64,
}

impl AcliClient {
    pub fn new(board_id: u64) -> Self {
        Self { board_id }
    }

    pub async fn fetch_active_sprint(&self) -> Result<Sprint> {
        let output = Command::new("acli")
            .args([
                "jira", "board", "list-sprints",
                "--id", &self.board_id.to_string(),
                "--json",
            ])
            .output()
            .await
            .context("Failed to run acli. Is it installed and in PATH?")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("acli list-sprints failed: {}", stderr);
        }

        let response: SprintListResponse = serde_json::from_slice(&output.stdout)
            .context("Failed to parse sprint list JSON")?;

        response
            .sprints
            .into_iter()
            .find(|s| s.state == "active")
            .map(Sprint::from)
            .context("No active sprint found")
    }

    pub async fn fetch_my_work_items(&self, sprint_id: u64) -> Result<Vec<WorkItem>> {
        let output = Command::new("acli")
            .args([
                "jira", "sprint", "list-workitems",
                "--sprint", &sprint_id.to_string(),
                "--board", &self.board_id.to_string(),
                "--json",
                "--jql", "assignee = currentUser()",
                "--paginate",
            ])
            .output()
            .await
            .context("Failed to run acli list-workitems")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("acli list-workitems failed: {}", stderr);
        }

        let response: IssueSearchResponse = serde_json::from_slice(&output.stdout)
            .context("Failed to parse work items JSON")?;

        Ok(response.issues.iter().map(WorkItem::from).collect())
    }

    pub async fn fetch_subtasks(&self, parent_key: &str) -> Result<Vec<Subtask>> {
        let jql = format!("parent = {}", parent_key);
        let output = Command::new("acli")
            .args([
                "jira", "workitem", "search",
                "--jql", &jql,
                "--json",
            ])
            .output()
            .await
            .context("Failed to run acli workitem search")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("acli subtask search failed for {}: {}", parent_key, stderr);
        }

        let response: IssueSearchResponse = serde_json::from_slice(&output.stdout)
            .context("Failed to parse subtasks JSON")?;

        Ok(response.issues.iter().map(Subtask::from).collect())
    }

    pub async fn fetch_all_data(&self) -> Result<(Sprint, Vec<WorkItem>)> {
        let sprint = self.fetch_active_sprint().await?;
        let mut work_items = self.fetch_my_work_items(sprint.id).await?;

        // Fetch subtasks in parallel for all work items
        let mut handles = Vec::new();
        for item in &work_items {
            let key = item.key.clone();
            let board_id = self.board_id;
            handles.push(tokio::spawn(async move {
                let client = AcliClient::new(board_id);
                (key.clone(), client.fetch_subtasks(&key).await)
            }));
        }

        for handle in handles {
            let (key, result) = handle.await?;
            if let Ok(subtasks) = result {
                if let Some(item) = work_items.iter_mut().find(|w| w.key == key) {
                    item.subtasks = subtasks;
                }
            }
        }

        Ok((sprint, work_items))
    }
}
```

- [ ] **Step 2: Update `src/main.rs` to test acli integration**

```rust
mod acli;
mod config;
mod model;

use acli::AcliClient;
use config::Config;

#[tokio::main]
async fn main() {
    let config = Config::load().expect("Failed to load config");
    let client = AcliClient::new(config.jira.board_id);

    println!("Fetching data...");
    match client.fetch_all_data().await {
        Ok((sprint, work_items)) => {
            println!("Sprint: {} ({})", sprint.name, sprint.state);
            for item in &work_items {
                println!("  {} [{}] {}", item.key, item.status, item.summary);
                for sub in &item.subtasks {
                    println!("    └─ {} [{}] {} ({})", sub.key, sub.status, sub.summary, sub.assignee);
                }
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

- [ ] **Step 3: Run and verify real data**

Run: `cargo run`
Expected: prints active sprint name and your work items with subtasks from board 272

- [ ] **Step 4: Commit**

```bash
git add src/acli.rs src/main.rs
git commit -m "feat: add acli client with sprint, workitem, subtask fetching"
```

---

### Task 4: App State

**Files:**
- Create: `src/app.rs`

- [ ] **Step 1: Create `src/app.rs`**

```rust
use crate::model::{Sprint, WorkItem};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Panel {
    WorkItems,
    Subtasks,
}

pub struct App {
    pub sprint: Option<Sprint>,
    pub work_items: Vec<WorkItem>,
    pub selected_work_item: usize,
    pub selected_subtask: usize,
    pub active_panel: Panel,
    pub loading: bool,
    pub error: Option<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            sprint: None,
            work_items: Vec::new(),
            selected_work_item: 0,
            selected_subtask: 0,
            active_panel: Panel::WorkItems,
            loading: true,
            error: None,
            should_quit: false,
        }
    }

    pub fn set_data(&mut self, sprint: Sprint, work_items: Vec<WorkItem>) {
        self.sprint = Some(sprint);
        self.work_items = work_items;
        self.selected_work_item = 0;
        self.selected_subtask = 0;
        self.loading = false;
        self.error = None;
    }

    pub fn set_error(&mut self, msg: String) {
        self.error = Some(msg);
        self.loading = false;
    }

    pub fn current_subtasks(&self) -> &[crate::model::Subtask] {
        self.work_items
            .get(self.selected_work_item)
            .map(|w| w.subtasks.as_slice())
            .unwrap_or(&[])
    }

    pub fn move_up(&mut self) {
        match self.active_panel {
            Panel::WorkItems => {
                if self.selected_work_item > 0 {
                    self.selected_work_item -= 1;
                    self.selected_subtask = 0;
                }
            }
            Panel::Subtasks => {
                if self.selected_subtask > 0 {
                    self.selected_subtask -= 1;
                }
            }
        }
    }

    pub fn move_down(&mut self) {
        match self.active_panel {
            Panel::WorkItems => {
                if self.selected_work_item + 1 < self.work_items.len() {
                    self.selected_work_item += 1;
                    self.selected_subtask = 0;
                }
            }
            Panel::Subtasks => {
                let len = self.current_subtasks().len();
                if self.selected_subtask + 1 < len {
                    self.selected_subtask += 1;
                }
            }
        }
    }

    pub fn toggle_panel(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::WorkItems => Panel::Subtasks,
            Panel::Subtasks => Panel::WorkItems,
        };
    }
}
```

- [ ] **Step 2: Add `mod app;` to `src/main.rs`**

Add `mod app;` to the module declarations at the top of `src/main.rs` (after `mod acli;`).

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat: add App state with panel navigation"
```

---

### Task 5: Event Handling

**Files:**
- Create: `src/event.rs`

- [ ] **Step 1: Create `src/event.rs`**

```rust
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

use crate::app::App;

pub enum AppEvent {
    Quit,
    Refresh,
    None,
}

pub fn handle_events(app: &mut App) -> anyhow::Result<AppEvent> {
    if event::poll(std::time::Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(AppEvent::None);
            }
            match key.code {
                KeyCode::Char('q') => {
                    app.should_quit = true;
                    return Ok(AppEvent::Quit);
                }
                KeyCode::Char('r') => return Ok(AppEvent::Refresh),
                KeyCode::Up => app.move_up(),
                KeyCode::Down => app.move_down(),
                KeyCode::Tab => app.toggle_panel(),
                _ => {}
            }
        }
    }
    Ok(AppEvent::None)
}
```

- [ ] **Step 2: Add `mod event;` to `src/main.rs`**

Add `mod event;` to the module declarations at the top of `src/main.rs`.

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add src/event.rs src/main.rs
git commit -m "feat: add keyboard event handling"
```

---

### Task 6: UI Rendering

**Files:**
- Create: `src/ui.rs`

- [ ] **Step 1: Create `src/ui.rs`**

```rust
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
```

- [ ] **Step 2: Add `mod ui;` to `src/main.rs`**

Add `mod ui;` to the module declarations at the top of `src/main.rs`.

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add src/ui.rs src/main.rs
git commit -m "feat: add TUI rendering with split panel layout"
```

---

### Task 7: Wire Everything Together in main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Rewrite `src/main.rs` with the full event loop**

```rust
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

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &client).await;

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
) -> Result<()> {
    let mut app = App::new();

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
            AppEvent::None => {}
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

async fn load_data(app: &mut App, client: &AcliClient) {
    match client.fetch_all_data().await {
        Ok((sprint, work_items)) => app.set_data(sprint, work_items),
        Err(e) => app.set_error(format!("{:#}", e)),
    }
}
```

- [ ] **Step 2: Run the full TUI**

Run: `cargo run`
Expected: TUI launches with the active sprint header, left panel showing your work items, right panel showing subtasks for the selected item. Arrow keys move selection, Tab switches panels, `r` refreshes, `q` quits.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire up full TUI event loop"
```

---

### Task 8: Polish & Final Touches

**Files:**
- Modify: `src/ui.rs`
- Create: `.gitignore`

- [ ] **Step 1: Create `.gitignore`**

```
/target
.superpowers/
```

- [ ] **Step 2: Add empty state message in `src/ui.rs`**

In `render_subtasks`, after `let subtasks = app.current_subtasks();`, add handling for empty subtasks:

Find this code in `render_subtasks`:
```rust
    let subtasks = app.current_subtasks();
    let items: Vec<ListItem> = subtasks
```

Replace with:
```rust
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
```

- [ ] **Step 3: Verify it compiles and runs**

Run: `cargo run`
Expected: TUI works correctly. Work items with no subtasks show "No subtasks" in the right panel.

- [ ] **Step 4: Commit**

```bash
git add .gitignore src/ui.rs
git commit -m "feat: add gitignore and empty state for subtasks"
```

- [ ] **Step 5: Final verification**

Run: `cargo run`
Expected: Full TUI working end-to-end:
1. Header shows active sprint name and work item count
2. Left panel lists your assigned work items with status colors
3. Right panel shows subtasks for selected work item
4. Arrow keys, Tab, `r`, `q` all work correctly
