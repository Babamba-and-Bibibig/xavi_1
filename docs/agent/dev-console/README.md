# Dev Console Agent Docs

이 폴더는 로컬 HTML 개발 참여 콘솔을 담당하는 프로젝트별 전문 역할 문서다.
`dev-console` 은 기본 7개 개발 루프 역할을 대체하지 않는다.
`orchestra` 가 개발 사이클 상태를 HTML/JSON 화면과 입력 큐로 노출해야 할 때만 실행한다.

## 담당 역할

- `development_trace` 원장을 읽어 사이클별 화면용 report model 구성
- 로컬 전용 HTML/CSS/vanilla JS 화면 구현
- 로컬 HTTP endpoint 구현 또는 보강
- HTML에서 받은 승인, 수정 요청, 질문을 append-only trace 입력으로 저장
- dev console 내부 클릭/입력처럼 콘솔 안에서 발생한 사용자 액션만 안정적으로 처리

## 시작 순서

1. 이 파일 `docs/agent/dev-console/README.md` 를 먼저 읽고 역할 경계를 확정한다.
2. `orchestra` 가 넘긴 cycle id, DB 경로, 서버 바인딩, 입력 큐 요구사항을 확인한다.
3. UI와 endpoint 변경은 `apps/xavi-dev-console/` 을 기본 작업 범위로 삼는다.
4. trace projection 이나 shared model 이 필요하면 `orchestra` 가 허용한 application/domain 범위 안에서만 추가한다.

## 접근 허용 범위

- `docs/agent/dev-console/README.md`
- `apps/xavi-dev-console/`
- `development_trace` 구현을 이해하는 데 필요한 source files
- `orchestra` 가 명시한 application/domain/infrastructure 파일
- dev console 검증에 필요한 하네스/테스트 파일

## 접근 금지 범위

- 다른 역할의 운영 문서
- `docs/human/`
- 사용자 홈 디렉터리, OS 설정
- 현재 콘솔 기능과 무관한 제품 코드

## 원칙

- source of truth 는 `.xavi/development_trace.sqlite3` 다.
- HTML/JSON 보고서는 파생 산출물이며 원장을 대체하지 않는다.
- 서버는 기본적으로 `127.0.0.1` 에만 바인딩한다.
- 1차 MVP 는 zero-npm, static HTML/CSS/vanilla JS 로 유지한다.
- HTML 입력은 Codex 세션을 직접 조종한다고 표현하지 않는다.
- HTML 입력은 append-only trace 또는 pending input queue 에 저장하고, `orchestra` 가 readback 해서 처리한다.
- `serve`, `export`, `open-cycle` 은 trace projection 과 report artifact viewer 운영을 위한 CLI 경계다.
- report server 는 이미 생성된 `cycle-report` artifact 를 서빙하는 viewer 이며, trace DB 에서 누락 artifact 를 합성하지 않는다.
- 외부 화면이나 OS 수준 사용자 행동을 dev-console 이 직접 수집하거나 조종한다고 표현하지 않는다.
- 비밀번호, 토큰, 카드번호, hidden input, 민감 URL query 값은 저장하지 않는다.
- 숨겨진 raw chain-of-thought 를 보여준다고 주장하지 않는다. UI에는 공개 가능한 orchestration artifact 만 표시한다.

## Current Bootstrap State

- app path: `apps/xavi-dev-console`
- source projection: `.xavi/development_trace.sqlite3` 를 local HTML console, report JSON, SSE role lanes 로 투영한다.
- CLI responsibility: `serve`, `export`, `open-cycle`
- report server responsibility: 이미 생성된 `cycle-report` artifact viewer
- report routes: `/reports`, `/reports/<cycle_id>/`, `/api/reports`, `/api/reports/<cycle_id>/ready`, alias routes
- `open-cycle` responsibility: artifact 존재 확인, matching local report server 재사용 또는 동일 configured server 시작, readiness 확인, 브라우저에서 해당 report 열기
- fail-closed principles: trace DB fallback synthesis 금지, 임의 포트 fallback 금지, wrong or stale server 재사용 금지
- cycle-specific verification snapshots are not stored in this baseline role document.

## Report Alias Routes

cycle alias report routing 은 dev-console viewer 기능이다.
source of truth 는 SQLite `development_cycle_aliases` 원장이고, report root 의
`aliases.json` 은 viewer/index projection 이다.

현재 route:

- `GET /api/reports/aliases.json`
- `GET /reports/by-alias/<alias>/`
- `GET /api/reports/by-alias/<alias>/<file>`

`/reports/by-alias/<alias>/` 는 `aliases.json` 을 통해 canonical
`/reports/<cycle_id>/` artifact 로 resolve 한다.
canonical route 는 계속 `/reports/<cycle_id>/` 이다.

`aliases.json` 은 strict parse 한다.
root, version, aliases array, entry object, required field, category/category_key/sequence
consistency 가 malformed 이면 전체 alias index 를 실패로 본다.
malformed entry 를 조용히 제외하고 나머지 alias 를 성공처럼 보여주지 않는다.

route 실패 정책:

- percent-encoded slash 를 포함한 alias 는 거부한다.
- artifact filename traversal 은 거부한다.
- 같은 alias 가 둘 이상의 cycle 로 매핑되면 ambiguous alias 로 실패한다.
- malformed `aliases.json` 은 `422/report_alias_index_malformed` 계열 응답과 화면 오류로 드러낸다.
- by-alias route 는 trace DB 에서 report 를 fallback 합성하지 않는다.

## Cycle Report Viewer Evidence Contract

dev-console report viewer 는 `.xavi/reports/development_cycles/<cycle_id>/`
artifact 를 보여주는 viewer 다.
요약 카드만 보여주는 대시보드가 아니라, `cycle-report` 가 만든 원문 evidence 중심
artifact 를 탐색하게 하는 화면이어야 한다.

기본 표시 근거:

- `user_request_verbatim`, `orchestra_delegations[].prompt_verbatim`, role return 원문
- 테스트 명령, 결과, 출력 원문
- `report.json.code_changes[]` 의 전체 hunk/line evidence
- `audit.json.excluded_code_changes[]` 의 제외 사유
- `audit.missing_evidence[]`, `audit.status`, `evidence_status`

`summary`, `display`, `derived` 필드는 navigation/display 보조 정보다.
원문 evidence 가 없으면 viewer 는 원문 없음과 audit warning/failure 를 보여주고,
summary/display/derived 를 원문처럼 승격하지 않는다.

명령 evidence 렌더링에서는 `command_verbatim`, `result_verbatim`,
`output_verbatim` 을 독립 블록으로 표시한다.
세 필드 중 첫 번째 non-empty 값만 고르는 렌더링은 회귀다.
세 필드가 모두 있으면 세 값이 모두 화면과 render model 에 남아야 하며,
없는 필드는 다른 필드에서 복사하지 않고 누락으로 남긴다.

## Report Open-Cycle Readiness Contract

`cycle-report` artifact 생성 뒤 브라우저에서 해당 cycle report 를 열어 확인하는 경로는
`xavi-dev-console open-cycle` 과 같은 report server 경로다.
Python static server, 임의 포트 변경, trace DB fallback report 합성은 성공 판정 경로가 아니다.

`open-cycle` 은 아래 순서로 fail-closed 판정한다.

- 먼저 `.xavi/reports/development_cycles/<cycle_id>/index.html` 이 존재하고 유효한지 확인한다.
- 같은 host/port 에 서버가 있으면 `/api/health` 로 `service=xavi-dev-console`, `status=ok`, 동일 `reports_dir` 를 확인한다.
- matching server 가 없으면 configured host/port 에 같은 report server 를 시작한다.
- 이어 `GET /api/reports/<cycle_id>/ready` 로 `service=xavi-dev-console`, `status=ok`, 동일 `reports_dir`, 동일 `cycle_id`, `artifact_files_present=true`, `index_html_bytes>0` 을 확인한다.
- readiness 판정은 큰 `/reports/<cycle_id>/` HTML 본문 fetch 에 의존하지 않는다.
- health 또는 ready 가 맞지 않으면 matching server 로 재사용하지 않는다.

대형 report artifact 경계:

- `/api/reports/<cycle_id>/report.json` 같은 artifact API 는 크기 제한으로 실패할 수 있다.
- 그 경우에도 `/reports/<cycle_id>/` 는 `cycle-report` 가 이미 만든 prebuilt `index.html` 을 직접 서빙해야 한다.
- `/reports/<cycle_id>/` 가 큰 `report.json` 을 다시 읽어 HTML 을 조립하다가 대형 artifact 오류를 내는 동작은 회귀다.
- `GET /api/reports/<cycle_id>/ready` 는 작은 JSON readiness marker 여야 하며 대형 HTML 전체를 반환하지 않는다.

prebuilt `index.html` 직접 서빙 경계:

- `reports_dir` 는 canonicalize 된 directory 여야 한다.
- `cycle_dir` 는 canonicalize 뒤 `reports_dir` 안에 있어야 한다.
- artifact file name 은 허용 목록 또는 안전한 file name 검증을 통과해야 한다.
- `index.html` symlink 는 거부한다.
- `report.json`, `raw.json`, `audit.json`, `context.md` 같은 required artifact 도 canonicalize 뒤 `cycle_dir` 밖으로 escape 하면 거부한다.
- missing artifact 를 trace DB 에서 합성하거나 `report.json` 으로 `index.html` 을 재구성하지 않는다.

포트와 stale server 처리:

- report server 포트는 repo 설정값 또는 `orchestra` 가 전달한 값으로 고정한다.
- 지정 포트에 오래된 `xavi-dev-console serve` 가 떠 있고 새 `/api/reports/<cycle_id>/ready` endpoint 를 모르면, 해당 프로세스를 종료하고 새 바이너리를 같은 포트에서 다시 띄운다.
- 포트를 바꿔서 성공처럼 보이게 하는 것은 우회다.
- 같은 포트가 다른 프로세스에 점유되어 있으면 임의 포트 fallback 없이 fail-closed 로 보고한다.

현재 회귀 테스트가 고정하는 경계:

- `cycle_report_artifact`: prebuilt index 직접 서빙, artifact bundle validation, symlink/canonical containment
- `open_cycle_probe`: health/ready marker 재사용, readiness route failure, large HTML fetch 회피, wrong server fail-closed
- `cycle_report`: report viewer/schema/audit 관련 회귀 묶음

## Report Viewer Regression Notes

다음 버그는 이미 수정된 회귀로 취급한다.

- alias route 는 malformed alias index 를 부분 성공으로 숨기지 않는다.
- report artifact viewer 는 prebuilt `index.html` 을 직접 서빙하고 trace DB 에서 누락 artifact 를 합성하지 않는다.
- large artifact API 실패가 `/reports/<cycle_id>/` prebuilt HTML 서빙 실패로 번지면 회귀다.
- symlink containment 와 canonical path containment 검사를 우회하면 회귀다.
- ready endpoint 는 작은 readiness marker 로 유지하고, 큰 HTML 본문 fetch 를 readiness 판정으로 사용하지 않는다.
- wrong or stale server 는 재사용하지 않고 fail-closed 로 처리한다.

현재 테스트가 고정하는 report viewer 경로:

- alias route strict parse 와 canonical cycle artifact resolve
- prebuilt report artifact 직접 서빙
- large artifact 와 ready endpoint 분리
- symlink/canonical containment
- wrong server fail-closed

## Verification Notes Template

새 dev-console cycle 의 검증 메모가 필요하면 아래 형식으로만 남긴다.
초기 role 문서에는 특정 과거 cycle id, alias, 로컬 프로세스 번호, 로컬 최종 URL, pass count 를 남기지 않는다.

```text
## <cycle-id 또는 cycle placeholder>

- scope:
- commands:
- result:
- remaining risks:
```

## Context Report

작업 종료 응답에는 아래 항목을 포함한다.

- `context_level`: `low`, `medium`, `high`, `near-limit`
- `basis`: 판단 근거
- `read_files`: 실제로 읽은 주요 파일
- `carryover_summary`: 다음 역할이 이어받아야 할 요약
- `recommended_action`: `continue` 또는 `close_and_respawn`
