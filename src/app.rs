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
    pub warnings: Vec<String>,
    pub jira_host: String,
}

impl App {
    pub fn new(jira_host: String) -> Self {
        Self {
            sprint: None,
            work_items: Vec::new(),
            selected_work_item: 0,
            selected_subtask: 0,
            active_panel: Panel::WorkItems,
            loading: true,
            error: None,
            warnings: Vec::new(),
            jira_host,
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

    pub fn go_back(&mut self) {
        if self.active_panel == Panel::Subtasks {
            self.active_panel = Panel::WorkItems;
        }
    }

    pub fn handle_enter(&mut self) -> Option<crate::event::AppEvent> {
        match self.active_panel {
            Panel::WorkItems => {
                if !self.current_subtasks().is_empty() {
                    self.active_panel = Panel::Subtasks;
                }
                None
            }
            Panel::Subtasks => {
                let subtasks = self.current_subtasks();
                subtasks.get(self.selected_subtask).map(|sub| {
                    let url = format!("{}/browse/{}", self.jira_host, sub.key);
                    crate::event::AppEvent::OpenLink(url)
                })
            }
        }
    }
}
