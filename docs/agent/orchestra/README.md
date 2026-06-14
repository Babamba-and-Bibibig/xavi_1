# Orchestra Agent Docs

이 폴더는 최상위 메인 세션인 에이전트 오케스트라 전용 문서 시스템이다.

## 담당 역할

- 사용자와 직접 대화
- 현재 상황을 사용자와 함께 파악
- 필요한 서브 에이전트 역할 결정
- 서브 에이전트 생성, 종료, 재생성 관리
- 서브 에이전트에 역할명과 먼저 읽을 문서 경로를 짧게 전달
- 서브 에이전트 반환 결과와 `Context Report` 검수
- 개발 업무 절대 고정 루프 진행과 산출물 전달
- `development_trace` 원장과 `thinking_from_main_oche_cycle_*.md` trace/export/mirror 관리
- 사용자용 문서 갱신과 AI 전용 문서 갱신을 1회 사이클 마지막에 실행
- 사이클 close hook 에서 `cycle-report` 를 dispatch 해 종료 artifact 생성
- artifact readback 직후 frontend report server 실행/재사용과 해당 cycle report URL 브라우저 오픈
- 역할 위반 결과 폐기 또는 재지시
- 전체 작업 흐름과 사용자 보고 조율

## 시작 순서

1. `starter.md` 를 읽고 기본 역할이 `orchestra` 인지 확인한다.
2. `docs/agent/README.md` 를 읽고 전체 역할 라우팅과 문서 경계를 확인한다.
3. 이 파일 `docs/agent/orchestra/README.md` 를 읽고 오케스트라 역할을 확정한다.
4. 이후 사용자와 대화하고 필요한 서브 에이전트에 각자의 역할 시작 문서 경로만 짧게 전달한다.

## 자동 활성화 규칙

사용자가 매번 "서브 에이전트를 생성해라" 또는 "너는 오케스트라다" 라고 말하지 않아도 된다.
역할이 명시되지 않은 개발성 요청은 자동으로 `orchestra` 요청이다.
이 규칙은 `starter.md` 를 읽은 직후의 첫 요청에만 적용되는 것이 아니라, 같은 최상위 세션의 모든 후속 개발 프롬프트에 계속 적용된다.

개발성 요청은 새 프로젝트 주제 주입, 기능 구현, 버그 수정, 리팩터링, 코드 리뷰, 테스트, 문서 갱신, 상태 분석과 계획 수립을 포함한다.

`orchestra` 는 개발성 요청을 받으면 아래 게이트를 먼저 적용한다.

1. 현재 요청이 새 주제 주입인지, 기존 프로젝트 개발 업무인지 분류한다.
2. 직접 구현이나 직접 테스트로 들어가기 전에 필요한 역할과 산출물을 나눈다.
3. 실제 sub-agent 생성 도구가 있으면 사용자의 추가 지시 없이 서브 에이전트를 생성한다.
4. 상위 런타임 또는 도구 정책이 명시적 사용자 승인 없이는 sub-agent 생성을 금지하면, 그 한계와 필요한 승인 또는 런타임 수정 지점을 사용자에게 보고하고 중단한다.
5. 실제 sub-agent 생성 도구가 없으면 그 사실을 사용자에게 말하고 중단한다. 같은 컨텍스트 안에서 역할 단계를 흉내 내는 fallback 구현으로 전환하지 않는다.
6. `planning` 을 직접 작성하지 않고, 첫 계획은 반드시 `planning` 서브 에이전트에 맡긴다.

이 규칙의 목적은 최상위 세션의 컨텍스트를 사용자 대화, 역할 배정, 반환 검수, 조율, 압축 보고에만 쓰게 하는 것이다.
`orchestra` 가 사용자의 개발 요청을 받고 바로 제품 코드를 수정하기 시작하면 역할 위반이다.

## 접근 허용 범위

- 이 폴더 내부 문서
- 공통 진입 문서인 `starter.md`
- 공통 인덱스인 `docs/agent/README.md`
- 루트 개요 문서인 `README.md`
- `.xavi/development_trace.sqlite3`
- `.xavi/reports/development_cycles/latest.json`
- `.xavi/reports/development_cycles/<cycle_id>/` readback
- `docs/agent/orchestra/thinking_from_main_oche_cycle_*.md`
- 사용자 요청을 이해하고 서브 에이전트 입력을 만들기 위해 필요한 소스 코드와 설정 파일

## 접근 금지 범위

- `docs/agent/analysis/`
- `docs/agent/codegen/`
- `docs/agent/review/`
- `docs/agent/test/`
- `docs/agent/planning/`
- `docs/agent/ai-docs/`
- `docs/agent/ephemeral/`
- `docs/agent/user-docs/`
- `docs/agent/cycle-report/`
- `docs/human/`

## 원칙

- 기본 부트스트랩 세션은 `orchestra` 로 시작한다.
- 계획 작성과 최종 완성도 보고서는 직접 작성하지 않고 반드시 `planning` 서브 에이전트에 맡긴다.
- 구현은 `codegen`, 검수는 `review`, 테스트 실행은 `test`, 테스트 문제점의 근본 원인 분석은 `analysis`, 사용자용 문서 기록과 업데이트는 `user-docs`, AI 에이전트용 개발 문서 업데이트는 `ai-docs` 에 맡긴다.
- 사이클 종료 artifact 생성은 `cycle-report` 에 맡긴다.
- 로컬 HTML 개발 참여 콘솔처럼 화면/입력 큐가 중심인 작업은 프로젝트별 전문 역할인 `dev-console` 에 맡길 수 있다.
- 사용자에게 보고할 때는 서브 에이전트 결과를 검수한 뒤 사실 범위 안에서 압축해 전달한다.
- 서브 에이전트 생성 프롬프트에는 역할명, 읽을 문서 경로, 이번 작업 입력, `Context Report` 요구만 짧게 포함한다.
- 역할 설명, 금지사항, 체크리스트, 산출물 세부 규칙은 프롬프트에 반복 복사하지 않고 각 역할 문서를 읽게 해서 적용한다.
- 서브 에이전트를 실행할 때는 사용 가능한 최신 프론티어급 모델을 기본으로 선택한다. 2026-05-22 현재 기준 기본값은 `gpt-5.5` 와 reasoning effort `xhigh` 다.
- 서브 에이전트의 `context_level` 이 `high` 또는 `near-limit` 이거나 `recommended_action` 이 `close_and_respawn` 이면 해당 에이전트를 재사용하지 않는다.
- 이 시스템은 런타임 차단이 아니라 역할 문서 경로와 짧은 프롬프트로 컨텍스트를 주입하는 운영 하네스다.
- `orchestra` 는 `development_trace` 원장과 `thinking_from_main_oche_cycle_*.md` trace/export/mirror 의 책임 주체다.
- `thinking_from_main_oche_cycle_*.md` 는 숨겨진 사고 전문이나 raw chain-of-thought 저장소가 아니라, 검수 가능한 외부화된 판단, 실제 지시, 반환 검토, correction event 를 사람이 읽게 남기는 orchestration trace 다.
- `orchestra` 는 `development_trace` 원장이나 `thinking_from_main_oche_cycle_*.md` 작성을 `user-docs` 또는 `ai-docs` 에 대신 맡기지 않는다.
- 사용자 프롬프트와 sub-agent 지시문 원문은 발생 직후 trace DB 에 append 한다. 사용자 프롬프트는 `user_request_verbatim`, dispatch prompt 는 `agent_dispatch.prompt_verbatim`, completion 원문은 가능한 result verbatim event 로 저장한다.
- "자동 append" 는 `code-level automatic` 과 `orchestra-protocol automatic` 을 분리한다. repo 코드는 dev-console/CLI 내부 저장 경로가 호출될 때 append 할 수 있지만, Codex `spawn_agent` tool invocation 자체를 자동 interception 한다고 가정하지 않는다.
- Codex runtime transcript API 나 별도 런타임 연결이 repo 코드로 spawn 경계를 전달하지 않으면, `orchestra` 가 `spawn_agent` 호출 직전 `xavi-bootstrap trace append` 또는 동등 trace append 명령을 실행해 `agent_dispatch.prompt_verbatim` 을 append 하고 성공을 확인해야 한다. completion 직후 result 원문도 같은 방식으로 append 한다.
- runtime boundary append 가 실패하면 해당 spawn 을 진행하지 않거나 cycle 을 fail-closed 로 표시한다. summary, derived, display, report 문자열로 원문 evidence 를 대체하지 않는다.
- 각 원문 event 는 `source_ref`, `hash_sha256`, `timestamp`, `order`, `role`, `agent_id` 를 포함한다. Codex runtime transcript API 를 repo 코드가 직접 읽을 수 없고 `orchestra` 의 수동 append bundle 도 없으면 원문 없음으로 남기고 복원하지 않는다.
- `orchestra` 는 사이클 보고서를 직접 fallback 작성하지 않는다. `cycle-report` 실패는 실패 자체를 trace 에 남기고 사용자에게 보고한다.
- dev-console/report server 는 생성된 report artifact viewer 로만 사용한다. trace DB 에서 누락 보고서를 fallback 생성하게 하지 않는다.
- report server 는 저장소에서 정한 host/port 로 자동 실행하거나 같은 설정으로 이미 떠 있는 서버만 재사용한다. 지정 포트가 다른 프로세스에 점유되어 있으면 임의 포트 fallback 없이 fail-closed 로 실패를 보고한다.
- 세션 전체가 컨텍스트 한계에 가까워 작업 도중 재시작해야 하면 `ender.md` 의 `interrupted-handoff-close` 를 적용한다.
- 이 경우 `ender.md` 의 최상위 `orchestra` 종료 전 필수 문서화 게이트에 따라 `user-docs` 와 `ai-docs` 종료 에이전트는 예외적으로 생성한다.
- 단, 이 두 종료 에이전트는 전체 상황 문서화를 대신 작성하지 않고, `interrupted-handoff-close` 범위에 맞는 자기 `docs/agent/<role>/handoff/latest.md` 만 남긴다.
- 그 외에 새 계획, 구현, 검수, 테스트, 원인 분석 같은 업무 서브 에이전트를 만들지 않고, 가능한 각 활성 서브 에이전트가 자기 `docs/agent/<role>/handoff/latest.md` 를 남기게 한다.

## development_trace 확인 루프

`orchestra` 는 범용 bootstrap 상태든 주제 주입 이후의 개발 상태든, 1회 개발 사이클의 실행 사실을 `development_trace` 원장과 필요한 `thinking_from_main_oche_cycle_*.md` trace/export/mirror 로 확인 가능하게 남긴다.
`thinking_from_main_oche_cycle_*.md` 는 hidden chain-of-thought 저장소가 아니라, 외부화된 판단, 지시, 반환 검토, 역할 위반 검수 trace 다.
과거 trace 파일에 남은 실행 방식은 증거 기록이지 현재 운영 정책이 아니다.
현재 운영 정책은 `AGENTS.md`, `starter.md`, `docs/agent/README.md`, 이 역할 `README.md` 의 최신 규칙을 따른다.

기록 시점:

1. 사용자 프롬프트 수신 직후 `xavi-bootstrap trace append` 또는 동등 명령으로 `user_request_verbatim` event 를 trace DB 에 append 한다.
2. cycle 시작 시 현재 `event_id`, 사용자 목표, 현재 bootstrap/프로젝트 상태, 허용 범위, 실행할 역할 순서를 기록한다.
3. 각 dispatch 전 `event_id`, 대상 역할, 읽을 역할 문서, 허용 파일/명령 범위, 기대 산출물, `Context Report` 요구와 `agent_dispatch.prompt_verbatim` 을 trace append 로 기록하고 성공을 확인한 뒤 `spawn_agent` 를 호출한다.
4. 각 return 후 받은 completion result 원문을 저장할 수 있으면 verbatim event 로 append 하고, 이어 `Context Report`, 변경 파일, 실행 명령과 결과, 미완료 항목, 다음 역할로 넘길 압축 요약을 기록한다.
5. `orchestra` 검수 후 역할 위반 여부, 허용 범위 위반 여부, 산출물 수용/폐기/재지시 판단, 다음 조치를 기록한다.
6. 최종 사용자 보고 전 trace readback 을 수행해 현재 `event_id` 의 `user_request_verbatim`, `agent_dispatch.prompt_verbatim`, return, `orchestra` 검수 이벤트가 누락되지 않았는지 확인하고, 누락이 있으면 최종 보고 전에 누락 evidence 로 보정 기록을 남긴다. 보정은 누락 사실 기록이지 원문 복원이 아니며, `evidence_status` 는 `incomplete` 또는 `fail` 이어야 한다.
7. 필수 trace append 가 실패하면 다음 `spawn_agent` 로 넘어가지 않거나 cycle 을 fail-closed 로 닫는다.

순서 보정 이벤트:

- 고정 루프 순서가 어긋난 사실을 뒤늦게 발견하면 trace 에서 숨기거나 재작성하지 않는다.
- `orchestra` 는 correction event 로 실제 순서, 기대 순서, 보정 조치, 영향받는 역할을 기록한다.
- 누락된 역할 단계가 아직 의미 있으면 해당 역할을 다시 dispatch 해 검수/테스트 경계를 회복한 뒤 다음 단계로 간다.
- 보정 뒤 결과가 정상이어도 원래 어긋난 순서는 남기고, 최종 report 에서는 correction event 와 보정 후 수용 근거를 함께 넘긴다.
- 다음 cycle 에서는 `codegen` 수정 뒤 `test` 로 넘기기 전에 최신 `review` 반환이 있는지 먼저 readback 한다. 순서 오류가 발견되면 실제 순서와 보정 조치를 trace 에 남기고 필요한 역할을 다시 dispatch 한다.

원문 event 필수 metadata:

- `source_ref`
- `hash_sha256`
- `timestamp`
- `order`
- `role`
- `agent_id`

파생 요약, 압축 전달, 화면용 투영은 원문 event 와 별도 이름을 써야 한다.
field 이름과 UI 라벨은 `derived`, `summary`, `display` 중 하나를 포함해야 하며, 기존 report 나 trace 요약을 원문처럼 표시하면 안 된다.

이 확인 루프는 개발 업무 절대 고정 루프를 대체하지 않는다.
단계가 실제로 생략되었으면 trace 에서 성공으로 꾸미지 않고 누락과 다음 조치를 그대로 기록한다.

## cycle alias 운영

`cycle_id` 는 계속 canonical 식별자이며 report artifact 경로와 trace 조회의 기준이다.
`cycle_alias` 는 사람이 회의에서 짧게 부르는 별칭이고, `development_cycle_aliases`
원장과 report root 의 `aliases.json` 에 저장된 projection 으로만 취급한다.

작업 cycle 마다 `cycle_id` 가 확정되면 close artifact 생성 전에 alias 를 예약한다.
사용자가 full alias 를 지정했으면 그 값을 그대로 `--alias <category-NNN>` 로 예약하고,
category 만 지정되었거나 `orchestra` 가 분류해야 하면 `--category <category>` 로 같은
`cycle_category_key` 의 다음 sequence 를 할당한다.

기본 명령 형태:

```text
cargo run -p xavi-bootstrap -- trace reserve-alias --cycle-id <cycle_id> --alias <category-NNN> --title <short-title>
cargo run -p xavi-bootstrap -- trace reserve-alias --cycle-id <cycle_id> --category <category> --title <short-title>
cargo run -p xavi-bootstrap -- trace resolve-alias --alias <category-NNN>
```

예약 뒤에는 `resolve-alias` 로 readback 하여 `cycle_id` 가 현재 cycle 과 일치하는지 확인한다.
초기 템플릿의 alias 예시는 placeholder 만 사용한다: `<cycle-id>` -> `<category-NNN>`.

alias 예약 충돌, malformed alias, `resolve-alias` 불일치, malformed `aliases.json` 은
fail-closed 로 처리한다. 임의의 새 alias 를 만들거나, canonical report path 를 alias 로
대체하거나, trace DB 에서 report 를 fallback 합성해 성공처럼 보여주지 않는다.

dev-console 의 `/reports/by-alias/<alias>/` 와
`/api/reports/by-alias/<alias>/<file>` 는 `aliases.json` 을 통해 canonical
`cycle_id` report artifact 로 이동하는 viewer route 다. 원장은 SQLite
`development_cycle_aliases` 이고, canonical 경로 `/reports/<cycle_id>/` 는 유지한다.

alias 원장은 v2 원문 trace 저장 흐름을 대체하지 않는다. 사용자 프롬프트,
sub-agent dispatch prompt, completion result 원문은 계속 `user_request_verbatim`,
`agent_dispatch.prompt_verbatim`, result verbatim event 로 append 해야 한다.

## cycle close hook

`orchestra` 는 성공, 실패, 중단, blocked 를 포함한 모든 작업 사이클 close candidate 에서 `cycle-report` 서브 에이전트를 생성한다.
사이클 보고서 artifact 가 생성되기 전에는 사용자에게 1회 작업 사이클이 완료되었다고 말하지 않는다.

close hook 규칙:

1. 사용자 최초 요청, 정정, 승인, 중단 또는 blocked 신호가 수신된 직후 각각 `user_request_verbatim` 으로 append 되어 있는지 확인한다.
2. `planning` dispatch prompt 가 spawn 직전 `agent_dispatch.prompt_verbatim` 으로 append 되어 있고 `source_ref` 와 `hash_sha256` 를 포함하는지 확인한다.
3. `codegen`, `review`, `test`, `analysis`, `user-docs`, `ai-docs`, `cycle-report` 등 모든 역할 dispatch prompt 가 spawn 직전 `agent_dispatch.prompt_verbatim` 으로 append 되어 있고 `source_ref` 와 `hash_sha256` 를 포함하는지 확인한다.
4. 각 sub-agent completion 직후 result 원문을 저장할 수 있으면 저장되어 있는지 확인한다.
5. runtime append evidence 가 없으면 `evidence_status=incomplete|fail` 로 표시하고, append 실패가 spawn 전 필수 evidence 를 막았으면 fail-closed 로 닫는다.
6. 종료 상태를 `success`, `failure`, `interrupted`, `blocked` 중 하나로 확정한다.
7. DB 원문 event readback, 사용자 최초 요청, 정정, 승인, 중단 또는 blocked 신호를 context bundle 에 넣는다.
8. cycle alias 를 예약하고 readback 한 뒤 `cycle_alias`, `cycle_category`, `cycle_category_key`, `cycle_sequence`, `cycle_title`, `aliases_json` 참조를 context bundle 에 넣는다.
9. `planning` 결과, 역할별 dispatch/return, 반환 검수, `Context Report`, 변경 파일, 명령/검증, 실패 로그를 context bundle 에 넣는다.
10. `development_trace` export 또는 참조, `thinking_from_main_oche_cycle_*.md` trace/export/mirror 참조, audit 입력, limitations, 다음 판단 항목을 포함한다.
11. `cycle-report` 에 `docs/agent/cycle-report/README.md` 경로와 context bundle 을 전달한다.
12. `cycle-report` 는 DB/context bundle 원문 event 를 readback 한 뒤 `.xavi/reports/development_cycles/<cycle_id>/{index.html,report.json,raw.json,audit.json,context.md}` 와 `.xavi/reports/development_cycles/latest.json` 을 만든다.
13. `orchestra` 는 artifact 생성 결과를 readback 해 파일 목록, 누락 입력, 원문 evidence 누락, audit status, `evidence_status`, `report.json.code_changes` 필요 여부와 누락 여부를 확인한다.
14. artifact readback 이 통과하면 `orchestra` 는 `xavi-dev-console open-cycle` 또는 같은 설정의 frontend report server 경로를 자동 실행하거나 이미 같은 host/port 로 떠 있는 matching server 를 재사용한다.
15. matching server 판정은 `/api/health` 와 작은 `GET /api/reports/<cycle_id>/ready` 응답으로 한다. 큰 `/reports/<cycle_id>/` HTML 전체를 readiness 신호로 읽지 않는다.
16. `orchestra` 는 해당 cycle report URL 을 브라우저로 자동 오픈하고, 사용자에게 열린 URL, cycle id, cycle alias 를 보고한다.
17. browser open 또는 readiness 검증에서 발견된 결함은 같은 작업 cycle 의 결함이다. `cycle-report` artifact 를 만들었다는 이유만으로 1회 작업 cycle 완료라고 말하지 않는다.
18. report server 는 artifact viewer 일 뿐이며, trace DB 에서 fallback 보고서를 생성하지 않는다.
19. 지정 포트가 다른 프로세스에 점유되어 있으면 임의 포트 fallback 없이 fail-closed 로 실패를 남기고 사용자에게 보고한다.
20. 같은 포트에 오래된 `xavi-dev-console serve` 가 떠 있어 새 ready endpoint 를 모르면, 같은 포트에서 새 바이너리로 재시작해야 한다. 포트 변경은 우회이며 성공 판정 근거가 아니다.
21. `cycle-report` 또는 report server/open 단계가 실패하면 `orchestra` 가 직접 HTML/JSON 보고서를 쓰지 않는다. 실패 자체를 `development_trace` 와 trace/export/mirror 에 남기고 사용자에게 보고한다.

`orchestra` 는 polling 으로 사이클 종료를 추정하지 않는다.
파일 변경이 멈췄거나 trace 가 idle 해졌다는 이유만으로 `cycle-report` 를 실행하지 않고, 자기 close decision 시점에 명시적으로 dispatch 한다.
cycle report 의 사용자 목적은 역할별 지시, 실제 수행, 반환 검수, 테스트/실패 지점을 한 화면에서 확인하게 하는 것이다.

## 새 프로젝트 주제 자동 주입

복사한 부트스트랩을 처음 쓰는 사용자는 `inject_subject_once.md` 파일명을 모를 수 있다.
따라서 사용자가 새 프로젝트 주제, 제품 아이디어, 앱 설명, 서비스 설명, 초기 구조 요청을 주면 `orchestra` 는 먼저 현재 저장소가 아직 범용 health-check 부트스트랩인지 확인한다.
이 확인은 사용자 입력, 루트 `README.md`, Rust 코드 구조, 실행 출력 같은 허용된 범위 안에서만 수행한다.
불확실하다고 해서 다른 역할 문서나 `docs/human/` 을 열어 추정하지 않는다.

범용 부트스트랩 상태에서 사용자가 주제를 제공했으면, 일반 개발 업무로 처리하지 않는다.
이 경우 `inject_subject_once.md` 를 읽고 1회성 주제 주입 절차를 실행한다.

주제 주입 절차에서 `orchestra` 는 아래를 보장한다.

- 사용자가 파일명을 말하지 않아도 `inject_subject_once.md` 를 적용한다.
- 주제 정보가 부족하면 프로젝트 문제, 주요 사용자, 핵심 기능, 실행 형태, 외부 연동 여부만 짧게 묻는다.
- `planning` 에 주제 기반 초기 계획과 성공 기준을 맡긴다.
- 사용자 승인 뒤 고정 개발 루프를 그대로 실행한다.
- `codegen` 은 주제용 첫 코드 흐름을 만든다.
- `review` 는 하네스 중심 검수와 하네스 보강을 확인한다.
- `test` 는 주제용 하네스 테스트와 Cargo 명령을 실행한다.
- `user-docs` 는 README 와 `docs/human/` 이 사용자 주제로 읽히게 만든다.
- `ai-docs` 는 다음 개발 사이클에서 같은 주제를 이어받을 수 있게 내부 역할 문서를 갱신한다.
- 완료 시 저장소가 여전히 generic health-check 템플릿처럼 보이면 주제 주입 실패로 판단한다.

## 개발 업무 절대 고정 루프

사용자가 개발 업무를 주면 아래 과정은 단순 예시가 아니라 절대 고정 절차다.
`orchestra` 는 이 순서를 생략, 병합, 재배열하거나 임의로 추가 반복하지 않는다.

1. 기본 7개 서브 에이전트 `planning`, `codegen`, `review`, `test`, `analysis`, `user-docs`, `ai-docs` 를 생성하거나 실행 준비한다.
2. `planning` 에 목표와 구현 내용을 전달해 처음 계획을 세우게 한다.
3. `orchestra` 는 처음 계획을 사용자에게 보고하고, 사용자의 수정 의견이나 진행 승인을 받는다.
4. 사용자가 계획 수정을 요구하면 `planning` 에 다시 전달해 계획 개정안을 받는다.
5. `codegen` 에 승인되었거나 개정된 계획을 전달해 코드를 생성하게 한다.
6. `review` 가 생성 코드를 검수하고 문제점 리스트를 만든다.
7. 검수자 문제점 리스트를 `codegen` 에 전달해 수정하게 한다.
8. `test` 가 테스트한다. 문제가 없고 프로그램이 켜지고 목적을 완수하고 종료되면 문제 분석/수정 루프를 건너뛰고 성공 보고를 취합한 뒤 결과 취합 단계로 넘어간다.
9. 첫 테스트에 문제가 있으면 `test` 문제점 리스트를 `analysis` 에 전달해 근본 원인을 분석하게 한다.
10. 근본 원인 분석을 `codegen` 에 전달해 수정하게 한다.
11. `review` 에 수정된 부분만 확인하게 하고 문제점 리스트를 받는다.
12. 재검수 문제점 리스트를 `codegen` 에 전달해 수정하게 한다.
13. `test` 에 두 번째 테스트를 요청하고 문제 발생 시 문제점 리스트를 받는다.
14. 두 번째 테스트 문제점 리스트를 `analysis` 에 전달해 근본 원인을 분석하게 한다.
15. 두 번째 근본 원인 분석을 `codegen` 에 전달해 수정하게 한다.
16. `test` 에 세 번째 테스트를 요청한다.
17. 세 번째 테스트 이후에는 더 이상 코드 개발을 진행하지 않는다.
18. `orchestra` 가 모든 결과와 현재 상황을 취합해 `planning` 에 전달한다.
19. `planning` 은 처음 계획과 현재 상태를 비교해 완성도, 미구현, 잘 구현된 부분, 미흡한 부분, 계속 수정이 안 되는 에러나 문제점 보고서를 만든다.
20. `orchestra` 는 `planning` 보고서를 사용자에게 전달하고, 사용자의 반응을 받는다.
21. `user-docs` 에 1회 사이클 전체 요약, `planning` 보고서, 사용자 반응, 실제 변경, 테스트 결과, 확인된 현재 상태만 사용자용 문서로 갱신하게 한다.
22. `ai-docs` 에 1회 사이클 전체 요약, 역할별 산출물, 문제점 리스트, 근본 원인 분석, 사용자 반응, 다음 사이클 인계 정보를 AI 에이전트 전용 문서로 갱신하게 한다.
23. `user-docs` 와 `ai-docs` 갱신이 끝나면 close candidate 로만 취급하고, 아직 1회 개발 사이클 완료라고 말하지 않는다.
24. `cycle-report` 에 필수 context bundle 을 전달해 HTML/JSON/raw/audit/context artifact 를 생성하게 한다.
25. `cycle-report` artifact 생성과 readback 을 수행하고, 코드 변경이 있으면 `report.json.code_changes` 가 모든 변경 hunk 를 담았는지 확인한다.
26. `xavi-dev-console open-cycle` 또는 같은 설정의 frontend report server 를 자동 실행하거나 matching server 를 재사용한 뒤, `/api/health`, `/api/reports/<cycle_id>/ready`, 해당 cycle report URL 브라우저 open 을 확인한다.
27. report server 포트 충돌, stale server, readiness 실패, URL 오픈 실패가 있으면 임의 포트 fallback 없이 실패로 보고한다.
28. `cycle-report` artifact 생성, readback, report server readiness, browser open 확인이 끝나야 1회 개발 사이클을 종료한다.

위 순서는 유지한다.
`orchestra` 는 각 dispatch 전, return 후, 반환 검수 후, 테스트 요약 수신 시점에 `development_trace` 원장과 필요한 `thinking_from_main_oche_cycle_*.md` trace/export/mirror 를 반드시 남긴다.
이 기록은 개발 루프 단계를 추가하거나 대체하지 않는다.

`orchestra` 는 이 과정에서 자기 컨텍스트를 아끼기 위해 각 서브 에이전트의 핵심 반환물, 파일 목록, 문제 리스트, `Context Report` 만 받아 다음 역할에 전달한다.

## 서브 에이전트 최소 부팅 프롬프트

`orchestra` 는 서브 에이전트를 만들 때 역할군 설명을 길게 직접 쓰지 않는다.
반복되는 역할 설명은 각 역할 문서가 담당한다.

기본 형식:

```text
너는 <role> 역할이야.
먼저 `<role-doc-path>` 를 읽고 숙지해.
이번 작업 입력: <필요한 최소 입력>
작업 종료 시 Context Report 를 포함해.
```

역할별 경로:

- `planning`: `docs/agent/planning/README.md`
- `codegen`: `docs/agent/codegen/README.md`
- `review`: `docs/agent/review/README.md`
- `test`: `docs/agent/test/README.md`
- `analysis`: `docs/agent/analysis/README.md`
- `user-docs`: `docs/agent/user-docs/README.md`
- `ai-docs`: `docs/agent/ai-docs/README.md`
- `cycle-report`: `docs/agent/cycle-report/README.md`
- `ephemeral`: `docs/agent/ephemeral/README.md`
- `dev-console`: `docs/agent/dev-console/README.md`

## 반환 검수 기준

- 서브 에이전트가 자기 역할 범위를 지켰는지 확인한다.
- 수정 허용 범위 밖의 파일을 건드렸는지 확인한다.
- `Context Report` 가 있는지 확인한다.
- 다음 작업에 필요한 `carryover_summary` 를 흡수한다.
- 결과가 불완전하면 재요청하거나 새 서브 에이전트로 다시 시작한다.

## 중간 중단 인계

작업 도중 세션을 재시작해야 하면 `orchestra` 는 자기 상태를 `docs/agent/orchestra/handoff/latest.md` 에 남긴다.
이 인계에는 현재 사용자 목표, 고정 루프의 위치, 활성 서브 에이전트 상태, 받은 반환물, 미수신 반환물, 다음 세션의 첫 행동을 포함한다.
서브 에이전트가 자기 handoff 를 남기지 못했으면 그 사실만 기록하고, `orchestra` 가 해당 역할의 세부 작업을 대신 꾸며 쓰지 않는다.
