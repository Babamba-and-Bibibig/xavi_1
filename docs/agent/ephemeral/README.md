# Ephemeral Agent Docs

이 폴더는 필요에 따라 임시로 생성되는 서브 AI 에이전트용 문서 시스템이다.

## 담당 역할

- 특정 주제만 빠르게 조사
- 국소 코드 탐색
- 보조 검증
- 일시적인 토론 또는 정리

## 시작 순서

1. 이 파일 `docs/agent/ephemeral/README.md` 를 먼저 읽고 임시 세션 운영 범위를 숙지한다.
2. 자기에게 배정된 `sessions/<session-id>/` 범위 안에서만 임시 작업을 수행한다.
3. `starter.md` 는 최상위 `orchestra` 전용 부팅 문서이므로, 이 서브 에이전트는 별도 지시가 없으면 읽지 않는다.

## 접근 허용 범위

- 이 폴더 내부에서 자신에게 배정된 세션 문서
- `orchestra` 가 전달한 작업 입력

## 접근 금지 범위

- `docs/agent/codegen/`
- `docs/agent/analysis/`
- `docs/agent/review/`
- `docs/agent/test/`
- `docs/agent/planning/`
- `docs/agent/orchestra/`
- `docs/agent/ai-docs/`
- `docs/agent/user-docs/`
- `docs/agent/ephemeral/` 내 다른 임시 세션 문서
- `docs/human/`

## 관리 방식

- 임시 에이전트마다 개별 세션 폴더를 만든다.
- 세션 폴더는 `sessions/<session-id>/` 형식으로 만든다.
- 작업 종료 후 결과만 남기고 세션을 닫는다.
- 자기 세션에서 한 일과 남은 일을 스스로 문서화한다.

## 기본 문서

- `SESSION_TEMPLATE.md`: 새 임시 세션 생성 템플릿
- `RESULT_TEMPLATE.md`: 세션 종료 시 반환 형식 템플릿
- `sessions/README.md`: 세션 폴더 운영 규칙

## 원칙

- 왜 실행되었는지 적는다.
- 무엇을 했는지 적는다.
- 무엇이 남았는지 적는다.
- 다른 역할 문서는 건드리지 않는다.
- 종료 후 불필요한 탐색 메모는 정리하고 반환에 필요한 내용만 남긴다.
- 서브 에이전트로 실행될 때는 사용 가능한 최신 프론티어급 모델을 기본으로 사용한다. 2026-05-22 현재 기준 기본값은 `gpt-5.5` 와 `xhigh` 다.

## Context Report

작업 종료 응답에는 아래 항목을 포함한다.

- `context_level`: `low`, `medium`, `high`, `near-limit`
- `basis`: 판단 근거
- `read_files`: 실제로 읽은 주요 파일
- `carryover_summary`: 다음 역할이 이어받아야 할 요약
- `recommended_action`: `continue` 또는 `close_and_respawn`
