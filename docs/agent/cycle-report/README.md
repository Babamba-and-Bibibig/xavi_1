# Cycle Report Agent Docs

이 폴더는 1회 작업 사이클 종료 직후 실행되는 필수 보고서 artifact 역할 문서다.
`cycle-report` 는 사용자 문서나 AI 인계 문서를 쓰는 역할이 아니라, 해당 사이클에서 실제로 일어난 일을 HTML/JSON/raw/audit/context 산출물로 고정한다.
보고서의 사용자 목적은 각 역할군이 실제로 무엇을 지시받았고 무엇을 했는지 브라우저에서 시각적으로 확인하게 하는 것이다.

## 담당 역할

- 성공, 실패, 중단, blocked 를 포함한 모든 작업 사이클 종료 상태 보고서 작성
- `orchestra` 가 전달한 context bundle 을 검증하고 누락을 `audit.json` 에 기록
- 사용자의 최초 요청, 정정, 승인, 보류 지시를 사이클 단위로 정리
- `orchestra` 의 역할별 dispatch, return, 반환 검수, `Context Report` 를 사실 기반으로 정리
- trace DB/context bundle 에서 받은 `user_request_verbatim`, `agent_dispatch.prompt_verbatim`, completion result 원문 evidence 를 복사하고 metadata 를 검증
- 변경 파일, 실행 명령, 검증 결과, 실패 로그, trace export/audit 참조를 산출물에 연결
- 코드 생성/수정/삭제가 있었으면 모든 변경 hunk 를 `report.json.code_changes` 에 기록
- 보고서는 요약 중심 산출물이 아니라 원문 evidence 우선 산출물로 만든다. 역할 지시/반환, 테스트 원문, 전체 diff hunk, 제외된 코드 변경 사유, audit failure/missing evidence 를 기본 노출 근거로 둔다.
- HTML 에서 실패 지점과 책임 경계를 사용자가 확인할 수 있게 표시
- `.xavi/reports/development_cycles/latest.json` 이 최신 사이클 artifact 를 가리키게 갱신

## 시작 순서

1. 이 파일 `docs/agent/cycle-report/README.md` 를 먼저 읽고 역할 경계를 확정한다.
2. `orchestra` 가 넘긴 context bundle 만 기준으로 보고서 생성을 시작한다.
3. context bundle 의 필수 항목 누락을 먼저 검사한다.
4. 누락이 있어도 추정으로 채우지 않고 `audit.json` 과 `limitations` 에 남긴다.
5. 산출물을 지정 경로에 쓴 뒤, 생성 파일 목록과 `Context Report` 를 `orchestra` 에 반환한다.

## 실행 트리거

- `orchestra` 만 `cycle-report` 를 dispatch 한다.
- polling 으로 사이클 종료를 추정하지 않는다.
- 파일 변경, 프로세스 종료, trace idle 상태, 시간 경과만으로 직접 보고서를 만들지 않는다.
- `user-docs` 와 `ai-docs` 가 끝났거나, 사이클이 실패/중단/blocked 로 닫혀야 하는 close candidate 에 도달했을 때 `orchestra` 가 명시적으로 dispatch 한다.
- 보고서 artifact 생성 전에는 `orchestra` 가 사용자에게 사이클 완료라고 말하면 안 된다.
- 보고서 artifact 생성 직후에도 아직 1회 작업 사이클 완료가 아니다. `orchestra` 가 artifact readback, report server readiness, 해당 cycle URL 브라우저 open 까지 확인해야 완료 조건이 충족된다.
- report server readiness 또는 browser open 에서 발견된 결함은 같은 작업 cycle 의 결함이다. final report artifact 를 만들었다는 이유로 별도 후속 cycle 로 밀어내거나 완료라고 표현하지 않는다.
- dev-console/report server 는 이 역할이 만든 artifact 를 보여주는 viewer 이며, trace DB 에서 fallback 보고서를 생성하는 대체 경로가 아니다.

## 접근 허용 범위

- `docs/agent/cycle-report/README.md`
- `docs/agent/cycle-report/templates/`
- `orchestra` 가 전달한 context bundle
- `orchestra` 가 전달한 trace export, raw trace, audit 입력
- 보고서 산출물 경로 `.xavi/reports/development_cycles/<cycle_id>/`
- 최신 포인터 `.xavi/reports/development_cycles/latest.json`

## 접근 금지 범위

- 제품 코드 수정
- `docs/human/` 사용자 문서 작성 또는 갱신
- `docs/agent/ai-docs/` AI 인계 문서 작성 또는 갱신
- `docs/agent/user-docs/` 사용자 문서 역할 문서 작성 또는 갱신
- 다른 역할의 판단, 테스트, 리뷰, 구현을 대신 수행
- `orchestra` trace 원장이나 raw evidence 를 고쳐서 성공처럼 보이게 만들기
- 누락된 원문 evidence 를 기존 summary, display, report, 추정 문장으로 복원하거나 재생성하기

## 산출물 위치

모든 산출물은 아래 경로에 둔다.

```text
.xavi/reports/development_cycles/<cycle_id>/
├── index.html
├── report.json
├── raw.json
├── audit.json
└── context.md

.xavi/reports/development_cycles/latest.json
```

산출물 의미:

- `index.html`: 사용자가 브라우저에서 보는 정적 HTML 보고서
- `report.json`: 정규화된 사이클 요약 데이터
- `raw.json`: `orchestra` 가 넘긴 원본 context bundle 과 trace export 를 보존한 raw artifact
- `audit.json`: 필수 항목 존재 여부, 누락, 경고, 생성 실패 여부
- `context.md`: 다음 사람 또는 AI 가 빠르게 읽을 수 있는 사이클 context bundle
- `latest.json`: 최신 사이클 id, status, artifact 경로, 생성 시각 포인터

`index.html` 은 browser viewer 가 직접 열 수 있는 prebuilt artifact 다.
dev-console 의 canonical `/reports/<cycle_id>/` route 는 이 파일을 직접 서빙해야 하며,
큰 `report.json` 또는 raw artifact 를 다시 읽어 HTML 을 합성하다가 대형 artifact 오류로 실패하는 경로가 아니다.
prebuilt `index.html` 직접 서빙을 지원할 때는 reports root/cycle dir canonical containment,
`index.html` symlink 거부, required artifact symlink escape 거부가 함께 유지되어야 한다.

## cycle alias artifact 계약

`cycle_id` 는 canonical 식별자이고 report artifact 경로의 기준이다.
`cycle_alias` 는 사람이 회의에서 짧게 부르는 별칭이며, `category-NNN` 형식이어야 한다.
초기 템플릿의 예약 예시는 placeholder 만 사용한다: `<cycle-id>` -> `<category-NNN>`.

`cycle-report` 는 alias 원장을 새로 쓰는 역할이 아니다.
`orchestra` 가 `trace reserve-alias` 로 예약하고 `trace resolve-alias` 로 readback 한
context bundle 을 받아 `report.json`, `index.html`, `context.md`, `latest.json` 에
표시 필드로 반영한다.

alias 관련 report 필드:

- `cycle_alias`
- `cycle_category`
- `cycle_category_key`
- `cycle_sequence`
- `cycle_title`
- `artifacts.aliases_json`

`aliases.json` 은 report root 의 viewer/index projection 이다.
source of truth 는 SQLite `development_cycle_aliases` 원장이다.
`cycle-report` 는 malformed `aliases.json` 을 고치거나 재생성해서 성공처럼 만들지 않는다.
입력 bundle 에 alias 예약/readback 또는 `aliases_json` 경로가 누락되면
`audit.json` 의 `missing_required_inputs` 또는 `warnings` 에 기록한다.

by-alias route 는 dev-console viewer 책임이다.
`cycle-report` 는 `/reports/by-alias/<alias>/` 나
`/api/reports/by-alias/<alias>/<file>` 를 위해 trace DB 에서 fallback 보고서를
합성하지 않는다.

## 필수 context bundle

`orchestra` 는 `cycle-report` dispatch 때 아래 항목을 전달해야 한다.

- `cycle_id`
- `cycle_alias`: 예약된 alias. 아직 예약하지 못했으면 `null` 로 두고 audit 에 사유를 기록한다.
- `cycle_category`
- `cycle_category_key`
- `cycle_sequence`
- `cycle_title`
- `status`: `success`, `failure`, `interrupted`, `blocked`
- `user_request_verbatim`: schema 기준 canonical 사용자 최초 요청 원문 evidence. 원문이 없으면 `null` 로 두고 `audit.missing_evidence[]` 에 누락을 기록한다.
- `user_request_display_summary_ko`: schema 기준 canonical 사용자 요청 표시용 한국어 요약. 원문 evidence 가 아니라 derived/display projection 이다.
- `user_request`: legacy deprecated derived projection 이다. 호환 표시용으로만 둘 수 있고, 원문 필수 필드나 원문 evidence 로 요구하거나 표시하면 안 된다.
- `result_summary`: 사이클 결과 요약
- `orchestra_instruction`: `orchestra` 가 close hook 또는 실패 처리에서 내린 핵심 지시
- `role_returns`: 역할별 반환 요약을 role key 로 정규화한 객체
- `failure_point`: 실패, 중단, blocked 지점의 한 줄 요약. 성공이면 해당 없음으로 명시한다.
- `verification_result`: 실행한 검증과 결과의 한 줄 요약. 미실행이면 미실행 사유를 명시한다.
- `changed_files`: 변경 파일 경로 또는 변경 파일 요약 배열
- `evidence_status`: `complete`, `incomplete`, `fail` 중 하나. runtime append evidence 가 없으면 `incomplete` 또는 `fail` 이어야 한다.
- 코드 변경이 있었으면 `code_changes`: 이번 cycle 에서 생성, 수정, 삭제된 코드 hunk 배열
- 사용자 최초 요청, 정정, 승인, 보류 또는 중단 지시
- `planning` 초기 계획과 최종 완성도 보고
- 역할별 dispatch 입력, return 요약, 반환 검수 결과
- 역할별 `Context Report`
- 변경 파일 목록과 변경 의도
- 실행 명령, 검증 결과, 실패 명령, 핵심 로그. 명령 evidence 는 가능하면 `command_verbatim`, `result_verbatim`, `output_verbatim` 을 분리해 받는다.
- 실패한 경우 어디서 잘못됐는지에 대한 사실 기반 trace
- `development_trace` export 또는 참조
- alias reservation/readback 결과와 `aliases.json` artifact 참조
- `verbatim_evidence`: `user_request_verbatim`, `agent_dispatch.prompt_verbatim`, 가능한 completion result 원문 event 배열
- `orchestra` trace/export/mirror 참조
- audit 입력, 누락된 이벤트, 불일치 항목
- limitations
- 다음 판단 항목

## report.json 계약

`report.json` 은 브라우저 viewer 가 바로 읽는 top-level 요약 키와, 나중에 감사할 수 있는 구조화 evidence 를 함께 가진다.
schema-required top-level 키는 viewer 표시용 핵심 키보다 넓다.
아래 schema-required 목록은 `templates/report.schema.json` 의 top-level `required` 배열과 같은 순서로 유지한다.

schema-required top-level 키:

- `cycle_id`
- `status`
- `generated_at`
- `evidence_status`
- `user_request_verbatim`
- `user_request_display_summary_ko`
- `result_summary`
- `orchestra_instruction`
- `orchestra_delegations`
- `role_returns`
- `failure_point`
- `verification_result`
- `changed_files`
- `code_changes`
- `artifacts`
- `user`
- `planning`
- `roles`
- `failures`
- `files`
- `commands`
- `trace`
- `derived_summaries`
- `audit`
- `limitations`
- `next_decisions`

viewer-facing 핵심 키:

이 목록은 HTML 첫 화면과 viewer 요약에서 우선 노출할 핵심 표시 필드다.
schema-required 전체 목록이 아니며, report 생성자는 이 목록만 채우고 나머지 schema-required 키를 생략하면 안 된다.

- `user_request_verbatim`
- `user_request_display_summary_ko`
- `result_summary`
- `orchestra_instruction`
- `role_returns`
- `failure_point`
- `verification_result`
- `changed_files`
- `evidence_status`

legacy top-level 키:

- `user_request`: deprecated derived projection 이며 schema 의 canonical 원문 필드가 아니다. schema-required top-level 키도 아니다. 다음 `cycle-report` 작성자는 이 값을 원문 필수 필드로 요구하지 않는다.
- `orchestra_instruction`: schema-required 이지만 legacy derived projection 이다. 원문 지시 evidence 는 `orchestra_delegations[].prompt_verbatim` 에 둔다.

구조화 evidence/audit 키:

- `roles`: 역할별 dispatch, return, `Context Report`, `orchestra` 검수 결과 배열
- `failures`: 실패, 중단, blocked evidence 배열
- `files`: 파일별 actor, intent, status 배열
- `code_changes`: 파일별 코드 변경 hunk, 라인 배열, 한국어 설명 배열
- `commands`: 명령, actor, result, evidence 배열. 원문 필드가 있으면 `command_verbatim`, `result_verbatim`, `output_verbatim` 을 서로 독립된 evidence 로 둔다.
- `artifacts`: `index.html`, `report.json`, `raw.json`, `audit.json`, `context.md`, `latest.json` 경로 객체
- `user`: 사용자 최초 요청, 정정, 승인, 중단 또는 blocked 신호의 구조화 객체
- `planning`: 초기 계획, 최종 완성도 보고, 사용자 반응 객체
- `trace`: `development_trace`, `orchestra` trace, raw export 참조 객체
- `derived_summaries`: 원문 evidence 에서 파생된 display/summary projection 객체
- `audit`: 필수 입력, 원문 evidence, 파생 projection 검수 상태 객체
- `limitations`: 이번 보고서의 한계 배열
- `next_decisions`: 다음 판단 항목 배열

viewer-facing 요약 키는 표시를 위한 투영이고, schema-required 구조화 키는 감사와 재개를 위한 세부 근거다.
둘 중 하나만 채워서 다른 쪽을 비워 두지 않는다.
같은 사실을 표현할 때 두 표현은 서로 충돌하면 안 되며, 충돌하거나 입력이 부족하면 `audit.json` 의 `warnings` 또는 `missing_required_inputs` 에 남긴다.

### 원문 evidence 계약

`cycle-report` 는 원문 evidence 작성자가 아니다.
원문 evidence 는 trace DB export 또는 `orchestra` context bundle 로 받은 항목만 사용할 수 있고, 보고서 생성 중에 복원, 재생성, 재구성하지 않는다.
repo 코드가 Codex `spawn_agent` tool invocation 경계를 자동 interception 한다고 가정하지 않는다.
원문이 "DB에 저장된다" 고 표시하려면 `orchestra` 가 런타임 경계에서 `xavi-bootstrap trace append` 또는 동등 trace append 명령으로 append 한 event 가 실제로 있어야 한다.

필수 원문 event:

- `user_request_verbatim`: 사용자 프롬프트 수신 직후 저장된 원문
- `agent_dispatch.prompt_verbatim`: sub-agent spawn 직전 저장된 `orchestra` 지시문 원문
- completion result verbatim event: sub-agent 완료 직후 런타임이 제공한 result 원문

각 원문 event 는 아래 metadata 를 포함해야 한다.

- `source_ref`
- `hash_sha256`
- `timestamp`
- `order`
- `role`
- `agent_id`

metadata 가 없거나 원문 자체가 없으면 `audit.missing_evidence[]` 에 event 종류, 기대 source, 누락 metadata, 영향받는 화면 영역을 기록한다.
원문 evidence 가 누락된 사이클은 누락 범위에 따라 `audit.status` 를 `fail` 또는 `warn` 으로 설정하고 `evidence_status` 를 `incomplete` 또는 `fail` 로 설정한다.
HTML 첫 화면에는 원문 evidence 누락 경고를 표시하고, 성공처럼 보이는 표현으로 덮지 않는다.

Codex runtime transcript API 를 repo 코드가 직접 읽을 수 없는 현재 런타임에서는 `orchestra` 가 spawn 직전과 completion 직후 원문 bundle 을 trace DB 에 append 해야 한다.
그 bundle 도 없으면 원문 없음으로 표시한다.
v2 trace event 의 `source_ref` 와 `hash_sha256` 가 없는 prompt 는 원문처럼 표시하면 안 되며, 표시가 필요하면 `dispatch_summary_derived` 같은 파생 필드로만 보여준다.
기존 report, `role_returns`, `result_summary`, `orchestra` trace 요약, 화면 표시 문자열을 원문처럼 복원하거나 재생성하지 않는다.

파생 요약과 화면용 투영은 반드시 원문과 이름을 분리한다.
field 이름과 UI 라벨에는 `derived`, `summary`, `display` 중 하나를 포함해야 한다.
예를 들어 `request_summary`, `dispatch_summary_derived`, `role_result_display` 는 파생 데이터이고, `user_request_verbatim` 또는 `agent_dispatch.prompt_verbatim` 을 대체하지 않는다.

### viewer 원문 우선 계약

`index.html` 과 dev-console report viewer 는 사람이 먼저 확인해야 하는 근거를 원문 evidence 에서 출발해 보여준다.
첫 화면 또는 기본 펼침 영역은 아래 항목의 존재 여부와 누락 여부를 숨기지 않는다.

- 사용자 요청 원문과 role dispatch prompt 원문
- role completion/return 원문과 `Context Report`
- 테스트 명령, 결과, 출력 원문
- `report.json.code_changes[]` 의 전체 hunk/line evidence
- `audit.json.excluded_code_changes[]` 의 파일별 제외 사유
- `audit.missing_evidence[]`, `audit.missing_required_inputs`, `audit.status=fail|warn`, `evidence_status=incomplete|fail`

`summary`, `display`, `derived` 필드는 탐색을 돕는 보조 projection 이다.
보조 projection 은 원문 evidence 가 없을 때 원문처럼 승격하지 않는다.
viewer 가 화면을 줄여 보여줄 수는 있지만, 원문 evidence 누락을 요약으로 덮거나 성공처럼 보이게 하면 안 된다.

### command evidence 계약

명령 evidence 는 `command_verbatim`, `result_verbatim`, `output_verbatim` 을 각각 독립 표시한다.

- `command_verbatim`: 실행된 명령 문자열 또는 명령 원문 bundle
- `result_verbatim`: exit status, pass/fail 판정, runner 가 반환한 결과 원문
- `output_verbatim`: stdout/stderr/log 원문 또는 raw output 참조

세 필드는 서로 fallback 이 아니다.
한 필드가 비어 있다고 해서 다른 필드의 값을 복사하지 않고, 세 필드가 모두 존재하면 viewer 와 `report.json` 에 세 값을 모두 별도 evidence 로 표시한다.
과거 회귀였던 "첫 원문 필드만 표시" 동작은 실패로 취급한다.
다음 구현/검증 cycle 에서는 세 필드에 서로 다른 sentinel 값을 넣은 회귀 테스트로 HTML 또는 render model 에 세 값이 모두 남는지 확인한다.

### `code_changes` 계약

`codegen` 이 코드를 생성, 수정, 삭제했다면 `report.json.code_changes` 는 빈 배열이면 안 된다.
`changed_files` 는 요약이고, `code_changes` 는 사용자가 "모든 코드부분" 을 시각적으로 확인하기 위한 hunk 단위 evidence 다.
"모든 코드부분" 은 이번 cycle 에서 변경된 모든 코드 hunk 를 뜻한다.

각 `code_changes[]` 항목은 아래 정보를 포함한다.

- `file_path`: 저장소 기준 상대 경로. UI 에 표시할 파일명은 이 값을 기준으로 보여주며, 별도 파일명 전용 필드는 요구하지 않는다.
- `language`: 감지한 언어 또는 파일 형식
- `change_kind`: `added`, `modified`, `deleted`, `renamed`, `copied`, `mode_changed`, `unknown` 중 하나
- `author_roles`: 해당 변경을 만든 역할 배열
- `summary_ko`: 파일 변경의 한국어 요약
- `raw_diff_ref`: 원본 diff 또는 raw evidence 참조. 없으면 `null`
- `hunks[]`: 이번 cycle 에서 해당 파일에 생긴 모든 hunk 배열

각 `hunks[]` 항목은 아래 정보를 포함한다.

- `old_start`, `old_lines`, `new_start`, `new_lines`: hunk 의 이전/새 파일 라인 범위. 새 파일이나 삭제 파일처럼 한쪽 라인이 없으면 해당 start 는 `null` 이 될 수 있다.
- `heading`: hunk header 또는 의미 단위 제목
- `summary_ko`: 해당 hunk 의 한국어 요약
- `lines[]`: hunk 안의 라인 배열. 각 항목은 `kind`(`context`, `add`, `remove`), `old_line`, `new_line`, `content` 를 포함한다.
- `explanations[]`: 필요한 경우 hunk 안의 의미 단위별 한국어 설명. 각 항목은 `line_ref` 또는 `range_ref` 중 하나와 `text_ko` 를 포함한다.

이번 cycle 에서 변경된 모든 hunk 를 넣는다.
새 파일은 added hunk 로 전체 신규 내용을 보여준다.
삭제 파일은 삭제 hunk 로 제거된 내용을 보여준다.
rename/copy/mode change 처럼 content hunk 가 없을 수 있는 변경은 `change_kind` 로 표시하고, 실제 hunk 가 없으면 `hunks[]` 는 빈 배열로 둘 수 있다.
대형 파일, 바이너리 파일, 생성물, 비밀값 가능성이 있는 파일처럼 전체 hunk 를 싣기 위험한 경우에는 `code_changes` 에 축약 항목만 두고 `audit.json.excluded_code_changes` 에 `file_path`, `reason`, 필요한 경우 `evidence_ref`, `audit_json_key` 를 기록한다.
한국어 설명은 report layer annotation 이며, 실제 소스 코드에 설명용 한국어 주석을 강제로 삽입하지 않는다.

## 작성 원칙

- 사실과 추정을 분리한다.
- 사용자가 역할별 지시, 실제 수행, 반환 검수, 테스트/실패 지점을 시각적으로 비교할 수 있게 구성한다.
- 성공보다 실패 설명을 우선 누락 없이 남긴다.
- 실패한 경우 사용자가 무엇을 지시했고, `orchestra` 가 어떤 역할에 어떻게 지시했고, 어느 단계에서 잘못됐는지 HTML 첫 화면에서 확인 가능해야 한다.
- raw evidence 는 보존하고, 사람이 읽는 요약만 압축한다.
- hidden chain-of-thought 를 요구하거나 재구성하지 않는다.
- `development_trace` 와 `orchestra` trace 는 source of truth 로 취급하고, 보고서는 그 투영 artifact 로만 취급한다.
- source of truth 에 원문 evidence 가 없으면 보고서가 원문을 작성하지 않고 누락으로 표시한다.
- trace DB 는 raw evidence 참조일 뿐이며, context bundle 없이 trace DB 만 읽어 fallback 보고서를 합성하지 않는다.
- 사용자용 설명 문서는 `user-docs`, 다음 AI 인계 문서는 `ai-docs`, 사이클 사실 artifact 는 `cycle-report` 가 맡는다.

## 실패 처리

- context bundle 이 부족하면 보고서 생성을 중단하지 말고 가능한 artifact 를 만들되, `audit.json` 에 `missing_required_inputs` 를 기록한다.
- 원문 evidence 가 부족하면 `audit.missing_evidence[]` 를 기록하고 `audit.status` 를 `fail` 또는 `warn` 으로 설정한다.
- runtime append evidence 가 없으면 `evidence_status=incomplete|fail` 이어야 하며, `source_ref` 또는 `hash_sha256` 가 없는 prompt 를 원문처럼 표시하면 보고서 실패로 취급한다.
- `index.html` 또는 `report.json` 을 만들 수 없으면 실패 사실, 가능한 원인, 생성된 부분 산출물, 미생성 파일을 `orchestra` 에 반환한다.
- `orchestra` 는 `cycle-report` 실패 시 직접 대체 보고서를 쓰지 않는다.
- `orchestra` 는 실패 자체를 `development_trace` 와 trace/export/mirror 에 남기고 사용자에게 보고해야 한다.
- `cycle-report` 는 실패를 감추거나 사이클을 성공으로 보정하지 않는다.
- code hunk evidence 를 만들 수 없는 파일이 있으면 성공처럼 조용히 생략하지 않고 `audit.json.excluded_code_changes` 또는 `missing_required_inputs` 에 남긴다.

## 템플릿

- `templates/context.md`: context bundle 입력 템플릿
- `templates/report.schema.json`: `report.json` 검증 스키마
- `templates/index.html`: 정적 HTML 보고서 템플릿

## Context Report

작업 종료 응답에는 아래 항목을 포함한다.

- `context_level`: `low`, `medium`, `high`, `near-limit`
- `basis`: 판단 근거
- `read_files`: 실제로 읽은 주요 파일
- `generated_artifacts`: 생성 또는 갱신한 artifact 경로
- `missing_inputs`: 누락된 필수 입력
- `carryover_summary`: 다음 역할이 이어받아야 할 요약
- `recommended_action`: `continue` 또는 `close_and_respawn`
