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
3. `starter.md` 는 최상위 `orchestra` 의 세션 시작/복구 문서이므로, `test` 의 첫 읽기 문서나 일반 작업 context 로 읽지 않는다.

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
- docs-only 작업은 `rg`, diff, readback, `git diff --check` 중심으로 검증하고, 코드 변경이 없으면 Cargo 테스트를 불필요하게 실행하지 않는다.
- 하네스 테스트가 있으면 하네스 테스트를 우선 실행한다.
- `test` 역할은 테스트 코드를 작성하거나 제품 코드를 수정하지 않는다.
- 테스트 코드가 없거나 부족하면 직접 만들지 않고, 누락된 하네스 fixture, double, scenario, assertion, tests 항목을 문제점 리스트로 반환해 `orchestra` 가 `review` 에 보강을 맡기게 한다.
- 반복 테스트는 기존 `crates/xavi-harness/` 구조와 Cargo 명령을 기준으로 실행한다.
- 실패하면 실패 명령, 핵심 오류, 재현 방법, 문제점 리스트를 반환한다.
- 문제점의 근본 원인 판단은 `analysis` 역할에 맡기고, `test` 는 분석을 대신하지 않는다.
- cycle close 또는 report 검증을 배정받으면 artifact 파일 존재, `report.json` 필수 키, 코드 변경 시 `code_changes` 존재, `audit.json` 누락/제외 사유, report server 고정 포트 동작을 확인한다.
- 원문 evidence 검증을 배정받으면 `user_request_verbatim`, `agent_dispatch.prompt_verbatim`, 가능한 completion result verbatim event 가 trace DB/context bundle 에 있고, 각 event 에 `source_ref`, `hash_sha256`, `timestamp`, `order`, `role`, `agent_id` 가 있는지 확인한다.
- v2 trace event 의 `source_ref` 와 `hash_sha256` 가 없는 prompt 를 원문처럼 표시하면 실패로 반환한다.
- `spawn_agent` 직전 `xavi-bootstrap trace append` 또는 동등 trace append evidence 가 없으면 runtime append evidence 누락으로 기록하고, `evidence_status=incomplete|fail` 인지 확인한다.
- 원문 evidence 가 없으면 `audit.missing_evidence[]`, `audit.status=fail|warn`, HTML 화면 경고가 있는지 확인한다. 누락 상태를 success 처럼 보이게 표시하면 실패로 반환한다.
- append 실패 후 summary, derived, display, report 문자열로 대체해 success 처리하면 fail-closed 위반으로 실패 반환한다.
- `summary`, `derived`, `display` 데이터가 원문 필드처럼 표시되거나 기존 report 를 원문처럼 복원/재생성하는 동작은 성공으로 인정하지 않는다.
- command evidence 검증을 배정받으면 `command_verbatim`, `result_verbatim`, `output_verbatim` 이 모두 독립 렌더링되는지 확인한다. 첫 non-empty 원문 필드만 표시하거나 다른 필드 값을 복사해 채우는 동작은 실패로 반환한다.
- report server 검증에서 지정 포트가 다른 프로세스에 점유되어 있으면 임의 포트로 바꾸지 않고 실패로 반환한다.
- dev-console/report server 는 artifact viewer 로만 검증하며, trace DB 에서 fallback 보고서를 생성하는 동작을 성공으로 인정하지 않는다.
- cycle alias 검증을 배정받으면 `trace reserve-alias`/`trace resolve-alias` readback,
  `aliases.json`, canonical `/reports/<cycle_id>/`, by-alias viewer route,
  malformed/ambiguous/traversal fail-closed 경로를 확인한다.
- alias 검증에서 malformed `aliases.json` entry 를 조용히 제외하고 나머지를 성공 처리하면 실패로 반환한다.
- `resolve-alias` 결과의 `cycle_id` 가 현재 cycle 과 다르거나 alias 충돌 뒤 임의 fallback alias 를 만들면 실패로 반환한다.
- 서브 에이전트로 실행될 때는 사용 가능한 최신 프론티어급 모델을 기본으로 사용한다. 2026-05-22 현재 기준 기본값은 `gpt-5.5` 와 `xhigh` 다.

## Verification Result Template

새 작업 cycle 의 검증 결과는 필요할 때 아래 형식으로 짧게 남긴다.
초기 부트스트랩 문서에는 특정 과거 cycle id, alias, pass count 를 남기지 않는다.

```text
## <cycle-id 또는 cycle placeholder>

- scope:
- commands:
- result:
- failures:
- residual risk:
```

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
