#![allow(dead_code)]

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
