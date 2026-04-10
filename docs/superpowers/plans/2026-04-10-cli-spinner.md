# CLI Spinner Progress Indicator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** CLI 서브커맨드 실행 시 데이터 페칭 중 stderr에 스피너를 표시하여 사용자에게 진행 상태를 알린다.

**Architecture:** `indicatif` 크레이트를 `cli.rs`의 3개 커맨드 함수(`cmd_sprint`, `cmd_scrum`, `cmd_write`)에 적용. 각 함수에서 API 호출 전 스피너를 시작하고, 완료(성공/에러) 시 `finish_and_clear()`로 정리.

**Tech Stack:** Rust, indicatif, tokio (기존)

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `Cargo.toml` | Modify | `indicatif` 의존성 추가 |
| `src/cli.rs` | Modify | 3개 커맨드 함수에 스피너 로직 적용 |

---

### Task 1: Add `indicatif` dependency

**Files:**
- Modify: `Cargo.toml:21` (dependencies 끝)

- [ ] **Step 1: Add indicatif to Cargo.toml**

`Cargo.toml`의 `[dependencies]` 섹션 끝에 추가:

```toml
indicatif = "0.17"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: 컴파일 성공, indicatif 다운로드 및 resolve

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "feat: indicatif 의존성 추가"
```

---

### Task 2: Add spinner to `cmd_sprint`

**Files:**
- Modify: `src/cli.rs:1-12` (imports 및 cmd_sprint 함수 시작부)

- [ ] **Step 1: Add indicatif import**

`src/cli.rs` 상단에 import 추가:

```rust
use indicatif::ProgressBar;
```

- [ ] **Step 2: Add spinner to cmd_sprint**

`cmd_sprint` 함수에서 `fetch_all_data` 호출 전에 스피너를 시작하고, 호출 후 정리한다. 기존 코드:

```rust
pub async fn cmd_sprint(client: &AcliClient, _host: &str, json: bool) -> Result<()> {
    let (sprint, work_items, warnings) = client
        .fetch_all_data(false)
        .await
        .context("스프린트 데이터 조회 실패")?;
```

변경 후:

```rust
pub async fn cmd_sprint(client: &AcliClient, _host: &str, json: bool) -> Result<()> {
    let spinner = ProgressBar::new_spinner().with_message("Loading sprint data...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let result = client.fetch_all_data(false).await;
    spinner.finish_and_clear();

    let (sprint, work_items, warnings) = result.context("스프린트 데이터 조회 실패")?;
```

핵심: `result`를 먼저 받고, `spinner.finish_and_clear()` 후에 `?`로 에러를 전파한다. 이렇게 하면 에러 시에도 스피너가 정리된다.

- [ ] **Step 3: Verify it compiles and runs**

Run: `cargo check`
Expected: 컴파일 성공

Run: `cargo run -- sprint`
Expected: 스피너가 stderr에 표시되고, 완료 후 스프린트 데이터 출력

- [ ] **Step 4: Commit**

```bash
git add src/cli.rs
git commit -m "feat: cmd_sprint에 스피너 진행 표시 추가"
```

---

### Task 3: Add spinner to `cmd_scrum`

**Files:**
- Modify: `src/cli.rs:66-70` (cmd_scrum 함수 시작부)

- [ ] **Step 1: Add spinner to cmd_scrum**

기존 코드:

```rust
pub async fn cmd_scrum(client: &AcliClient, json: bool) -> Result<()> {
    let (days, warnings) = client
        .fetch_scrum_data(false)
        .await
        .context("스크럼 데이터 조회 실패")?;
```

변경 후:

```rust
pub async fn cmd_scrum(client: &AcliClient, json: bool) -> Result<()> {
    let spinner = ProgressBar::new_spinner().with_message("Loading scrum data...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let result = client.fetch_scrum_data(false).await;
    spinner.finish_and_clear();

    let (days, warnings) = result.context("스크럼 데이터 조회 실패")?;
```

- [ ] **Step 2: Verify it compiles and runs**

Run: `cargo check`
Expected: 컴파일 성공

Run: `cargo run -- scrum`
Expected: 스피너가 stderr에 표시되고, 완료 후 스크럼 데이터 출력

- [ ] **Step 3: Commit**

```bash
git add src/cli.rs
git commit -m "feat: cmd_scrum에 스피너 진행 표시 추가"
```

---

### Task 4: Add spinner to `cmd_write`

**Files:**
- Modify: `src/cli.rs:112-116` (cmd_write 함수 시작부)

- [ ] **Step 1: Add spinner to cmd_write**

`cmd_write`는 두 단계가 있다: (1) 스크럼 데이터 fetch, (2) 코멘트 작성. 전체를 하나의 스피너로 감싼다.

기존 코드:

```rust
pub async fn cmd_write(client: &AcliClient, target: &str) -> Result<()> {
    let (days, _) = client
        .fetch_scrum_data(false)
        .await
        .context("스크럼 데이터 조회 실패")?;
```

변경 후:

```rust
pub async fn cmd_write(client: &AcliClient, target: &str) -> Result<()> {
    let spinner = ProgressBar::new_spinner().with_message("Writing scrum comment...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let result = client.fetch_scrum_data(false).await;
    spinner.finish_and_clear();

    let (days, _) = result.context("스크럼 데이터 조회 실패")?;
```

Note: `create_comment` 호출(line 135)은 스피너 없이 둔다. 데이터 fetch가 대부분의 대기 시간이고, 코멘트 작성은 빠르다.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: 컴파일 성공

- [ ] **Step 3: Commit**

```bash
git add src/cli.rs
git commit -m "feat: cmd_write에 스피너 진행 표시 추가"
```
