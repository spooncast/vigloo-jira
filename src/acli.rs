use anyhow::{Context, Result};
use chrono::{Datelike, Days, Local, NaiveDate, Weekday};
use tokio::process::Command;
use tokio::sync::OnceCell;

use crate::cache;
use crate::model::*;

pub struct AcliClient {
    board_id: u64,
    project: String,
    cached_account_id: OnceCell<String>,
}

impl AcliClient {
    pub fn new(board_id: u64, project: String) -> Self {
        Self {
            board_id,
            project,
            cached_account_id: OnceCell::new(),
        }
    }

    // -- Cache-aware wrappers --

    async fn fetch_active_sprint_cached(&self) -> Result<Sprint> {
        if let Some(sprint) = cache::load_cached_sprint(self.board_id) {
            return Ok(sprint);
        }
        let sprint = self.fetch_active_sprint().await?;
        let _ = cache::save_sprint_cache(self.board_id, &sprint);
        Ok(sprint)
    }

    async fn find_scrum_epic_cached(&self) -> Result<String> {
        if let Some(key) = cache::load_cached_epic_key(&self.project) {
            return Ok(key);
        }
        let key = self.find_scrum_epic().await?;
        let _ = cache::save_epic_key_cache(&self.project, &key);
        Ok(key)
    }

    async fn fetch_current_user_account_id_cached(&self) -> Result<String> {
        self.cached_account_id
            .get_or_try_init(|| self.fetch_current_user_account_id())
            .await
            .cloned()
    }

    // -- Raw fetch methods --

    async fn fetch_active_sprint(&self) -> Result<Sprint> {
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

    async fn fetch_all_work_items(&self, sprint_id: u64) -> Result<Vec<WorkItem>> {
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

    async fn fetch_my_subtasks(parent_key: &str) -> Result<Vec<Subtask>> {
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

        let issues: Vec<IssueRaw> = serde_json::from_slice(&output.stdout)
            .context("Failed to parse subtasks JSON")?;

        Ok(issues.iter().map(Subtask::from).collect())
    }

    // -- Orchestration --

    pub async fn fetch_all_data(&self, force: bool) -> Result<(Sprint, Vec<WorkItem>, Vec<String>)> {
        if !force {
            if let Some(cached) = cache::load_cached_sprint_data(self.board_id) {
                return Ok(cached);
            }
        }

        let sprint = self.fetch_active_sprint_cached().await?;
        let all_work_items = self.fetch_all_work_items(sprint.id).await?;
        let mut warnings = Vec::new();

        // Fetch my subtasks in parallel
        let mut handles = Vec::new();
        for item in &all_work_items {
            let key = item.key.clone();
            handles.push(tokio::spawn(async move {
                (key.clone(), Self::fetch_my_subtasks(&key).await)
            }));
        }

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

        let _ = cache::save_sprint_data_cache(self.board_id, &sprint, &work_items_with_my_subs, &warnings);
        Ok((sprint, work_items_with_my_subs, warnings))
    }

    // -- Scrum methods --

    async fn fetch_current_user_account_id(&self) -> Result<String> {
        let output = Command::new("acli")
            .args([
                "jira", "workitem", "search",
                "--jql", "assignee = currentUser()",
                "--json",
                "--limit", "1",
            ])
            .output()
            .await
            .context("Failed to run acli for currentUser lookup")?;

        if !output.status.success() {
            anyhow::bail!("Failed to fetch current user");
        }

        let issues: Vec<IssueRaw> = serde_json::from_slice(&output.stdout)
            .context("Failed to parse currentUser response")?;

        issues
            .first()
            .and_then(|i| i.fields.assignee.as_ref())
            .map(|a| a.account_id.clone())
            .context("Could not determine current user accountId")
    }

    async fn find_scrum_epic(&self) -> Result<String> {
        let now = Local::now();
        let quarter = (now.month() - 1) / 3 + 1;
        let keyword = format!("{} {}Q", now.year(), quarter);
        let jql = format!(
            "project = {} AND issuetype = Epic AND summary ~ \"{}\" AND summary ~ \"Daily scrum\"",
            self.project, keyword
        );

        let output = Command::new("acli")
            .args(["jira", "workitem", "search", "--jql", &jql, "--json"])
            .output()
            .await
            .context("Failed to search for scrum epic")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Scrum epic search failed: {}", stderr);
        }

        let issues: Vec<IssueRaw> = serde_json::from_slice(&output.stdout)
            .context("Failed to parse scrum epic response")?;

        issues
            .first()
            .map(|i| i.key.clone())
            .with_context(|| format!("No Daily scrum epic found for {}", keyword))
    }

    async fn fetch_scrum_day(&self, epic_key: &str, date: NaiveDate, label: &str) -> Result<ScrumDay> {
        let date_str = date.format("%Y-%m-%d").to_string();
        let jql = format!(
            "parent = {} AND summary ~ \"{}\"",
            epic_key, date_str
        );

        let output = Command::new("acli")
            .args(["jira", "workitem", "search", "--jql", &jql, "--json"])
            .output()
            .await
            .context(format!("{} ({}) 스크럼 데이 검색 실행 실패", label, date_str))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{} ({}) 스크럼 데이 검색 실패: {}", label, date_str, stderr);
        }

        let issues: Vec<IssueRaw> = serde_json::from_slice(&output.stdout)
            .context(format!("{} ({}) 스크럼 데이 응답 파싱 실패", label, date_str))?;

        match issues.first() {
            Some(issue) => Ok(ScrumDay {
                key: issue.key.clone(),
                label: label.to_string(),
                date: date_str,
                status: issue.fields.status.name.clone(),
                my_comment: None,
            }),
            None => Ok(ScrumDay {
                key: String::new(),
                label: label.to_string(),
                date: date_str,
                status: "Not found".to_string(),
                my_comment: None,
            }),
        }
    }

    async fn fetch_my_comment(&self, issue_key: &str, account_id: &str) -> Result<Option<ScrumComment>> {
        let output = Command::new("acli")
            .args([
                "jira", "workitem", "view", issue_key,
                "--fields", "comment",
                "--json",
            ])
            .output()
            .await
            .context("Failed to fetch comments")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Comment fetch failed for {}: {}", issue_key, stderr);
        }

        let response: CommentViewResponse = serde_json::from_slice(&output.stdout)
            .context("Failed to parse comment response")?;

        let my_comment = response
            .fields
            .comment
            .comments
            .iter()
            .rev()
            .find(|c| c.author.account_id == account_id)
            .map(ScrumComment::from_raw);

        Ok(my_comment)
    }

    pub async fn fetch_scrum_data(&self, force: bool) -> Result<(Vec<ScrumDay>, Vec<String>)> {
        if !force {
            if let Some(cached) = cache::load_cached_scrum_data(&self.project) {
                return Ok(cached);
            }
        }

        let mut warnings = Vec::new();

        // Parallel: epic key (cached) + account id (cached)
        let (epic_key, account_id) = tokio::try_join!(
            self.find_scrum_epic_cached(),
            self.fetch_current_user_account_id_cached(),
        )
        .context("Epic key 또는 Account ID 조회 실패")?;

        let today = Local::now().date_naive();
        let yesterday = prev_workday(today);
        let tomorrow = next_workday(today);

        // Parallel: fetch all three scrum days
        let (tomorrow_result, today_result, yesterday_result) = tokio::join!(
            self.fetch_scrum_day(&epic_key, tomorrow, "내일"),
            self.fetch_scrum_day(&epic_key, today, "오늘"),
            self.fetch_scrum_day(&epic_key, yesterday, "어제"),
        );

        let mut tomorrow_scrum = tomorrow_result.context("내일 스크럼 데이 조회 실패")?;
        let mut today_scrum = today_result.context("오늘 스크럼 데이 조회 실패")?;
        let mut yesterday_scrum = yesterday_result.context("어제 스크럼 데이 조회 실패")?;

        // Parallel: fetch comments
        let tomorrow_key = tomorrow_scrum.key.clone();
        let today_key = today_scrum.key.clone();
        let yesterday_key = yesterday_scrum.key.clone();
        let aid1 = account_id.clone();
        let aid2 = account_id.clone();

        let (tomorrow_comment, today_comment, yesterday_comment) = tokio::join!(
            async {
                if tomorrow_key.is_empty() { return Ok(None); }
                self.fetch_my_comment(&tomorrow_key, &aid1).await
            },
            async {
                if today_key.is_empty() { return Ok(None); }
                self.fetch_my_comment(&today_key, &aid2).await
            },
            async {
                if yesterday_key.is_empty() { return Ok(None); }
                self.fetch_my_comment(&yesterday_key, &account_id).await
            }
        );

        match tomorrow_comment {
            Ok(comment) => tomorrow_scrum.my_comment = comment,
            Err(e) => warnings.push(format!("Failed to load tomorrow's comment: {}", e)),
        }
        match today_comment {
            Ok(comment) => today_scrum.my_comment = comment,
            Err(e) => warnings.push(format!("Failed to load today's comment: {}", e)),
        }
        match yesterday_comment {
            Ok(comment) => yesterday_scrum.my_comment = comment,
            Err(e) => warnings.push(format!("Failed to load yesterday's comment: {}", e)),
        }

        let days = vec![yesterday_scrum, today_scrum, tomorrow_scrum];
        let _ = cache::save_scrum_data_cache(&self.project, &days, &warnings);
        Ok((days, warnings))
    }

    pub async fn create_comment(&self, issue_key: &str, adf_body: &serde_json::Value) -> Result<()> {
        let body_str = serde_json::to_string(adf_body)?;
        let output = Command::new("acli")
            .args([
                "jira", "workitem", "comment", "create",
                "--key", issue_key,
                "--body", &body_str,
            ])
            .output()
            .await
            .context("Failed to run acli comment create")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Comment create failed for {}: {}", issue_key, stderr);
        }

        Ok(())
    }
}

fn prev_workday(date: NaiveDate) -> NaiveDate {
    match date.weekday() {
        Weekday::Mon => date - Days::new(3),
        Weekday::Sun => date - Days::new(2),
        _ => date - Days::new(1),
    }
}

fn next_workday(date: NaiveDate) -> NaiveDate {
    match date.weekday() {
        Weekday::Fri => date + Days::new(3),
        Weekday::Sat => date + Days::new(2),
        _ => date + Days::new(1),
    }
}
