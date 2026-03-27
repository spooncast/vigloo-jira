use crate::model::{ScrumDay, Sprint, WorkItem};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Sprint,
    Scrum,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Panel {
    Left,
    Right,
}

pub struct App {
    // Global
    pub mode: Mode,
    pub active_panel: Panel,
    pub loading: bool,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub jira_host: String,

    // Sprint mode
    pub sprint: Option<Sprint>,
    pub work_items: Vec<WorkItem>,
    pub selected_work_item: usize,
    pub selected_subtask: usize,

    // Scrum mode
    pub scrum_days: Vec<ScrumDay>,
    pub selected_scrum_day: usize,
    pub scrum_scroll: u16,
}

impl App {
    pub fn new(jira_host: String) -> Self {
        Self {
            mode: Mode::Sprint,
            active_panel: Panel::Left,
            loading: true,
            error: None,
            warnings: Vec::new(),
            jira_host,
            sprint: None,
            work_items: Vec::new(),
            selected_work_item: 0,
            selected_subtask: 0,
            scrum_days: Vec::new(),
            selected_scrum_day: 0,
            scrum_scroll: 0,
        }
    }

    pub fn set_data(&mut self, sprint: Sprint, work_items: Vec<WorkItem>) {
        self.sprint = Some(sprint);
        self.work_items = work_items;
        self.selected_work_item = 0;
        self.selected_subtask = 0;
        self.loading = false;
        self.error = None;
        self.warnings.clear();
    }

    pub fn set_scrum_data(&mut self, days: Vec<ScrumDay>) {
        self.scrum_days = days;
        self.selected_scrum_day = 0;
        self.scrum_scroll = 0;
        self.loading = false;
        self.error = None;
    }

    pub fn add_warning(&mut self, msg: String) {
        self.warnings.push(msg);
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

    pub fn current_scrum_comment(&self) -> Option<&crate::model::ScrumComment> {
        self.scrum_days
            .get(self.selected_scrum_day)
            .and_then(|d| d.my_comment.as_ref())
    }

    pub fn move_up(&mut self) {
        match self.mode {
            Mode::Sprint => match self.active_panel {
                Panel::Left => {
                    if self.selected_work_item > 0 {
                        self.selected_work_item -= 1;
                        self.selected_subtask = 0;
                    }
                }
                Panel::Right => {
                    if self.selected_subtask > 0 {
                        self.selected_subtask -= 1;
                    }
                }
            },
            Mode::Scrum => {
                if self.scrum_scroll > 0 {
                    self.scrum_scroll -= 1;
                }
            }
        }
    }

    pub fn move_down(&mut self) {
        match self.mode {
            Mode::Sprint => match self.active_panel {
                Panel::Left => {
                    if self.selected_work_item + 1 < self.work_items.len() {
                        self.selected_work_item += 1;
                        self.selected_subtask = 0;
                    }
                }
                Panel::Right => {
                    let len = self.current_subtasks().len();
                    if self.selected_subtask + 1 < len {
                        self.selected_subtask += 1;
                    }
                }
            },
            Mode::Scrum => {
                self.scrum_scroll += 1;
            }
        }
    }

    pub fn move_left(&mut self) {
        if self.mode == Mode::Scrum && self.selected_scrum_day > 0 {
            self.selected_scrum_day -= 1;
            self.scrum_scroll = 0;
        }
    }

    pub fn move_right(&mut self) {
        if self.mode == Mode::Scrum && self.selected_scrum_day + 1 < self.scrum_days.len() {
            self.selected_scrum_day += 1;
            self.scrum_scroll = 0;
        }
    }

    pub fn toggle_panel(&mut self) {
        if self.mode == Mode::Sprint {
            self.active_panel = match self.active_panel {
                Panel::Left => Panel::Right,
                Panel::Right => Panel::Left,
            };
        }
    }

    pub fn go_back(&mut self) {
        match self.mode {
            Mode::Sprint => {
                if self.active_panel == Panel::Right {
                    self.active_panel = Panel::Left;
                }
            }
            Mode::Scrum => {
                // no panel navigation in scrum mode
            }
        }
    }

    pub fn switch_mode(&mut self, mode: Mode) -> bool {
        if self.mode != mode {
            self.mode = mode;
            self.active_panel = Panel::Left;
            self.error = None;
            true
        } else {
            false
        }
    }

    pub fn handle_enter(&mut self) -> Option<crate::event::AppEvent> {
        match self.mode {
            Mode::Sprint => match self.active_panel {
                Panel::Left => {
                    if !self.current_subtasks().is_empty() {
                        self.active_panel = Panel::Right;
                    }
                    None
                }
                Panel::Right => {
                    let subtasks = self.current_subtasks();
                    subtasks.get(self.selected_subtask).map(|sub| {
                        let url = format!("{}/browse/{}", self.jira_host, sub.key);
                        crate::event::AppEvent::OpenLink(url)
                    })
                }
            },
            Mode::Scrum => {
                // Enter opens the scrum day in browser
                self.scrum_days.get(self.selected_scrum_day).and_then(|day| {
                    if day.key.is_empty() {
                        None
                    } else {
                        Some(crate::event::AppEvent::OpenLink(
                            format!("{}/browse/{}", self.jira_host, day.key),
                        ))
                    }
                })
            }
        }
    }
}
