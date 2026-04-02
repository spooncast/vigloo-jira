use anyhow::{Context, Result};

use crate::acli::AcliClient;

pub async fn cmd_sprint(client: &AcliClient, _host: &str, json: bool) -> Result<()> {
    let (sprint, work_items, warnings) = client
        .fetch_all_data(false)
        .await
        .context("스프린트 데이터 조회 실패")?;

    if json {
        let output = serde_json::json!({
            "sprint": sprint,
            "work_items": work_items,
            "warnings": warnings,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Text output
    let start = sprint.start_date.split('T').next().unwrap_or(&sprint.start_date);
    let end = sprint.end_date.split('T').next().unwrap_or(&sprint.end_date);
    println!("Sprint: {} ({} ~ {})", sprint.name, start, end);
    println!();

    if work_items.is_empty() {
        println!("작업 항목이 없습니다.");
    } else {
        let key_w = work_items.iter().map(|w| w.key.len()).max().unwrap_or(10).max(3);
        let status_w = work_items.iter().map(|w| w.status.len()).max().unwrap_or(10).max(6);
        let assignee_w = work_items.iter().map(|w| w.assignee.len()).max().unwrap_or(10).max(8);

        println!(
            "{:<key_w$}  {:<status_w$}  {:<assignee_w$}  SUMMARY",
            "KEY", "STATUS", "ASSIGNEE",
            key_w = key_w, status_w = status_w, assignee_w = assignee_w,
        );
        println!("{}", "-".repeat(key_w + status_w + assignee_w + 50));

        for item in &work_items {
            println!(
                "{:<key_w$}  {:<status_w$}  {:<assignee_w$}  {}",
                item.key, item.status, item.assignee, item.summary,
                key_w = key_w, status_w = status_w, assignee_w = assignee_w,
            );
            for sub in &item.subtasks {
                println!(
                    "  {:<key_w$}  {:<status_w$}  {:<assignee_w$}  {}",
                    sub.key, sub.status, sub.assignee, sub.summary,
                    key_w = key_w, status_w = status_w, assignee_w = assignee_w,
                );
            }
        }
    }

    for w in &warnings {
        eprintln!("Warning: {}", w);
    }

    Ok(())
}

pub async fn cmd_scrum(client: &AcliClient, json: bool) -> Result<()> {
    let (days, warnings) = client
        .fetch_scrum_data(false)
        .await
        .context("스크럼 데이터 조회 실패")?;

    if json {
        let output = serde_json::json!({
            "days": days,
            "warnings": warnings,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Text output
    println!("Scrum Status");
    println!();

    let label_w = 6;
    let date_w = 10;
    let status_w = days.iter().map(|d| d.status.len()).max().unwrap_or(10).max(6);

    println!(
        "{:<label_w$}  {:<date_w$}  {:<status_w$}  COMMENT",
        "DAY", "DATE", "STATUS",
        label_w = label_w, date_w = date_w, status_w = status_w,
    );
    println!("{}", "-".repeat(label_w + date_w + status_w + 16));

    for day in &days {
        let comment_mark = if day.my_comment.is_some() { "v" } else { "-" };
        println!(
            "{:<label_w$}  {:<date_w$}  {:<status_w$}  {}",
            day.label, day.date, day.status, comment_mark,
            label_w = label_w, date_w = date_w, status_w = status_w,
        );
    }

    for w in &warnings {
        eprintln!("Warning: {}", w);
    }

    Ok(())
}

pub async fn cmd_write(client: &AcliClient) -> Result<()> {
    let (days, _) = client
        .fetch_scrum_data(false)
        .await
        .context("스크럼 데이터 조회 실패")?;

    let today = days.iter().find(|d| d.label == "오늘");
    let tomorrow = days.iter().find(|d| d.label == "내일");

    let today_comment = today.and_then(|d| d.my_comment.as_ref());
    let tomorrow_key = tomorrow.map(|d| d.key.as_str());

    match (today_comment, tomorrow_key) {
        (Some(comment), Some(key)) if !key.is_empty() => {
            let adf = comment
                .build_tomorrow_adf()
                .context("오늘 코멘트에서 테이블을 찾을 수 없습니다")?;
            client.create_comment(key, &adf).await?;
            println!("내일 스크럼 코멘트를 작성했습니다. ({})", key);
        }
        (None, _) => anyhow::bail!("오늘 스크럼 코멘트가 없습니다"),
        _ => anyhow::bail!("내일 스크럼 이슈를 찾을 수 없습니다"),
    }

    Ok(())
}

pub async fn cmd_open(client: &AcliClient, host: &str, mode: &str) -> Result<()> {
    match mode {
        "sprint" => {
            let (sprint, _, _) = client
                .fetch_all_data(false)
                .await
                .context("스프린트 데이터 조회 실패")?;
            let url = format!("{}/secure/RapidBoard.jspa?rapidView={}", host, sprint.id);
            println!("Opening: {}", url);
            open::that(&url).context("브라우저 열기 실패")?;
        }
        "scrum" => {
            let (days, _) = client
                .fetch_scrum_data(false)
                .await
                .context("스크럼 데이터 조회 실패")?;
            let today = days.iter().find(|d| d.label == "오늘");
            match today {
                Some(day) if !day.key.is_empty() => {
                    let url = format!("{}/browse/{}", host, day.key);
                    println!("Opening: {}", url);
                    open::that(&url).context("브라우저 열기 실패")?;
                }
                _ => anyhow::bail!("오늘 스크럼 이슈를 찾을 수 없습니다"),
            }
        }
        other => anyhow::bail!("알 수 없는 모드: '{}' (sprint 또는 scrum)", other),
    }

    Ok(())
}
