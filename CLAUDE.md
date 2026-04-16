# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 프로젝트 개요

Vigloo-Jira (`vj`)는 Jira와 연동하는 Rust TUI 애플리케이션이다. Sprint 모드(활성 스프린트 작업 항목 및 서브태스크 조회)와 Scrum 모드(일일 스크럼 코멘트 조회/작성) 두 가지 모드를 제공한다. Ratatui + Crossterm + Tokio 기반.

## 빌드 및 실행

```bash
cargo build                    # 디버그 빌드
cargo build --release          # 릴리스 빌드
cargo run                      # 디버그 모드 실행
```

크로스 컴파일 타겟: `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`.

테스트 스위트나 린터는 현재 설정되어 있지 않다.

## 아키텍처

**진입점:** `src/main.rs` — 터미널 설정(raw mode, alternate screen), 비동기 채널 생성, 이벤트 루프 실행.

**핵심 모듈:**
- `app.rs` — 모든 UI 상태를 관리하는 `App` 구조체 (모드, 선택 인덱스, 패널, 로딩/에러 상태). `Sprint`과 `Scrum` 두 가지 모드.
- `ui.rs` — Ratatui 렌더링. 모드별 다른 레이아웃 (Sprint: 분할 패널, Scrum: 테이블 뷰).
- `acli.rs` — Atlassian CLI (`acli`) 바이너리를 통한 Jira 연동. `acli` 서브프로세스를 실행하고 JSON 출력을 파싱. 직접 HTTP 호출 없음.
- `model.rs` — 도메인 타입 (`Sprint`, `WorkItem`, `Subtask`, `ScrumDay`, `ScrumComment`, `ScrumTable`). 스크럼 코멘트를 위한 ADF(Atlassian Document Format) 파싱/빌드 포함.
- `config.rs` — `~/.config/vigloo-jira/config.toml`에서 TOML 설정 로드 (board_id, host, project).
- `cache.rs` — `~/.cache/vigloo-jira/`에 TTL 기반 파일 캐싱. 스프린트 메타데이터는 스프린트 종료일까지, 데이터는 5분 TTL.
- `event.rs` — 키보드 입력을 앱 액션으로 매핑 (Quit, Refresh, SwitchMode, OpenLink, WriteScrum).

**비동기 패턴:** 데이터 페칭은 백그라운드 Tokio 태스크에서 실행되며, `mpsc::unbounded_channel`을 통해 메인 스레드로 결과를 전송. UI 렌더링은 메인 스레드에서 블로킹 없이 수행.

## 사전 요구사항

`acli` (Atlassian CLI) 바이너리가 설치 및 인증되어 있어야 `vj`가 동작한다. 설치 스크립트에서 이를 확인한다.

## 주요 설계 결정

- 모든 Jira API 접근은 직접 REST가 아닌 `acli` 서브프로세스 호출을 통해 수행 — 인증 토큰 관리를 회피.
- ADF(Atlassian Document Format)는 `model.rs`에서 직접 파싱하여 스크럼 테이블을 추출하고 새 코멘트 본문을 구성.
- 병렬 API 호출은 `tokio::join!` / `tokio::try_join!`을 사용하여 성능 최적화.
- 바이너리 이름은 빠른 터미널 접근을 위해 `vj`로 설정.
