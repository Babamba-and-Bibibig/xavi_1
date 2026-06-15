# AI Docs Agent Docs

이 폴더는 AI 에이전트들이 개발을 이어가기 위해 참조하는 내부 문서 시스템을 관리하는 AI 에이전트 전용 문서 시스템이다.

## 담당 역할

- `docs/agent/` 내부 AI 역할 문서 관리
- `starter.md`, `inject_subject_once.md`, `ender.md` 의 AI 운영 규칙 갱신
- 1회 개발 사이클 결과를 AI 에이전트가 이어받을 수 있는 형태로 압축
- 역할별 입력, 산출물, 금지사항, 반복 문제, 다음 작업 인계 정보 정리
- 사용자용 설명이 아니라 다음 AI 작업자가 참고할 운영 문맥 작성

## 시작 순서

1. 이 파일 `docs/agent/ai-docs/README.md` 를 먼저 읽고 AI 에이전트용 문서 관리 범위를 숙지한다.
2. `orchestra` 가 넘긴 1회 사이클 요약과 역할별 산출물만 AI 전용 문서로 압축한다.
3. `ender.md`, `inject_subject_once.md`, `docs/agent/README.md` 는 이 역할 README 를 먼저 읽은 뒤, 실제 운영 문서 갱신이 필요한 작업 입력에서 해당 경로가 명시될 때만 추가로 읽는다.
4. `starter.md` 는 서브 에이전트 생성 시점의 첫 읽기 문서나 전체 context 문서가 아니라 최상위 `orchestra` 세션 시작/복구 문서다. `starter.md` 갱신이 필요하면 `orchestra` 가 필요한 문제 상황과 좁은 발췌 또는 줄 범위를 작업 입력으로 제공하고, `ai-docs` 는 그 범위만 기준으로 수정한다.

## 접근 허용 범위

- `starter.md`
- `ender.md`
- `inject_subject_once.md`
- `docs/agent/README.md`
- `docs/agent/*/README.md`
- 각 역할의 `docs/agent/` 내부 관리 문서
- `orchestra` 가 전달한 작업 입력
- AI 문서 갱신에 필요한 코드와 설정 파일

## 접근 금지 범위

- `docs/human/`
- `README.md`
- 사용자용 보고서 문서

## 원칙

- `ai-docs` 는 사용자용 문서를 작성하지 않는다.
- `user-docs` 는 AI 에이전트용 개발 문서를 작성하지 않는다.
- 두 문서 시스템은 목적, 독자, 위치, 문체를 완전히 분리한다.
- AI 문서는 다음 에이전트가 작업을 이어가기 위한 지시, 제약, 결정, 문제 인계만 남긴다.
- 사용자에게 보여줄 설명, 개요, 튜토리얼, 자연어 보고서는 `user-docs` 역할로 보내고 직접 작성하지 않는다.
- 없는 기능이나 확인되지 않은 상태를 사실처럼 적지 않는다.
- 운영 문서 수정 책임자로서 범용 bootstrap 일반성을 보존하고, 역할 README 사이의 cross-doc consistency 를 확인한다.
- 실제 sub-agent 도구 부재나 상위 정책 차단을 단일 컨텍스트 fallback 구현으로 표현하지 않는다. 필요한 승인, 도구 노출, 운영 방식 수정 지점을 보고하고 중단하는 문장으로 통일한다.
- cycle close 계약을 바꿀 때는 `orchestra` dispatch 기반 `cycle-report`, artifact readback, frontend report server 자동 실행/재사용, readiness 확인, 해당 cycle URL 브라우저 오픈, 포트 충돌 fail-closed, trace DB fallback 생성 금지를 함께 맞춘다.
- 원문 evidence 계약을 바꿀 때는 사용자 프롬프트 수신 직후 `user_request_verbatim`, sub-agent spawn 직전 `agent_dispatch.prompt_verbatim`, completion 직후 가능한 result 원문 저장, `source_ref`/`hash_sha256`/`timestamp`/`order`/`role`/`agent_id` metadata, `audit.missing_evidence[]`, `audit.status=fail|warn`, derived/summary/display 라벨을 함께 맞춘다.
- runtime boundary evidence 계약을 바꿀 때는 repo 코드가 Codex `spawn_agent` tool invocation 자체를 자동 interception 하지 못할 수 있음을 명시하고, `code-level automatic` 과 `orchestra-protocol automatic` 을 분리한다. append 실패는 summary 대체 없이 spawn 중단 또는 fail-closed/incomplete evidence 로 맞춘다.
- 코드 변경 보고 계약을 바꿀 때는 `codegen` 반환 evidence, `review` evidence 검수, `cycle-report` 의 `report.json.code_changes`, `audit.json` 제외 사유, 한국어 report layer annotation 원칙을 함께 맞춘다.
- cycle-report/viewer 계약을 바꿀 때는 원문 evidence 우선 표시, 역할 지시/반환 원문, 테스트 원문, 전체 diff hunk, `excluded_code_changes` 사유, `audit.missing_evidence[]`/failure 화면 경고를 함께 맞춘다. `summary`, `display`, `derived` 는 보조 projection 이며 원문 evidence 누락을 대체하지 않는다고 명시한다.
- command evidence 계약을 바꿀 때는 `command_verbatim`, `result_verbatim`, `output_verbatim` 세 필드가 독립 표시되어야 한다고 역할 문서와 테스트 인계에 남긴다. 첫 non-empty 필드만 렌더링하는 동작은 회귀로 기록한다.
- orchestration 순서 보정이 있으면 실제 순서 오류와 보정 조치를 숨기지 않고 내부 문서에 남긴다. codegen 수정 뒤 test 전에 최신 review 반환이 있는지 확인하는 교훈처럼 다음 cycle 의 실행 게이트가 되는 내용은 `orchestra` 문서에도 반영한다.
- 제품 코드 구현 완료, 테스트 pass, 현재 cycle trace audit pass 여부는 서로 다른 판정 축으로 기록한다. 구현이 완료됐더라도 해당 cycle 의 append-only trace 오염, close-stage 누락, orchestration order 또는 step assignment 문제가 있으면 audit/report 상태는 별도로 fail/warn/incomplete 로 남긴다.
- trace ledger 는 append-only 로 취급한다. 이미 남은 giant `cycle_step`, duplicate `cycle_step`, 잘못된 순서의 boundary append 는 사후 재작성하지 않고 `audit.json`, `report.json`, 다음 cycle AI 인계에 그대로 노출한다.
- command 검증을 문서화할 때는 sandbox 결과와 escalated rerun 결과를 분리한다. sandbox `EPERM` 같은 환경 실패가 있었으면 pass 로 덮지 않고, escalated rerun pass 와 함께 별도 사실로 적는다.
- 문서가 비대해지면 오래된 작업 로그를 압축하고 현재 운영 규칙과 다음 작업 인계만 남긴다.
- 서브 에이전트로 실행될 때는 사용 가능한 최신 프론티어급 모델을 기본으로 사용한다. 2026-05-22 현재 기준 기본값은 `gpt-5.5` 와 `xhigh` 다.

## 사이클별 AI 인계 메모

정상 사이클에서 다음 AI 에이전트가 이어받아야 할 cycle-specific 운영 교훈은 `docs/agent/ai-docs/cycle-<cycle_id>.md` 형식으로 남긴다.
`docs/agent/ai-docs/handoff/latest.md` 는 `ender.md` 의 `interrupted-handoff-close` 전용이므로 정상 cycle close 메모로 사용하지 않는다.

초기 부트스트랩 상태에서는 최근 cycle-specific 인계 메모를 두지 않는다.
프로젝트 개발이 시작된 뒤 실제 cycle 에서 다음 에이전트가 반드시 이어받아야 할 운영 교훈이 생기면 이 섹션에 해당 파일을 추가한다.

## 개발 사이클 마지막 갱신

1회 개발 사이클의 마지막에서 `orchestra` 는 아래 입력을 `ai-docs` 에 전달한다.

- 처음 `planning` 계획 요약
- 코드 생성과 수정 요약
- 검수 문제점 리스트와 반영 여부
- 테스트 결과와 남은 문제점
- `analysis` 근본 원인 분석 요약
- 최종 `planning` 완성도 보고서
- 사용자에게 전달된 보고와 사용자의 반응
- 다음 사이클에서 AI 에이전트가 반드시 이어받아야 할 제약과 미결 사항

`ai-docs` 는 이 입력을 바탕으로 AI 전용 문서를 갱신한다.
이 갱신이 끝난 뒤에도 아직 1회 개발 사이클 완료가 아니다.
`orchestra` 가 `cycle-report` artifact 를 생성, readback 하고 frontend report server readiness 와 해당 cycle URL browser open 을 확인해 사용자가 보고서를 확인할 수 있게 해야 사이클 종료 조건이 충족된다.

## 중간 중단 인계

`ender.md` 가 `interrupted-handoff-close` 로 종료 유형을 분류하면 `ai-docs` 는 전체 역할 문서나 루트 운영 문서를 대신 정리하지 않는다.
이 경우 자기 작업 상태만 `docs/agent/ai-docs/handoff/latest.md` 에 남긴다.

남길 내용:

- 처리하려던 AI 문서 갱신 범위
- 아직 반영하지 못한 운영 규칙
- 보류한 판단
- 다음 `ai-docs` 세션이 이어야 할 작업
- 자기 `Context Report`

## Context Report

작업 종료 응답에는 아래 항목을 포함한다.

- `context_level`: `low`, `medium`, `high`, `near-limit`
- `basis`: 판단 근거
- `read_files`: 실제로 읽은 주요 파일
- `carryover_summary`: 다음 역할이 이어받아야 할 요약
- `recommended_action`: `continue` 또는 `close_and_respawn`
