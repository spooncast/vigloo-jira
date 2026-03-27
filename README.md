# vigloo-jira

Vigloo 팀을 위한 Jira TUI 도구. [Atlassian CLI (acli)](https://developer.atlassian.com/cloud/acli/) 기반.

## Features

### Sprint Mode (`1`)

현재 활성 스프린트에서 나에게 할당된 서브태스크가 있는 워크아이템을 조회.

- 왼쪽: 워크아이템 목록
- 오른쪽: 선택한 워크아이템의 내 서브태스크
- `Enter`: 서브태스크 → 브라우저에서 Jira 이슈 열기

### Scrum Mode (`2`)

Daily Scrum 에픽에서 어제/오늘/내일 스크럼 코멘트를 테이블 형식으로 조회.

- `←→`: 날짜 전환
- `↑↓`: 코멘트 스크롤
- `w` → `Enter`: 오늘의 "오늘 할 것"을 내일 스크럼에 "한 것(어제 한 것)"으로 자동 작성

### Caching

- 스프린트 메타데이터: 스프린트 종료일까지 캐시
- 스크럼 에픽 키: 분기 동안 캐시
- 전체 데이터: 5분 TTL 캐시 (재실행 시 즉시 로드)
- `r`: 캐시 무시하고 강제 새로고침

## Prerequisites

- [acli](https://developer.atlassian.com/cloud/acli/) 설치 및 인증 (`acli auth login`)
- Rust 1.70+

## Install

```bash
git clone <repo-url>
cd vigloo-jira
cargo build --release
```

바이너리: `target/release/vigloo-jira`

## Configuration

`~/.config/vigloo-jira/config.toml` (선택사항, 없으면 기본값 사용):

```toml
[jira]
board_id = 272
host = "https://spoonradio.atlassian.net"
project = "CLIP"
```

## Keybindings

| Key | Action |
|-----|--------|
| `1` / `2` | Sprint / Scrum 모드 전환 |
| `↑` `↓` | 항목 이동 / 스크롤 |
| `←` `→` | 스크럼 날짜 전환 |
| `Enter` | 선택 / 브라우저 열기 |
| `Esc` | 뒤로 |
| `Tab` | 패널 전환 (Sprint 모드) |
| `w` | 내일 스크럼 작성 (Scrum 모드) |
| `r` | 새로고침 |
| `q` | 종료 |

## Tech Stack

Rust, [ratatui](https://github.com/ratatui/ratatui), crossterm, tokio, serde, chrono
