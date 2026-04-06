use std::process::Command;

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

pub async fn cmd_write(client: &AcliClient, target: &str) -> Result<()> {
    let (days, _) = client
        .fetch_scrum_data(false)
        .await
        .context("스크럼 데이터 조회 실패")?;

    let (source_label, target_label, source_desc, target_desc) = match target {
        "tomorrow" => ("오늘", "내일", "오늘", "내일"),
        "today" => ("어제", "오늘", "어제", "오늘"),
        other => anyhow::bail!("알 수 없는 대상: '{}' (today 또는 tomorrow)", other),
    };

    let source_day = days.iter().find(|d| d.label == source_label);
    let target_day = days.iter().find(|d| d.label == target_label);

    let source_comment = source_day.and_then(|d| d.my_comment.as_ref());
    let target_key = target_day.map(|d| d.key.as_str());

    match (source_comment, target_key) {
        (Some(comment), Some(key)) if !key.is_empty() => {
            let adf = comment
                .build_tomorrow_adf()
                .context(format!("{} 코멘트에서 테이블을 찾을 수 없습니다", source_desc))?;
            client.create_comment(key, &adf).await?;
            println!("{} 스크럼 코멘트를 작성했습니다. ({})", target_desc, key);
        }
        (None, _) => anyhow::bail!("{} 스크럼 코멘트가 없습니다", source_desc),
        _ => anyhow::bail!("{} 스크럼 이슈를 찾을 수 없습니다", target_desc),
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

pub async fn cmd_update() -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    let repo = "spooncast/vigloo-jira";

    // Fetch latest version tag from GitHub API
    println!("최신 버전 확인 중...");
    let output = Command::new("curl")
        .args(["-fsSL", &format!("https://api.github.com/repos/{}/releases/latest", repo)])
        .output()
        .context("GitHub API 호출 실패")?;

    if !output.status.success() {
        anyhow::bail!("GitHub에서 최신 릴리스 정보를 가져올 수 없습니다");
    }

    let response: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("릴리스 정보 파싱 실패")?;
    let latest_tag = response["tag_name"]
        .as_str()
        .context("tag_name을 찾을 수 없습니다")?;
    let latest_version = latest_tag.strip_prefix('v').unwrap_or(latest_tag);

    if latest_version == current_version {
        println!("이미 최신 버전입니다. (v{})", current_version);
        return Ok(());
    }

    println!("v{} -> v{} 업데이트 중...", current_version, latest_version);

    // Detect platform asset name
    let asset = detect_asset_name()?;
    let download_url = format!("https://github.com/{}/releases/latest/download/{}", repo, asset);

    // Download to temp file
    let tmp_path = "/tmp/vj_update";
    let status = Command::new("curl")
        .args(["-fsSL", &download_url, "-o", tmp_path])
        .status()
        .context("바이너리 다운로드 실패")?;

    if !status.success() {
        anyhow::bail!("바이너리 다운로드 실패: {}", download_url);
    }

    // Make executable
    Command::new("chmod")
        .args(["+x", tmp_path])
        .status()
        .context("chmod 실패")?;

    // Replace current binary
    let current_exe = std::env::current_exe().context("현재 실행 파일 경로를 알 수 없습니다")?;
    let install_path = current_exe.to_string_lossy();

    let status = Command::new("sudo")
        .args(["mv", tmp_path, &install_path])
        .status()
        .context("바이너리 교체 실패 (sudo 권한 필요)")?;

    if !status.success() {
        anyhow::bail!("바이너리 교체 실패");
    }

    println!("v{} 업데이트 완료!", latest_version);
    Ok(())
}

fn detect_asset_name() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    match (os, arch) {
        ("macos", "aarch64") => Ok("vj-darwin-arm64".to_string()),
        ("macos", "x86_64") => Ok("vj-darwin-x86_64".to_string()),
        ("linux", "x86_64") => Ok("vj-linux-x86_64".to_string()),
        _ => anyhow::bail!("지원하지 않는 플랫폼: {} {}", os, arch),
    }
}
