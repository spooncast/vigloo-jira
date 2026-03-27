#![allow(dead_code)]

use serde::{Deserialize, Serialize};

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
    pub account_id: String,
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

// -- comment JSON shapes --

#[derive(Debug, Deserialize)]
pub struct CommentViewResponse {
    pub fields: CommentViewFields,
}

#[derive(Debug, Deserialize)]
pub struct CommentViewFields {
    pub comment: CommentWrapper,
}

#[derive(Debug, Deserialize)]
pub struct CommentWrapper {
    pub comments: Vec<CommentRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentRaw {
    pub author: CommentAuthor,
    pub body: serde_json::Value,
    pub created: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentAuthor {
    pub account_id: String,
    pub display_name: String,
}

// -- domain models --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sprint {
    pub id: u64,
    pub name: String,
    pub state: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub issue_type: String,
    pub assignee: String,
    pub priority: String,
    pub subtasks: Vec<Subtask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    pub key: String,
    pub summary: String,
    pub status: String,
    pub assignee: String,
    pub priority: String,
}

// -- scrum domain models --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrumDay {
    pub key: String,
    pub label: String, // "오늘 (03-26)" or "어제 (03-25)"
    pub date: String,  // "2026-03-26"
    pub status: String,
    pub my_comment: Option<ScrumComment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrumComment {
    pub author: String,
    pub created: String,
    pub table: ScrumTable,
    pub raw_body: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrumTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl ScrumComment {
    pub fn from_raw(raw: &CommentRaw) -> Self {
        Self {
            author: raw.author.display_name.clone(),
            created: raw.created[..16].to_string(),
            table: extract_adf_table(&raw.body),
            raw_body: raw.body.clone(),
        }
    }

    /// Extract the raw ADF content of a specific column (0-indexed) from the table's data row.
    pub fn raw_column_content(&self, col_idx: usize) -> Option<serde_json::Value> {
        let content = self.raw_body.get("content")?.as_array()?;
        let table_node = content.iter().find(|n| {
            n.get("type").and_then(|t| t.as_str()) == Some("table")
        })?;
        let rows = table_node.get("content")?.as_array()?;
        // Row 0 is header, row 1+ is data
        let data_row = rows.get(1)?;
        let cells = data_row.get("content")?.as_array()?;
        let cell = cells.get(col_idx)?;
        // Return the cell's content array (the inner nodes)
        Some(cell.get("content").cloned().unwrap_or(serde_json::Value::Array(vec![])))
    }

    /// Build an ADF comment for tomorrow: today's "오늘 할 것" becomes "한 것(어제 한 것)"
    pub fn build_tomorrow_adf(&self) -> Option<serde_json::Value> {
        // Column 1 = "오늘 할 것"
        let today_todo_content = self.raw_column_content(1)?;

        let empty_cell_content = serde_json::json!([
            {"type": "paragraph", "content": [{"type": "text", "text": " "}]}
        ]);

        // Extract original headers from raw ADF
        let content = self.raw_body.get("content")?.as_array()?;
        let table_node = content.iter().find(|n| {
            n.get("type").and_then(|t| t.as_str()) == Some("table")
        })?;
        let rows = table_node.get("content")?.as_array()?;
        let header_row = rows.first()?.clone();

        let adf = serde_json::json!({
            "version": 1,
            "type": "doc",
            "content": [
                {
                    "type": "table",
                    "attrs": {"isNumberColumnEnabled": false, "layout": "default"},
                    "content": [
                        header_row,
                        {
                            "type": "tableRow",
                            "content": [
                                {
                                    "type": "tableCell",
                                    "attrs": {},
                                    "content": today_todo_content
                                },
                                {
                                    "type": "tableCell",
                                    "attrs": {},
                                    "content": empty_cell_content
                                },
                                {
                                    "type": "tableCell",
                                    "attrs": {},
                                    "content": empty_cell_content
                                }
                            ]
                        }
                    ]
                }
            ]
        });

        Some(adf)
    }
}

fn extract_adf_table(body: &serde_json::Value) -> ScrumTable {
    let empty = ScrumTable {
        headers: Vec::new(),
        rows: Vec::new(),
    };

    let content = match body.get("content").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return empty,
    };

    // Find first table node
    let table_node = match content.iter().find(|n| {
        n.get("type").and_then(|t| t.as_str()) == Some("table")
    }) {
        Some(t) => t,
        None => {
            // No table — fallback to plain text
            let text = extract_adf_text_flat(body);
            return ScrumTable {
                headers: vec!["Content".to_string()],
                rows: vec![vec![text]],
            };
        }
    };

    let table_rows = match table_node.get("content").and_then(|c| c.as_array()) {
        Some(r) => r,
        None => return empty,
    };

    let mut headers = Vec::new();
    let mut rows = Vec::new();

    for (i, row) in table_rows.iter().enumerate() {
        let cells = match row.get("content").and_then(|c| c.as_array()) {
            Some(c) => c,
            None => continue,
        };
        let cell_texts: Vec<String> = cells.iter().map(|cell| extract_cell_text(cell)).collect();
        if i == 0 {
            headers = cell_texts;
        } else {
            rows.push(cell_texts);
        }
    }

    ScrumTable { headers, rows }
}

fn extract_cell_text(cell: &serde_json::Value) -> String {
    let content = match cell.get("content").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return String::new(),
    };

    let mut lines = Vec::new();
    extract_block_nodes(content, &mut lines, 0);
    lines.join("\n")
}

fn extract_block_nodes(nodes: &[serde_json::Value], lines: &mut Vec<String>, depth: usize) {
    let indent = "  ".repeat(depth);
    for node in nodes {
        let node_type = node.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match node_type {
            "paragraph" => {
                let text = extract_inline_text(node);
                if !text.is_empty() && text.trim() != "\u{a0}" && text.trim() != "" {
                    lines.push(format!("{}{}", indent, text));
                }
            }
            "bulletList" | "orderedList" => {
                if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
                    for item in items {
                        extract_list_item(item, lines, depth);
                    }
                }
            }
            "listItem" => {
                extract_list_item(node, lines, depth);
            }
            _ => {
                let text = extract_adf_text_flat(node);
                if !text.is_empty() && text.trim() != "\u{a0}" && text.trim() != "" {
                    lines.push(format!("{}{}", indent, text));
                }
            }
        }
    }
}

fn extract_list_item(item: &serde_json::Value, lines: &mut Vec<String>, depth: usize) {
    let indent = "  ".repeat(depth);
    let children = match item.get("content").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return,
    };

    // A listItem can contain: paragraph(s) and nested bulletList/orderedList
    let mut item_text_parts = Vec::new();
    let mut nested_nodes = Vec::new();

    for child in children {
        let child_type = child.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match child_type {
            "paragraph" => {
                let text = extract_inline_text(child);
                if !text.is_empty() && text.trim() != "\u{a0}" && text.trim() != "" {
                    item_text_parts.push(text);
                }
            }
            "bulletList" | "orderedList" => {
                nested_nodes.push(child);
            }
            _ => {
                let text = extract_adf_text_flat(child);
                if !text.is_empty() && text.trim() != "\u{a0}" && text.trim() != "" {
                    item_text_parts.push(text);
                }
            }
        }
    }

    // Push the list item's own text
    if !item_text_parts.is_empty() {
        let combined = item_text_parts.join(" ");
        lines.push(format!("{}• {}", indent, combined));
    }

    // Process nested lists with increased depth
    for nested in nested_nodes {
        if let Some(items) = nested.get("content").and_then(|c| c.as_array()) {
            for sub_item in items {
                extract_list_item(sub_item, lines, depth + 1);
            }
        }
    }
}

fn extract_inline_text(node: &serde_json::Value) -> String {
    let content = match node.get("content").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return String::new(),
    };

    // Check if this paragraph contains only inlineCard(s) and no meaningful text
    let has_meaningful_text = content.iter().any(|item| {
        if item.get("type").and_then(|t| t.as_str()) == Some("text") {
            item.get("text")
                .and_then(|t| t.as_str())
                .map(|t| !t.trim().is_empty())
                .unwrap_or(false)
        } else {
            false
        }
    });
    let only_cards = !has_meaningful_text
        && content.iter().any(|item| {
            item.get("type").and_then(|t| t.as_str()) == Some("inlineCard")
        });

    let mut parts = Vec::new();
    for item in content {
        let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
        match item_type {
            "text" => {
                if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                    parts.push(text.to_string());
                }
            }
            "inlineCard" => {
                if let Some(url) = item
                    .get("attrs")
                    .and_then(|a| a.get("url"))
                    .and_then(|u| u.as_str())
                {
                    if only_cards {
                        // Show full URL when no surrounding text
                        parts.push(url.split('?').next().unwrap_or(url).to_string());
                    } else if url.contains("/browse/") {
                        // Extract Jira key, strip query string
                        let after_browse = url.split("/browse/").last().unwrap_or(url);
                        let key = after_browse.split('?').next().unwrap_or(after_browse);
                        parts.push(key.to_string());
                    } else {
                        parts.push(url.split('?').next().unwrap_or(url).to_string());
                    }
                }
            }
            _ => {}
        }
    }
    parts.join("")
}

fn extract_adf_text_flat(node: &serde_json::Value) -> String {
    let mut texts = Vec::new();
    extract_adf_text_recursive(node, &mut texts);
    texts.join("")
}

fn extract_adf_text_recursive(node: &serde_json::Value, texts: &mut Vec<String>) {
    if let Some(obj) = node.as_object() {
        if obj.get("type").and_then(|t| t.as_str()) == Some("text") {
            if let Some(text) = obj.get("text").and_then(|t| t.as_str()) {
                texts.push(text.to_string());
            }
        }
        if let Some(content) = obj.get("content").and_then(|c| c.as_array()) {
            for child in content {
                extract_adf_text_recursive(child, texts);
            }
        }
    }
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
