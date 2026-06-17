use crate::model::{ScrumDay, Sprint, WorkItem};

/// CLIP 워크플로 서브태스크 상태. 모두 global transition 이라 현재 상태와 무관하게 전이 가능.
pub const SUBTASK_STATUSES: [&str; 4] = ["해야 할 일", "진행 중", "검토 중", "완료"];

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
    pub confirm_write: bool,

    // 상태 변경 피커: Some(idx) = 열림(idx 강조), None = 닫힘
    pub status_picker: Option<usize>,
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
            confirm_write: false,
            status_picker: None,
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
        self.selected_scrum_day = 1;
        self.scrum_scroll = 0;
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

    pub fn current_scrum_comment(&self) -> Option<&crate::model::ScrumComment> {
        self.scrum_days
            .get(self.selected_scrum_day)
            .and_then(|d| d.my_comment.as_ref())
    }

    pub fn today_scrum(&self) -> Option<&crate::model::ScrumDay> {
        self.scrum_days.iter().find(|d| d.label == "오늘")
    }

    pub fn tomorrow_scrum(&self) -> Option<&crate::model::ScrumDay> {
        self.scrum_days.iter().find(|d| d.label == "내일")
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

    // -- 상태 변경 피커 --

    /// Sprint 모드 Right 패널에서 선택된 서브태스크가 있을 때만 피커를 연다.
    /// 시작 강조 위치는 현재 상태로 맞춘다(없으면 첫 항목).
    pub fn open_status_picker(&mut self) {
        if self.mode != Mode::Sprint || self.active_panel != Panel::Right {
            return;
        }
        let Some(current) = self
            .current_subtasks()
            .get(self.selected_subtask)
            .map(|s| s.status.clone())
        else {
            return;
        };

        let idx = SUBTASK_STATUSES
            .iter()
            .position(|s| *s == current)
            .unwrap_or(0);
        self.status_picker = Some(idx);
    }

    pub fn close_status_picker(&mut self) {
        self.status_picker = None;
    }

    pub fn status_picker_up(&mut self) {
        if let Some(idx) = self.status_picker {
            if idx > 0 {
                self.status_picker = Some(idx - 1);
            }
        }
    }

    pub fn status_picker_down(&mut self) {
        if let Some(idx) = self.status_picker {
            if idx + 1 < SUBTASK_STATUSES.len() {
                self.status_picker = Some(idx + 1);
            }
        }
    }

    /// 현재 강조된 상태를 로컬에 낙관적으로 반영하고 전이 이벤트를 만든다.
    /// 실제 전이는 백그라운드에서 수행, 실패 시에만 에러로 되돌린다.
    pub fn confirm_status_pick(&mut self) -> Option<crate::event::AppEvent> {
        let idx = self.status_picker.take()?;
        let status = SUBTASK_STATUSES.get(idx).copied()?;

        let wi = self.selected_work_item;
        let si = self.selected_subtask;
        let sub = self.work_items.get_mut(wi)?.subtasks.get_mut(si)?;
        let key = sub.key.clone();
        sub.status = status.to_string();

        Some(crate::event::AppEvent::Transition { key, status: status.to_string() })
    }
}
