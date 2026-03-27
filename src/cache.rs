use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::model::{ScrumDay, Sprint, WorkItem};

#[derive(Serialize, Deserialize)]
struct CachedSprint {
    sprint: Sprint,
    valid_until: String,
}

#[derive(Serialize, Deserialize)]
struct CachedEpicKey {
    epic_key: String,
    year: i32,
    quarter: u32,
}

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("vigloo-jira")
}

fn sprint_path(board_id: u64) -> PathBuf {
    cache_dir().join(format!("sprint_{}.json", board_id))
}

fn epic_path(project: &str, year: i32, quarter: u32) -> PathBuf {
    cache_dir().join(format!("scrum_epic_{}_{}q{}.json", project, year, quarter))
}

pub fn load_cached_sprint(board_id: u64) -> Option<Sprint> {
    let content = fs::read_to_string(sprint_path(board_id)).ok()?;
    let cached: CachedSprint = serde_json::from_str(&content).ok()?;
    // end_date comes as "2026-04-08T15:03:49.000Z" — extract date part
    let date_str = cached.valid_until.split('T').next().unwrap_or(&cached.valid_until);
    let valid_until = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
    let today = Local::now().date_naive();
    if today <= valid_until {
        Some(cached.sprint)
    } else {
        None
    }
}

pub fn save_sprint_cache(board_id: u64, sprint: &Sprint) -> Result<()> {
    let dir = cache_dir();
    fs::create_dir_all(&dir)?;
    let cached = CachedSprint {
        sprint: sprint.clone(),
        valid_until: sprint.end_date.clone(),
    };
    let json = serde_json::to_string_pretty(&cached)?;
    fs::write(sprint_path(board_id), json)?;
    Ok(())
}

pub fn load_cached_epic_key(project: &str) -> Option<String> {
    let now = Local::now();
    let year = now.year();
    let quarter = (now.month() - 1) / 3 + 1;
    let content = fs::read_to_string(epic_path(project, year, quarter)).ok()?;
    let cached: CachedEpicKey = serde_json::from_str(&content).ok()?;
    if cached.year == year && cached.quarter == quarter {
        Some(cached.epic_key)
    } else {
        None
    }
}

pub fn save_epic_key_cache(project: &str, epic_key: &str) -> Result<()> {
    let now = Local::now();
    let year = now.year();
    let quarter = (now.month() - 1) / 3 + 1;
    let dir = cache_dir();
    fs::create_dir_all(&dir)?;
    let cached = CachedEpicKey {
        epic_key: epic_key.to_string(),
        year,
        quarter,
    };
    let json = serde_json::to_string_pretty(&cached)?;
    fs::write(epic_path(project, year, quarter), json)?;
    Ok(())
}

// -- TTL-based cache for sprint data and scrum data --

const TTL_SECS: u64 = 5 * 60; // 5 minutes

#[derive(Serialize, Deserialize)]
struct CachedSprintData {
    sprint: Sprint,
    work_items: Vec<WorkItem>,
    warnings: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct CachedScrumData {
    days: Vec<ScrumDay>,
    warnings: Vec<String>,
}

fn sprint_data_path(board_id: u64) -> PathBuf {
    cache_dir().join(format!("sprint_data_{}.json", board_id))
}

fn scrum_data_path(project: &str) -> PathBuf {
    let today = Local::now().format("%Y-%m-%d");
    cache_dir().join(format!("scrum_data_{}_{}.json", project, today))
}

fn is_file_fresh(path: &PathBuf) -> bool {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|modified| {
            SystemTime::now()
                .duration_since(modified)
                .map(|d| d.as_secs() < TTL_SECS)
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

pub fn load_cached_sprint_data(board_id: u64) -> Option<(Sprint, Vec<WorkItem>, Vec<String>)> {
    let path = sprint_data_path(board_id);
    if !is_file_fresh(&path) {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    let cached: CachedSprintData = serde_json::from_str(&content).ok()?;
    Some((cached.sprint, cached.work_items, cached.warnings))
}

pub fn save_sprint_data_cache(
    board_id: u64,
    sprint: &Sprint,
    work_items: &[WorkItem],
    warnings: &[String],
) -> Result<()> {
    let dir = cache_dir();
    fs::create_dir_all(&dir)?;
    let cached = CachedSprintData {
        sprint: sprint.clone(),
        work_items: work_items.to_vec(),
        warnings: warnings.to_vec(),
    };
    let json = serde_json::to_string(&cached)?;
    fs::write(sprint_data_path(board_id), json)?;
    Ok(())
}

pub fn load_cached_scrum_data(project: &str) -> Option<(Vec<ScrumDay>, Vec<String>)> {
    let path = scrum_data_path(project);
    if !is_file_fresh(&path) {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    let cached: CachedScrumData = serde_json::from_str(&content).ok()?;
    Some((cached.days, cached.warnings))
}

pub fn save_scrum_data_cache(
    project: &str,
    days: &[ScrumDay],
    warnings: &[String],
) -> Result<()> {
    let dir = cache_dir();
    fs::create_dir_all(&dir)?;
    let cached = CachedScrumData {
        days: days.to_vec(),
        warnings: warnings.to_vec(),
    };
    let json = serde_json::to_string(&cached)?;
    fs::write(scrum_data_path(project), json)?;
    Ok(())
}
