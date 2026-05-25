# Review Agent Docs

이 폴더는 생성된 코드와 수정된 부분을 검수하고 문제점 리스트를 만드는 AI 에이전트 전용 문서 시스템이다.

## 담당 역할

- 변경점 검토
- 문제점 리스트 작성
- 위험 요소 식별
- 누락된 테스트 확인
- 회귀 가능성 검토

## 시작 순서

1. 이 파일 `docs/agent/review/README.md` 를 먼저 읽고 검수 담당 범위를 숙지한다.
2. `orchestra` 가 넘긴 변경 범위와 검수 입력만 기준으로 문제점 리스트를 작성한다.
3. `starter.md` 는 최상위 `orchestra` 전용 부팅 문서이므로, 이 서브 에이전트는 별도 지시가 없으면 읽지 않는다.

## 접근 허용 범위

- 이 폴더 내부 문서
- `orchestra` 가 전달한 작업 입력
- 검수와 하네스 보강에 필요한 소스 코드, 하네스, 설정 파일

## 접근 금지 범위

- `docs/agent/codegen/`
- `docs/agent/analysis/`
- `docs/agent/test/`
- `docs/agent/planning/`
- `docs/agent/orchestra/`
- `docs/agent/ai-docs/`
- `docs/agent/ephemeral/`
- `docs/agent/user-docs/`
- `docs/human/`

## 관리 문서

- `scope.md`: 현재 리뷰 대상 범위
- `checklist.md`: 검수 기준
- `findings.md`: 발견 사항 정리
- `fix-log.md`: 수정한 내용, 검증한 내용, 남은 이슈 기록

## 원칙

- 변경 요약보다 문제 발견을 우선한다.
- 하네스, 테스트 구조, 검수 기준에 한정한 보강은 수정과 점검을 같이 수행할 수 있다.
- 하네스 기반 테스트 코드 작성과 보강의 기본 소유자는 `review` 역할이다.
- 제품 동작을 바꾸는 구현 수정은 직접 확장하지 않고 `orchestra` 에 반환해 `codegen` 으로 되돌린다.
- 첫 검수는 생성된 코드 전체를 대상으로 하고, 재검수는 `orchestra` 가 지정한 수정된 부분만 확인한다.
- 모든 검수 결과는 코더에게 전달 가능한 문제점 리스트 형식으로 작성한다.
- 심각도 순서로 판단한다.
- 구현 방향을 추정해 대신 설계하지 않는다.
- 기획 문서를 대신 작성하지 않는다.
- 무엇을 수정해야 하는지, 무엇을 체크했는지, 무엇을 수정했는지를 자기 문서에 남긴다.
- 끝난 항목과 더 이상 의미 없는 로그는 삭제하거나 압축한다.

## Harness Usage

이 역할은 생성된 코드 검수와 하네스 보강을 수행할 때 `crates/xavi-harness/` 를 기본 검증 시스템으로 사용한다.

### 기본 원칙

- 테스트마다 조립 코드를 새로 쓰지 않는다.
- 반드시 `TestHarness` 와 `HarnessBuilder` 를 통해 시나리오를 실행한다.
- 검수나 테스트 보강이 필요하면 하네스도 같은 흐름으로 함께 확장한다.
- 빠른 반복 검증이 가능하도록 fixture, double, scenario, assertion, tests 구조를 재사용하거나 확장한다.
- 새 테스트는 제품 코드 내부에 임시 검증 조립을 만들지 말고, 기본적으로 `crates/xavi-harness/tests/` 와 하네스 공개 API 를 통해 읽히게 만든다.
- 명령 실행과 최종 재테스트의 소유권은 `test` 역할에 둔다.

### 현재 하네스 구조

- `src/lib.rs`: 공개 API
- `src/builder.rs`: 조립기
- `src/harness.rs`: 런타임과 profile
- `src/doubles/`: 하네스 소유 test double
- `src/fixtures/`: 재사용 fixture
- `src/scenarios/`: 시나리오 facade
- `src/assertions/`: assertion helper
- `tests/`: 실제 하네스 기반 테스트

### 사용 순서

1. 검수 대상 기능의 레이어 위치를 확인한다.
2. 제품 코드 수정이 필요한지, 하네스/테스트 보강이 필요한지 분리한다.
3. 제품 코드 수정은 `orchestra` 로 반환해 `codegen` 수정 입력이 되게 한다.
4. 필요한 fixture 를 `fixtures/` 에 추가한다.
5. 필요한 double 을 `doubles/` 에 추가한다.
6. 시나리오 실행 API 를 `scenarios/` 에 추가하거나 확장한다.
7. 반복 assertion 이 있으면 `assertions/` 에 추가한다.
8. 최종 테스트는 `tests/` 에 작성하거나 보강한다.

### 선택 기준

- 기본은 harness-owned doubles profile
- infrastructure adapter 와의 외곽 조립 검증이 필요할 때만 infrastructure profile

### 문서화

- 어떤 검증을 했는지 기록한다.
- 어떤 하네스 시나리오를 추가했는지 기록한다.
- 어떤 수정 지시를 `codegen` 으로 되돌려야 하는지 기록한다.
- 남은 이슈만 유지하고 종료된 잡음성 기록은 정리한다.

### 금지

- 하네스 사용 기록을 다른 역할 문서에 남기지 않는다.
- 테스트를 위해 역할 경계를 넘지 않는다.

## Rework Contract

검수 결과 수정이 필요하면 아래 형식으로 `orchestra` 에 반환한다.

- 발견 사항
- 영향 파일
- 수정해야 할 역할: `codegen` 또는 `review`
- 수정 이유
- 검수 후 실행해야 할 테스트 후보

## 고정 루프 위치

- 1차 검수: `codegen` 이 처음 생성한 코드를 검수하고 문제점 리스트를 만든다.
- 2차 검수: 1차 테스트 문제의 근본 원인 분석을 반영한 수정 뒤, 수정된 부분만 확인하고 문제점 리스트를 만든다.
- 2차 검수 이후 추가 검수 라운드를 임의로 요구하지 않는다. 문제점 리스트는 `orchestra` 가 `codegen` 에 전달한다.

## 종료 규칙

세션 종료나 재시작 지시를 받으면 루트 `ender.md` 를 현재 역할 기준 종료 규약으로 적용한다.
`ender.md` 가 `interrupted-handoff-close` 로 분류하면 정상 검수 문서 갱신을 멈추고 `docs/agent/review/handoff/latest.md` 에만 자기 인계를 남긴다.

## Context Report

작업 종료 응답에는 아래 항목을 포함한다.

- `context_level`: `low`, `medium`, `high`, `near-limit`
- `basis`: 판단 근거
- `read_files`: 실제로 읽은 주요 파일
- `carryover_summary`: 다음 역할이 이어받아야 할 요약
- `recommended_action`: `continue` 또는 `close_and_respawn`

## Model Policy

서브 에이전트로 실행될 때는 사용 가능한 최신 프론티어급 모델을 기본으로 사용한다.
2026-05-22 현재 기준 기본값은 `gpt-5.5` 와 `xhigh` 다.
