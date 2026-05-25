# Codegen Constraints

이 문서는 `codegen` 이 구현할 때 지켜야 하는 정적 제약을 적는다.
`codegen` 은 이 문서를 읽을 수 있지만 직접 작성하거나 갱신하지 않는다.

## 기본 규칙

- 클린 아키텍처 레이어를 침범하지 않는다.
- 안쪽 레이어는 바깥 레이어를 참조하지 않는다.
- `domain` 은 순수 규칙과 핵심 개념만 담고 외부 IO, adapter, framework 세부를 알지 않는다.
- `application` 은 유스케이스와 port 를 담고 concrete infrastructure 구현에 직접 의존하지 않는다.
- `infrastructure` 는 외부 시스템 adapter 와 기술 세부를 담당한다.
- `apps/xavi-bootstrap` 은 실행 진입점과 composition root 로만 사용한다.
- 하네스는 별도 개발 시스템으로 유지한다.
- 하네스 기반 테스트 코드 작성/보강의 기본 소유자는 `review` 이다.
- `codegen` 은 `orchestra` 가 명시적으로 허용한 경우에만 `crates/xavi-harness/` 를 수정한다.
- 하네스 수정을 허용받으면 fixture, double, scenario, assertion, tests 구조를 사용한다.
- 파일은 책임 기준으로 배치한다.

## 관리 기준

- 새 구현 제약이 필요하면 주제 주입 단계에서 `ai-docs` 가 `codegen` 이 읽을 수 있는 정적 컨텍스트로 이 문서에 반영한다.
- 구현 도중 반복되는 제한 조건도 `codegen` 이 직접 누적하지 않고, 필요한 경우 상위 세션에 반환 요약으로 보고한다.
