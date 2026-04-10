# CLI Spinner Progress Indicator

## Problem

CLI 서브커맨드(`vj sprint`, `vj scrum`, `vj write`) 실행 시 데이터 페칭 중 아무런 피드백이 없어 사용자가 프로그램이 멈춘 것으로 오해할 수 있다.

## Solution

`indicatif` 크레이트를 사용하여 CLI 모드에서 데이터 페칭 중 stderr에 스피너 애니메이션을 표시한다.

## Design

### Scope

**변경 대상:**
- `Cargo.toml` — `indicatif` 의존성 추가
- `src/cli.rs` — `cmd_sprint`, `cmd_scrum`, `cmd_write` 함수에 스피너 적용

**변경하지 않는 것:**
- TUI 모드 로딩 표시 (기존 ratatui 기반 유지)
- `--json` 출력 형식
- 캐시 로직, API 호출 구조
- `cmd_open`, `cmd_update` (API 호출 없음)

### Spinner Behavior

```
$ vj sprint
⠋ Loading sprint data...    ← stderr, animated
(완료 후 스피너 줄 제거, 결과 stdout 출력)
```

- **라이브러리**: `indicatif`
- **스타일**: 기본 dots 스피너
- **출력 대상**: stderr (`DrawTarget::stderr()`)
- **완료 시**: `finish_and_clear()` — 스피너 줄 제거
- **에러 시**: `finish_and_clear()` 후 기존 에러 처리 유지

### Messages per Command

| Command | Spinner Message |
|---------|----------------|
| `sprint` | `Loading sprint data...` |
| `scrum` | `Loading scrum data...` |
| `write` | `Writing scrum comment...` |

### Implementation Pattern

```rust
use indicatif::{ProgressBar, ProgressStyle};

let spinner = ProgressBar::new_spinner()
    .with_style(ProgressStyle::default_spinner())
    .with_message("Loading sprint data...");
spinner.enable_steady_tick(std::time::Duration::from_millis(80));

// ... API calls ...

spinner.finish_and_clear();
// print results to stdout
```
