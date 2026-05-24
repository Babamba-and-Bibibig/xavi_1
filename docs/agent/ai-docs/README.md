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
3. `starter.md`, `ender.md`, `inject_subject_once.md`, `docs/agent/README.md` 는 실제 운영 문서 갱신이 필요할 때만 읽는다.

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
- 문서가 비대해지면 오래된 작업 로그를 압축하고 현재 운영 규칙과 다음 작업 인계만 남긴다.
- 서브 에이전트로 실행될 때는 사용 가능한 최신 프론티어급 모델을 기본으로 사용한다. 2026-05-22 현재 기준 기본값은 `gpt-5.5` 와 `xhigh` 다.

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
이 갱신까지 끝나야 1회 개발 사이클이 종료된다.

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
