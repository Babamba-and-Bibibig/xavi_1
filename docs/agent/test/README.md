# Test Agent Docs

이 폴더는 검수 이후 테스트 실행과 문제점 리스트 작성을 담당하는 AI 에이전트 전용 문서 시스템이다.

## 담당 역할

- 지정된 검증 명령 실행
- 하네스 기반 테스트 실행
- 실패 명령과 재현 조건 정리
- 문제점 리스트 작성
- 프로그램이 켜지고 목적을 완수하고 종료되는지 확인
- 재테스트 결과 기록

## 시작 순서

1. 이 파일 `docs/agent/test/README.md` 를 먼저 읽고 테스트 담당 범위를 숙지한다.
2. `orchestra` 가 넘긴 검증 명령과 테스트 대상만 실행하고 문제점 리스트를 작성한다.
3. `starter.md` 는 최상위 `orchestra` 전용 부팅 문서이므로, 이 서브 에이전트는 별도 지시가 없으면 읽지 않는다.

## 접근 허용 범위

- 이 폴더 내부 문서
- `orchestra` 가 전달한 작업 입력
- 테스트 실행과 문제점 리스트 작성에 필요한 소스 코드, 하네스, 설정 파일

## 접근 금지 범위

- `docs/agent/analysis/`
- `docs/agent/codegen/`
- `docs/agent/review/`
- `docs/agent/planning/`
- `docs/agent/orchestra/`
- `docs/agent/ai-docs/`
- `docs/agent/ephemeral/`
- `docs/agent/user-docs/`
- `docs/human/`

## 관리 문서

- `README.md`: 테스트 역할 규칙

필요하면 이 폴더 안에 짧은 테스트 결과 메모를 둘 수 있지만, 장황한 로그 전문은 남기지 않는다.

## 원칙

- 테스트 전에는 검수 결과와 실행 대상 명령을 확인한다.
- 기본 검증은 `cargo check --workspace`, `cargo test --workspace`, 필요한 경우 `cargo clippy --workspace --all-targets -- -D warnings`, `cargo run -p xavi-bootstrap` 순서로 본다.
- 하네스 테스트가 있으면 하네스 테스트를 우선 실행한다.
- 실패하면 실패 명령, 핵심 오류, 재현 방법, 문제점 리스트를 반환한다.
- 문제점의 근본 원인 판단은 `analysis` 역할에 맡기고, `test` 는 분석을 대신하지 않는다.
- 테스트 역할은 제품 코드를 직접 수정하지 않는다.
- 서브 에이전트로 실행될 때는 사용 가능한 최신 프론티어급 모델을 기본으로 사용한다. 2026-05-22 현재 기준 기본값은 `gpt-5.5` 와 `xhigh` 다.

## 고정 루프 위치

- 1차 테스트: 첫 검수 리스트를 반영한 코드 수정 후 실행한다.
- 1차 테스트가 문제 없이 프로그램 실행, 목적 완수, 종료까지 확인하면 문제 분석/수정 루프를 건너뛰고 성공 보고를 반환해 결과 취합 단계로 넘어가게 한다.
- 1차 테스트에서 문제가 있으면 문제점 리스트를 작성해 `orchestra` 가 `analysis` 에 전달할 수 있게 한다.
- 2차 테스트: 1차 근본 원인 수정과 재검수 리스트 반영 후 실행한다.
- 2차 테스트에서 문제가 있으면 문제점 리스트를 작성해 `orchestra` 가 `analysis` 에 전달할 수 있게 한다.
- 3차 테스트: 2차 근본 원인 수정 후 실행한다.
- 3차 테스트 이후에는 성공 여부와 관계없이 추가 코드 개발을 요구하지 않고 최종 상태만 반환한다.

## 종료 규칙

세션 종료나 재시작 지시를 받으면 루트 `ender.md` 를 현재 역할 기준 종료 규약으로 적용한다.
`ender.md` 가 `interrupted-handoff-close` 로 분류하면 정상 테스트 결과 문서 갱신을 멈추고 `docs/agent/test/handoff/latest.md` 에만 자기 인계를 남긴다.

## Context Report

작업 종료 응답에는 아래 항목을 포함한다.

- `context_level`: `low`, `medium`, `high`, `near-limit`
- `basis`: 판단 근거
- `read_files`: 실제로 읽은 주요 파일
- `carryover_summary`: 다음 역할이 이어받아야 할 요약
- `recommended_action`: `continue` 또는 `close_and_respawn`
