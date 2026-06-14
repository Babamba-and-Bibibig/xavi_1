# User Feature Map

이 문서는 프로젝트 주제 주입 뒤 기능 단위로 구현 방식과 사용자 관점 의미를 정리하는 곳이다.
현재 저장소는 범용 AI 협업 부트스트랩 초기 상태이므로 특정 기능 이력은 기록하지 않는다.

## 초기 상태

- 아직 사용자 프로젝트 기능은 정의되지 않았다.
- 현재 코드의 health-check 예제와 AI 운영 도구는 새 프로젝트를 시작하기 위한 부트스트랩 구조다.
- 과거 cycle id, 별칭, 로컬 실행 결과는 이 초기 템플릿에 남기지 않는다.

## 부트스트랩에서 유지되는 개념

### 역할 기반 개발 루프

- `orchestra` 가 사용자와 대화하고 역할별 작업을 조율한다.
- `planning`, `codegen`, `review`, `test`, `analysis`, `user-docs`, `ai-docs`, `cycle-report` 가 각자 정해진 책임을 가진다.
- 실제 sub-agent 도구가 없으면 단일 컨텍스트 구현으로 우회하지 않고 제한을 보고한다.

### development_trace

- 개발 cycle 중 발생한 사용자 요청, 역할 지시, 역할 반환, 검증 결과의 원장 역할을 한다.
- raw evidence 는 가능한 한 그대로 보존하고, 화면과 요약은 파생 projection 으로 구분한다.
- 누락된 원문 evidence 는 요약으로 복원하지 않고 누락 상태로 표시해야 한다.

### cycle report 와 dev-console

- `cycle-report` 는 한 작업 cycle 이 끝날 때 HTML/JSON/raw/audit/context artifact 를 만든다.
- `apps/xavi-dev-console` 은 이 artifact 와 `development_trace` 를 브라우저에서 확인하게 돕는 부트스트랩 운영 지원 도구다.
- dev-console 은 보고서를 새로 꾸며내는 대체 경로가 아니라, 이미 생성된 근거를 보여주는 viewer 로 취급한다.

### cycle alias

- 긴 canonical cycle id 는 자동화와 artifact 경로의 기준으로 유지한다.
- 사람이 회의나 문서에서 부를 짧은 이름은 `<category-NNN>` 형식의 alias 로 둘 수 있다.
- 예시는 placeholder 로만 둔다: `<cycle-id>` 와 `<category-NNN>`.

## 주제 주입 뒤 작성할 항목

각 기능마다 아래 항목을 채운다.

- 기능 이름
- 관련 파일
- 구현 목적
- 동작 방식
- 핵심 로직 요약
- 사용자가 확인해야 할 포인트
- 검증 결과와 남은 제한
