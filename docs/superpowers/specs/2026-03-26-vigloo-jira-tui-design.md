# vigloo-jira TUI Design Spec

## Overview

Rust + Ratatui 기반 TUI 도구. Atlassian CLI(acli)를 통해 Vigloo 팀의 Jira 보드에서 활성 스프린트를 조회하고, 현재 사용자에게 할당된 워크아이템과 서브태스크를 좌우 분할 패널로 보여준다.

**MVP 범위:** 읽기 전용 (조회만)

## Data Flow

1. `acli jira board list-sprints --id {board_id} --json` → active 스프린트 필터링
2. `acli jira sprint list-workitems --sprint {sprint_id} --board {board_id} --json --jql "assignee = currentUser()"` → 내 워크아이템
3. 각 워크아이템별 `acli jira workitem search --jql "parent = {key}" --json` → 서브태스크 (tokio 병렬 호출)

**참고:** `list-sprints` 응답은 `{ "sprints": [...] }`, `list-workitems`와 `workitem search` 응답은 `{ "issues": [...] }` 래퍼 구조이다. 파싱 시 래퍼를 벗기고 내부 배열을 사용한다.

## UI Layout

좌우 분할 패널:

```
┌──────────────────────────────────────────────────────────┐
│ 🏃 Vigloo (1.6.10) - 3/25~4/8  |  내 워크아이템: 3개     │
├─────────────────────────┬────────────────────────────────┤
│ WORK ITEMS              │ SUBTASKS (CLIP-7619)           │
│                         │                                │
│ ▸ CLIP-7619 보상타이머..│ [해야 할 일] CLIP-8746 Design  │
│   CLIP-7622 콜드 스타트..│ [진행 중]   CLIP-8747 Draft   │
│   CLIP-7630 저가 옵션.. │ [해야 할 일] CLIP-8748 API    │
│                         │                                │
├─────────────────────────┴────────────────────────────────┤
│ ←→: 패널 전환  ↑↓: 이동  r: 새로고침  q: 종료            │
└──────────────────────────────────────────────────────────┘
```

## Architecture

```
src/
├── main.rs    — 엔트리포인트, 터미널 초기화, 메인 루프
├── app.rs     — App 상태 (스프린트, 워크아이템, 선택 인덱스, 패널 포커스)
├── ui.rs      — Ratatui 렌더링 (좌우 분할 레이아웃)
├── event.rs   — 키보드 이벤트 핸들링
├── acli.rs    — acli CLI 실행 + JSON 파싱 (외부 의존 격리)
└── model.rs   — Sprint, WorkItem, Subtask 데이터 모델
```

### Module Responsibilities

| 모듈 | 책임 |
|------|------|
| `main.rs` | 터미널 설정, App 초기화, 이벤트 루프 실행 |
| `app.rs` | 앱 상태 관리: 현재 스프린트, 워크아이템 목록, 선택 인덱스, 좌/우 패널 포커스 |
| `ui.rs` | `Layout::horizontal`로 좌우 분할, 워크아이템 리스트 + 서브태스크 리스트 렌더링 |
| `event.rs` | 키 입력 → App 상태 변경 매핑 |
| `acli.rs` | `tokio::process::Command`로 acli 실행, `serde_json`으로 파싱. 모든 외부 의존이 여기에 격리 |
| `model.rs` | `Sprint`, `WorkItem`, `Subtask` 구조체. acli JSON → 도메인 모델 변환 |

## Data Models

```rust
struct Sprint {
    id: u64,
    name: String,
    state: String,       // "active", "closed", "future"
    start_date: String,
    end_date: String,
    goal: String,
}

struct WorkItem {
    key: String,          // "CLIP-7619"
    summary: String,
    status: String,       // "해야 할 일", "진행 중", "검토 중", "완료"
    issue_type: String,   // "작업", "스토리", "버그"
    assignee: String,
    priority: String,
    subtasks: Vec<Subtask>,
}

struct Subtask {
    key: String,
    summary: String,
    status: String,
    assignee: String,
    priority: String,
}
```

## Configuration

TOML 설정 파일: `~/.config/vigloo-jira/config.toml`

```toml
[jira]
board_id = 272
```

- 코드에 기본값 하드코딩 (`board_id = 272`)
- 설정 파일이 존재하면 해당 값으로 오버라이드
- 설정 파일 없어도 정상 동작

의존성: `toml`, `dirs` 크레이트

## Keybindings

| 키 | 동작 |
|---|------|
| `↑` / `↓` | 현재 패널에서 항목 이동 |
| `Tab` | 좌↔우 패널 포커스 전환 |
| `r` | 데이터 새로고침 |
| `q` | 종료 |

## Status Colors

| 상태 | 색상 |
|------|------|
| 해야 할 일 | 회색 (Gray) |
| 진행 중 | 노랑 (Yellow) |
| 검토 중 | 파랑 (Blue) |
| 완료 | 초록 (Green) |

## Dependencies

| 크레이트 | 용도 |
|---------|------|
| `ratatui` | TUI 프레임워크 |
| `crossterm` | 터미널 제어 (백엔드) |
| `tokio` | 비동기 런타임 (acli 병렬 호출) |
| `serde` + `serde_json` | JSON 역직렬화 |
| `toml` | 설정 파일 파싱 |
| `dirs` | XDG 경로 탐색 (`~/.config/`) |

## Future Extensions

MVP 이후 추가 가능한 기능들 (이 스펙 범위 밖):
- 서브태스크 상태 전환
- 코멘트 조회/추가
- 워크로그 관리
- 다중 보드 지원
