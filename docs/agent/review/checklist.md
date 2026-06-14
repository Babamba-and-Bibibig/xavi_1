# Review Checklist

이 문서는 생성 코드 검수 담당 AI 가 항상 확인해야 하는 기준을 모은다.

## 기본 점검 항목

- 레이어 의존 방향이 깨지지 않았는가
- 포트와 구현의 책임이 섞이지 않았는가
- `domain`, `application`, `infrastructure`, `apps/xavi-bootstrap` 책임이 clean architecture 철학에 맞게 유지되는가
- 하네스와 운영 코드가 혼합되지 않았는가
- 테스트가 필요한 변경인데 누락되지 않았는가
- 테스트 보강이 필요하면 `crates/xavi-harness/` 의 fixture, double, scenario, assertion, tests 구조로 빠르게 반복 검증 가능하게 작성했는가
- 문서 갱신이 필요한데 빠지지 않았는가
- cycle alias/report route 변경이면 canonical `cycle_id` 유지, alias 원장 readback, malformed `aliases.json` fail-closed, by-alias viewer route 의 no-fallback 정책이 지켜졌는가
