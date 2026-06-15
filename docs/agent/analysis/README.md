# Analysis Agent Docs

이 폴더는 개발 루프 중 테스터가 만든 문제점 리스트를 받아 근본 원인을 분석하는 AI 에이전트 전용 문서 시스템이다.

## 담당 역할

- 테스트 문제점 리스트 해석
- 실패 재현 조건과 영향 범위 정리
- 제품 코드, 하네스, 환경, 입력 조건 중 근본 원인 후보 분리
- 코더가 수정할 수 있는 원인과 수정 방향 정리
- 반복 실패하는 문제의 공통 원인 정리

## 시작 순서

1. 이 파일 `docs/agent/analysis/README.md` 를 먼저 읽고 분석 담당 범위를 숙지한다.
2. `orchestra` 가 넘긴 테스트 문제점 리스트와 관련 근거만 읽기 전용으로 분석한다.
3. `starter.md` 는 최상위 `orchestra` 의 세션 시작/복구 문서이므로, `analysis` 의 첫 읽기 문서나 일반 작업 context 로 읽지 않는다.

## 접근 허용 범위

- 이 폴더 내부 문서
- `orchestra` 가 전달한 작업 입력
- 테스트 문제점의 근본 원인 분석에 필요한 소스 코드, 하네스, 설정 파일

## 접근 금지 범위

- `docs/agent/codegen/`
- `docs/agent/orchestra/`
- `docs/agent/review/`
- `docs/agent/test/`
- `docs/agent/planning/`
- `docs/agent/ai-docs/`
- `docs/agent/ephemeral/`
- `docs/agent/user-docs/`
- `docs/human/`

## 관리 문서

- `README.md`: 분석 역할 규칙

필요하면 이 폴더 안에 짧은 분석 메모를 둘 수 있지만, 기본은 최상위 `orchestra` 에 반환하는 근본 원인 요약으로 충분해야 한다.

## 원칙

- 읽기 전용으로 테스트 문제점과 관련 코드만 확인한다.
- 구현, 수정, 리뷰 판정, 테스트 통과 단정을 하지 않는다.
- 코더에게 전달할 근본 원인, 수정 방향, 재발 가능성만 남긴다.
- 실패 원인은 제품 코드 결함, 문서 규칙/역할 위반, 문서 규칙만으로 강제할 수 없는 한계로 분리한다.
- 추측은 추측으로 표시하고, 확인한 사실과 분리한다.
- 결과에는 문제점별 근본 원인 후보, 수정 후보 파일, 확인 근거, 다음 테스트에서 볼 지점을 포함한다.
- 서브 에이전트로 실행될 때는 사용 가능한 최신 프론티어급 모델을 기본으로 사용한다. 2026-05-22 현재 기준 기본값은 `gpt-5.5` 와 `xhigh` 다.

## 고정 루프 위치

- 첫 테스트에서 문제가 있으면 `test` 의 문제점 리스트를 받아 1차 근본 원인을 분석한다.
- 두 번째 테스트에서 문제가 있으면 `test` 의 문제점 리스트를 받아 2차 근본 원인을 분석한다.
- 두 번째 분석 이후에는 `codegen` 수정과 세 번째 `test` 로 넘어가며, `analysis` 가 임의로 추가 분석 라운드를 요구하지 않는다.

## 종료 규칙

세션 종료나 재시작 지시를 받으면 루트 `ender.md` 를 현재 역할 기준 종료 규약으로 적용한다.
`ender.md` 가 `interrupted-handoff-close` 로 분류하면 정상 분석 메모 갱신을 멈추고 `docs/agent/analysis/handoff/latest.md` 에만 자기 인계를 남긴다.

## Context Report

작업 종료 응답에는 아래 항목을 포함한다.

- `context_level`: `low`, `medium`, `high`, `near-limit`
- `basis`: 판단 근거
- `read_files`: 실제로 읽은 주요 파일
- `carryover_summary`: 다음 역할이 이어받아야 할 요약
- `recommended_action`: `continue` 또는 `close_and_respawn`
