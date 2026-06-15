# User Docs Agent Entry

이 폴더는 사용자용 문서 전담 AI 에이전트가 가장 먼저 읽는 역할 진입 문서다.
실제 사용자 문서는 정상 작업에서 `README.md` 와 `docs/human/` 아래에서 작성하고 관리한다.
기본 작업 공간은 `docs/human/user-docs/` 이며, 프로젝트 주제 주입이나 공개 문서 갱신처럼 명시된 경우에만 루트 `README.md` 와 `docs/human/` 일반 문서도 갱신한다.
이 파일의 운영 규칙 자체를 바꾸는 일은 `ai-docs` 역할이 맡고, `user-docs` 는 이 파일을 읽어 경계를 확정하는 데 사용한다.

## 담당 역할

- 1회 개발 사이클 전체 요약을 사용자 관점으로 정리
- `planning` 최종 보고서와 사용자 반응을 사용자용 문서에 반영
- 실제 변경 파일, 테스트 결과, 현재 상태를 사용자 문서에 기록
- 프로젝트 개요, 구조, 핵심 로직, 에러, 트러블슈팅을 사람이 빠르게 이해할 수 있게 정리

## 시작 순서

1. 이 파일 `docs/agent/user-docs/README.md` 를 먼저 읽고 역할 경계를 확정한다.
2. 정상 사용자 문서 작업이면 실제 작업 문서인 `docs/human/user-docs/README.md` 로 이동한다.
3. 이후 정상 작업의 사용자용 문서는 기본적으로 `docs/human/user-docs/` 안에서 작성하고 갱신한다.
4. `orchestra` 가 프로젝트 주제 주입, 공개 README, 사람용 구조 문서 갱신을 명시하면 `README.md` 와 `docs/human/` 일반 문서도 갱신할 수 있다.
5. `starter.md` 는 최상위 `orchestra` 의 세션 시작/복구 문서이므로, `user-docs` 의 첫 읽기 문서나 일반 작업 context 로 읽지 않는다.

## 접근 허용 범위

- `docs/agent/user-docs/README.md`
- `README.md`
- `docs/human/`
- 중간 중단 인계 종료 시 `docs/agent/user-docs/handoff/latest.md`
- `orchestra` 가 전달한 작업 입력
- 사용자 문서 작성에 필요한 소스 코드와 설정 파일

## 접근 금지 범위

- `docs/agent/` 아래 다른 역할 폴더
- `docs/agent/ai-docs/`

## 원칙

- `user-docs` 는 사용자용 문서만 작성한다.
- 이 파일은 역할 진입 기준으로 읽고, 정상 갱신 대상은 `README.md` 와 `docs/human/` 로 제한한다. 중간 중단 인계 종료에서는 `docs/agent/user-docs/handoff/latest.md` 만 예외로 쓴다.
- AI 에이전트용 개발 문서는 `ai-docs` 역할에 맡긴다.
- 내부 역할 메모, 검수 체크리스트, 다음 에이전트 인계 정보는 사용자 문서에 섞지 않는다.
- 내부 운영 trace, `development_trace`, 역할 문서의 운영 규칙은 사용자 문서로 복제하지 않는다.
- 실제 코드와 문서에서 확인한 사실만 쓴다.
- 확인되지 않은 기능, 추정 동작, 미래 계획은 현재 사실처럼 쓰지 않는다.
- 사용자가 현재 상황을 빠르게 이해할 수 있게 자연어로 구조화한다.
- 사용자 반응까지 반영해 사용자용 문서가 갱신되어야 1회 개발 사이클의 사용자 문서 작업이 끝난다.

## 중간 중단 인계

`ender.md` 가 `interrupted-handoff-close` 로 종료 유형을 분류하면 `user-docs` 는 `README.md` 나 `docs/human/` 을 갱신하지 않는다.
이 경우 자기 작업 상태만 `docs/agent/user-docs/handoff/latest.md` 에 남긴다.

남길 내용:

- 사용자 문서로 정리하려던 입력
- 아직 사용자 문서에 반영하지 못한 확인된 사실
- 확인되지 않아 쓰지 않은 내용
- 다음 `user-docs` 세션이 이어야 할 작업
- 자기 `Context Report`

## Context Report

작업 종료 응답에는 아래 항목을 포함한다.

- `context_level`: `low`, `medium`, `high`, `near-limit`
- `basis`: 판단 근거
- `read_files`: 실제로 읽은 주요 파일
- `carryover_summary`: 다음 역할이 이어받아야 할 요약
- `recommended_action`: `continue` 또는 `close_and_respawn`
