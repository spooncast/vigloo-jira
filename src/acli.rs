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

    pub async fn fetch_all_work_items(&self, sprint_id: u64) -> Result<Vec<WorkItem>> {
        let output = Command::new("acli")
            .args([
                "jira", "sprint", "list-workitems",
                "--sprint", &sprint_id.to_string(),
                "--board", &self.board_id.to_string(),
                "--json",
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

    pub async fn fetch_my_subtasks(&self, parent_key: &str) -> Result<Vec<Subtask>> {
        let jql = format!("parent = {} AND assignee = currentUser()", parent_key);
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

        // workitem search returns a plain array [...], not { "issues": [...] }
        let issues: Vec<IssueRaw> = serde_json::from_slice(&output.stdout)
            .context("Failed to parse subtasks JSON")?;

        Ok(issues.iter().map(Subtask::from).collect())
    }

    pub async fn fetch_all_data(&self) -> Result<(Sprint, Vec<WorkItem>, Vec<String>)> {
        let sprint = self.fetch_active_sprint().await?;
        let all_work_items = self.fetch_all_work_items(sprint.id).await?;
        let mut warnings = Vec::new();

        // Fetch my subtasks in parallel for all work items
        let mut handles = Vec::new();
        for item in &all_work_items {
            let key = item.key.clone();
            let board_id = self.board_id;
            handles.push(tokio::spawn(async move {
                let client = AcliClient::new(board_id);
                (key.clone(), client.fetch_my_subtasks(&key).await)
            }));
        }

        // Collect results, keep only work items with my subtasks
        let mut work_items_with_my_subs: Vec<WorkItem> = Vec::new();
        for handle in handles {
            let (key, result) = handle.await?;
            match result {
                Ok(subtasks) => {
                    if !subtasks.is_empty() {
                        if let Some(mut item) = all_work_items.iter().find(|w| w.key == key).cloned() {
                            item.subtasks = subtasks;
                            work_items_with_my_subs.push(item);
                        }
                    }
                }
                Err(e) => {
                    warnings.push(format!("Failed to load subtasks for {}: {}", key, e));
                }
            }
        }

        Ok((sprint, work_items_with_my_subs, warnings))
    }
}
