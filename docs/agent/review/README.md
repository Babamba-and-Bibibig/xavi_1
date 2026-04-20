# Review Agent Docs

이 폴더는 코드 수정과 검수를 담당하는 AI 에이전트 전용 문서 시스템이다.

## 담당 역할

- 변경점 검토
- 필요한 코드 수정
- 위험 요소 식별
- 누락된 테스트 확인
- 회귀 가능성 검토

## 접근 허용 범위

- 이 폴더 내부 문서
- 공통 진입 문서인 `starter.md`
- 공통 인덱스인 `docs/agent/README.md`

## 접근 금지 범위

- `docs/agent/codegen/`
- `docs/agent/planning/`
- `docs/agent/ephemeral/`
- `docs/human/`

## 관리 문서

- `scope.md`: 현재 리뷰 대상 범위
- `checklist.md`: 검수 기준
- `findings.md`: 발견 사항 정리
- `fix-log.md`: 수정한 내용, 검증한 내용, 남은 이슈 기록

## 원칙

- 변경 요약보다 문제 발견을 우선한다.
- 필요한 수정이 있으면 수정과 점검을 같이 수행할 수 있다.
- 심각도 순서로 판단한다.
- 구현 방향을 추정해 대신 설계하지 않는다.
- 기획 문서를 대신 작성하지 않는다.
- 무엇을 수정해야 하는지, 무엇을 체크했는지, 무엇을 수정했는지를 자기 문서에 남긴다.
- 끝난 항목과 더 이상 의미 없는 로그는 삭제하거나 압축한다.

## Harness Usage

이 역할은 테스트와 검증을 수행할 때 `crates/xavi-harness/` 를 기본 검증 시스템으로 사용한다.

### 기본 원칙

- 테스트마다 조립 코드를 새로 쓰지 않는다.
- 반드시 `TestHarness` 와 `HarnessBuilder` 를 통해 시나리오를 실행한다.
- 검수나 수정이 필요하면 하네스도 같은 흐름으로 함께 확장한다.

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
2. 필요한 수정이 있으면 먼저 반영한다.
3. 필요한 fixture 를 `fixtures/` 에 추가한다.
4. 필요한 double 을 `doubles/` 에 추가한다.
5. 시나리오 실행 API 를 `scenarios/` 에 추가하거나 확장한다.
6. 반복 assertion 이 있으면 `assertions/` 에 추가한다.
7. 최종 테스트는 `tests/` 에 작성하거나 보강한다.

### 선택 기준

- 기본은 harness-owned doubles profile
- infrastructure adapter 와의 외곽 조립 검증이 필요할 때만 infrastructure profile

### 문서화

- 어떤 검증을 했는지 기록한다.
- 어떤 하네스 시나리오를 추가했는지 기록한다.
- 어떤 수정이 테스트를 통과시켰는지 기록한다.
- 남은 이슈만 유지하고 종료된 잡음성 기록은 정리한다.

### 금지

- 하네스 사용 기록을 다른 역할 문서에 남기지 않는다.
- 테스트를 위해 역할 경계를 넘지 않는다.
