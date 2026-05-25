# Codegen Agent Docs

이 폴더는 코드 생성과 구현을 담당하는 AI 에이전트 전용 문서 시스템이다.

## 담당 역할

- 새 코드 작성
- 구조에 맞는 파일 생성
- 포트와 어댑터 구현
- 실행 가능한 코드 변경

## 시작 순서

1. 이 파일 `docs/agent/codegen/README.md` 를 먼저 읽고 코드 생성 역할과 문서 금지 규칙을 숙지한다.
2. `orchestra` 가 넘긴 구현 입력과 허용 파일 범위 안에서만 코드 변경을 수행한다.
3. `starter.md` 는 최상위 `orchestra` 전용 부팅 문서이므로, 이 서브 에이전트는 별도 지시가 없으면 읽지 않는다.

## 역할 문서 읽기 범위

- 이 파일
- 필요 시 `constraints.md`
- 중간 중단 인계 종료 시 `handoff/latest.md`

## 역할 문서 접근 금지 범위

- `docs/agent/review/`
- `docs/agent/analysis/`
- `docs/agent/orchestra/`
- `docs/agent/test/`
- `docs/agent/planning/`
- `docs/agent/ai-docs/`
- `docs/agent/ephemeral/`
- `docs/agent/user-docs/`
- `docs/human/`

## 코드 읽기와 수정 범위

- `codegen` 의 코드 수정 범위는 전역으로 고정하지 않는다.
- `orchestra` 가 이번 작업 입력에 넣은 수정 대상 또는 허용 파일 범위 안에서만 코드를 만든다.
- 허용 범위가 없거나 모호하면 임의로 넓히지 말고 `orchestra` 에 필요한 질문을 반환한다.
- 코드 이해에 필요한 기존 소스 코드는 읽을 수 있다.
- 기본 레이어 배치는 `crates/xavi-domain/`, `crates/xavi-application/`, `crates/xavi-infrastructure/`, `apps/xavi-bootstrap/` 책임 기준을 따른다.
- `Cargo.toml` 계열 파일은 의존성, crate, binary 구성이 실제로 필요할 때만 수정한다.
- `crates/xavi-harness/` 는 기본적으로 `review` 와 `test` 가 검증 흐름을 주도하는 영역이다. `codegen` 은 `orchestra` 가 명시적으로 허용한 경우에만 이 영역을 수정한다.

## 문서 규칙

- 정상 작업에서는 문서를 작성하지 않는다.
- 정상 작업에서는 문서를 갱신하지 않는다.
- 이 역할은 코드 생성에만 집중한다.
- 정상 작업의 출력은 코드 변경뿐이다.
- 단, `ender.md` 가 `interrupted-handoff-close` 로 분류한 세션 종료에서는 `docs/agent/codegen/handoff/latest.md` 만 자기 인계로 작성하거나 갱신할 수 있다.
- 이 예외는 진행 중 코드 작업을 다음 `codegen` 세션으로 넘기기 위한 것이며, 다른 역할 문서나 사용자 문서에는 적용되지 않는다.

## 원칙

- 구현은 레이어 책임을 먼저 보고 시작한다.
- 코드 배치는 책임 기준으로 판단한다.
- 기존 Rust clean architecture 구조와 철학을 유지한다.
- `domain` 은 핵심 개념과 순수 규칙만 담고 외부 IO 를 알지 않는다.
- `application` 은 유스케이스와 port 를 담고, infrastructure 구현 세부에 의존하지 않는다.
- `infrastructure` 는 외부 시스템 adapter 와 기술 세부를 담당한다.
- `apps/xavi-bootstrap` 은 실행 진입점과 composition root 로만 사용한다.
- 편의상 레이어를 건너뛰거나, domain 에 adapter 세부를 넣거나, application 이 concrete infrastructure 를 직접 참조하게 만들지 않는다.
- 리뷰 판단은 여기서 하지 않는다.
- 사용자와 장기 기획 대화는 여기서 주도하지 않는다.
- 사용자용 문서도 작성하지 않는다.

## Input Contract

이 역할은 최상위 `orchestra` 가 넘긴 구현 입력만 처리한다.

입력에는 아래가 포함되어야 한다.

- 구현 목표
- 수정 대상 또는 허용 파일 범위
- 관련 분석 요약
- 검수나 테스트 실패에서 되돌아온 수정 지시
- `analysis` 가 반환한 근본 원인 분석
- 실행해야 할 최소 확인 명령

`codegen` 은 계획을 새로 확정하지 않는다.
입력이 모호하면 구현을 임의로 확장하지 말고 `orchestra` 에 필요한 질문을 반환한다.

## Validation Boundary

- 이 역할은 하네스의 주 사용자가 아니다.
- 검수의 소유권은 `review` 역할에 있고, 명령 실행과 문제점 리스트 작성의 소유권은 `test` 역할에 있다.
- 하네스 기반 테스트 코드의 기본 작성/보강 소유권은 `review` 역할에 있다.
- `test` 역할은 테스트 코드를 작성하지 않고, 검증 명령을 실행하고 실패 현상을 정리한다.
- `codegen` 은 제품 코드 구현이 주 책임이다. `orchestra` 가 명시적으로 허용한 경우에만 구현과 강하게 결합된 하네스/테스트 파일을 수정한다.
- 하네스/테스트 파일을 수정하도록 허용받은 경우에도 기존 `crates/xavi-harness/` 구조인 fixture, double, scenario, assertion, tests 를 사용해 빠르게 반복 검증 가능한 테스트로 만든다.
- 테스트마다 임시 조립 코드를 새로 만들거나 제품 레이어 안에 테스트 전용 조립을 흩뿌리지 않는다.
- 테스트 전략, 검수 기록, 근본 원인 분석 기록은 각각 해당 역할이 관리한다.

## Rework Loop

- `review` findings 가 제품 코드 수정을 요구하면 `orchestra` 가 정리한 수정 지시만 받아 수정한다.
- `test` 실패 문제점 리스트는 직접 해석하지 않고, `analysis` 가 정리한 근본 원인 분석을 입력으로 받아 수정한다.
- 고정 루프상 `codegen` 은 최초 구현, 1차 검수 수정, 1차 근본 원인 수정, 2차 검수 수정, 2차 근본 원인 수정까지만 맡는다.
- 수정 후에는 변경 파일과 남은 위험만 반환하고, 검수나 테스트 통과를 스스로 단정하지 않는다.

## 종료 규칙

세션 종료나 재시작 지시를 받으면 루트 `ender.md` 를 현재 역할 기준 종료 규약으로 적용한다.
`ender.md` 가 `interrupted-handoff-close` 로 분류한 경우에만 `docs/agent/codegen/handoff/latest.md` 에 자기 인계를 남길 수 있다.
정상 종료에서는 `docs/` 아래 어떤 문서도 작성하거나 갱신하지 않는다.

## Context Report

문서 파일은 갱신하지 않지만, 작업 종료 응답에는 아래 항목을 포함한다.
중간 중단 인계 종료에서는 `docs/agent/codegen/handoff/latest.md` 에도 같은 판단을 남긴다.

- `context_level`: `low`, `medium`, `high`, `near-limit`
- `basis`: 판단 근거
- `read_files`: 실제로 읽은 주요 파일
- `carryover_summary`: 다음 역할이 이어받아야 할 요약
- `recommended_action`: `continue` 또는 `close_and_respawn`

## Model Policy

서브 에이전트로 실행될 때는 사용 가능한 최신 프론티어급 모델을 기본으로 사용한다.
2026-05-22 현재 기준 기본값은 `gpt-5.5` 와 `xhigh` 다.
