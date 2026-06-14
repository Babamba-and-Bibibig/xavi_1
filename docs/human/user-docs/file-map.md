# User File Map

이 문서는 사용자가 파일별 역할을 빠르게 이해하도록 돕는 안내 문서다.
현재는 특정 프로젝트 주제 주입 전이므로 부트스트랩 구조 중심으로만 정리한다.

## 주요 부트스트랩 파일

- `starter.md`
  - 최상위 세션 시작 규약이다.
  - 역할이 명시되지 않은 개발 요청을 `orchestra` 로 라우팅하는 기준을 제공한다.
- `inject_subject_once.md`
  - 범용 health-check 부트스트랩에 최초 프로젝트 주제를 주입할 때 사용하는 규약이다.
- `ender.md`
  - 정상 종료와 중간 인계 종료를 구분하고, 역할별 문서 정리 책임을 정한다.
- `docs/agent/`
  - AI 역할별 운영 문서를 둔다.
  - 사용자용 설명이 아니라 다음 AI 작업자가 따라야 할 책임과 금지사항을 담는다.
- `docs/human/user-docs/`
  - 주제 주입 뒤 사용자에게 보여줄 문서를 둔다.
  - 개발이 시작된 뒤 이 문서들은 Git에 올라갈 수 있으며, 문서 디렉터리를 ignore 해서 숨기지 않는다.
- `apps/xavi-bootstrap/`
  - 현재 실행 진입점이다.
  - health-check 예제와 trace 관련 CLI 경로를 조립한다.
- `apps/xavi-dev-console/`
  - `development_trace` 와 cycle report artifact 를 브라우저에서 확인하게 돕는 부트스트랩 운영 지원 도구다.
  - 사용자 입력 저장, report viewer, readiness 확인 같은 로컬 지원 화면을 담당한다.
- `crates/xavi-domain/`
  - 도메인 모델과 검증 규칙을 둔다.
- `crates/xavi-application/`
  - 유스케이스와 port 계약을 둔다.
- `crates/xavi-infrastructure/`
  - SQLite 등 외부 저장소 adapter 구현을 둔다.
- `crates/xavi-harness/`
  - 시나리오 기반 검증 하네스를 둔다.
- `.xavi/development_trace.sqlite3`
  - 개발 cycle evidence 원장으로 쓰이는 로컬 DB 경로다.
  - raw evidence 의 기준이며, 사용자 문서가 이 DB 내용을 임의로 재작성하지 않는다.

## 주제 주입 뒤 확장할 내용

프로젝트 주제가 정해지면 각 파일 설명은 실제 도메인 기능 기준으로 다시 채운다.
파일별 책임, 어느 레이어에 속하는지, 사용자가 읽을 때 주목할 포인트를 짧게 남긴다.
