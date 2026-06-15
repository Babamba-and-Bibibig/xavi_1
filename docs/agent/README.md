# Agent Documentation Systems

이 디렉터리는 내부 AI 열 기본 역할과 프로젝트별 전문 역할을 위한 문서 시스템 루트다.
루트 `AGENTS.md` 는 자동 로드 가능한 환경에서 `starter.md` 로 들어오기 전의 짧은 부팅 계약 역할을 한다.
최상위 `orchestra` 세션은 `starter.md` 를 읽은 뒤 이 파일을 읽고 전체 역할 경로를 확인한다.
`orchestra` 가 생성한 서브 에이전트는 `starter.md` 를 다시 읽지 않고, 배정된 자기 역할 폴더의 `README.md` 를 첫 문서로 바로 읽는다.
세션을 마무리할 때는 루트의 `ender.md` 를 현재 역할 기준 종료 규약으로 적용해야 한다.
`ender.md` 는 정상 사이클 종료와 중간 중단 인계 종료를 먼저 분류한다.

## 내부 역할

- `orchestra`: 최상위 사용자 대화, 서브 에이전트 관리, 반환 검수 전담
- `planning`: 초기 계획 초안과 최종 완성도 보고서 전담
- `analysis`: 테스트 문제점의 근본 원인 분석 전담
- `codegen`: clean architecture 를 유지하는 제품 코드 생성 전담
- `review`: 생성된 코드 검수와 하네스 기반 테스트 코드 작성/보강 전담
- `test`: 검증 명령 실행, 문제점 리스트 작성, 재테스트 전담. 테스트 코드는 직접 작성하지 않음
- `ai-docs`: AI 에이전트 전용 개발 문서와 인계 정보 관리 전담
- `user-docs`: 사용자 문서 전담 역할의 에이전트 진입점과 사용자 문서 갱신 전담
- `cycle-report`: 작업 사이클 종료 직후 HTML/JSON/raw/audit/context artifact 생성 전담
- `ephemeral`: 임시 테스트포스 전담

## 프로젝트별 전문 역할

- `dev-console`: `development_trace` 기반 로컬 HTML 개발 참여 콘솔과 입력 큐 전담

## 문서/컨텍스트 규칙

- 최상위 `orchestra` 는 `starter.md` → `docs/agent/README.md` → `docs/agent/orchestra/README.md` 순서로 읽고 자기 역할을 확정한다.
- 자동 로드 환경에서 `AGENTS.md` 를 먼저 읽었더라도, 최상위 `orchestra` 는 반드시 위 순서로 상세 규칙을 이어서 확인한다.
- 서브 에이전트는 `starter.md` 와 이 공통 인덱스를 반복해서 읽지 않고, `docs/agent/<role>/README.md` 를 첫 문서로 바로 읽고 자기 역할을 확정한다.
- 각 서브 에이전트는 자기 역할 시작 폴더를 먼저 읽고 숙지한 뒤에만 실제 작업을 시작한다.
- `orchestra` 는 서브 에이전트 생성 때 역할 설명 전문을 반복하지 않고, 역할명, 먼저 읽을 자기 역할 README 경로, 현재 문제 상황, 필요한 압축 context, 기대 산출물, `Context Report` 요구만 짧게 전달한다.
- 서브 에이전트 생성 프롬프트에는 `starter.md`, 이 공통 인덱스, `docs/agent/orchestra/README.md` 를 부팅 또는 첫 읽기 문서로 넣지 않는다. 운영 문서 검토가 필요한 작업도 먼저 역할 README 를 읽게 한 뒤 진행한다. `starter.md` 관련 작업은 전체 파일 읽기를 맡기지 않고, `orchestra` 가 필요한 문제 상황과 좁은 발췌 또는 줄 범위만 작업 입력으로 제공한다.
- 반복되는 금지사항, 체크리스트, 산출물 규칙은 각 역할의 `README.md` 가 담당한다.
- 이 공통 인덱스는 bootstrap 전체의 역할 라우팅, 읽기 순서, 공통 경계만 유지한다.
- 상세 운영 규칙, 검증 방식, 산출물 형식, 역할별 금지사항은 각 역할 README 가 맡으며, 이 파일에 중복해 쌓지 않는다.
- `user-docs` 는 `docs/agent/user-docs/README.md` 를 먼저 읽고 역할 경계를 확정한 뒤, 정상 작업에서는 `README.md` 와 `docs/human/` 의 사용자용 문서를 관리한다. 기본 작업 공간은 `docs/human/user-docs/` 이며, 프로젝트 주제 주입이나 공개 문서 갱신처럼 명시된 경우에는 `docs/human/` 일반 문서도 관리한다. 중간 중단 인계 종료에서는 자기 handoff 만 `docs/agent/user-docs/handoff/latest.md` 에 남긴다.
- 단, `ai-docs` 는 AI 에이전트용 개발 문서 관리 역할이므로 `docs/agent/` 내부 역할 문서와 루트 운영 문서를 읽고 갱신할 수 있다.
- `ai-docs` 를 제외한 역할 폴더 사이의 교차 읽기와 교차 수정은 금지한다.
- 역할이 바뀌면 기존 세션을 재사용하지 않고 새 세션을 연다.
- `codegen` 은 문서화를 하지 않는다. 단, 중간 중단 인계 종료에서는 `docs/agent/codegen/handoff/latest.md` 하나만 작성할 수 있다.
- `orchestra`, `planning`, `analysis`, `review`, `test`, `ephemeral` 은 자기 역할 범위 안의 작업 기록 문서를 스스로 갱신해야 한다.
- 역할 규칙, 부팅 규칙, 역할별 README 구조처럼 에이전트 문서 시스템 자체를 바꾸는 일은 `ai-docs` 가 맡는다.
- `user-docs` 는 사용자용 문서를 스스로 갱신하고, 역할 진입 문서 자체의 운영 규칙 변경은 `ai-docs` 가 맡는다.
- 문서가 불필요하게 누적되지 않게 항상 정리하고 압축해야 한다.
- 서브 에이전트를 실행할 때는 사용 가능한 최신 프론티어급 모델을 기본으로 선택한다. 2026-05-22 현재 기준 기본값은 `gpt-5.5` 와 reasoning effort `xhigh` 다.
- 기본 최상위 세션은 `orchestra` 이며, 계획 작업과 최종 완성도 보고도 반드시 `planning` 서브 에이전트로 실행한다.
- 범용 health-check 부트스트랩 상태에서 사용자가 새 프로젝트 주제를 말하면, `orchestra` 는 일반 개발 업무보다 먼저 `inject_subject_once.md` 를 적용한다.
- 사용자가 개발 업무를 주면 `orchestra` 는 기본 7개 서브 에이전트 `planning`, `codegen`, `review`, `test`, `analysis`, `user-docs`, `ai-docs` 를 생성하거나 실행 준비한다.
- 실제 sub-agent 생성 도구가 제공되면 사용자가 매번 다시 지시하지 않아도 `orchestra` 가 직접 서브 에이전트를 생성한다.
- 상위 런타임 또는 도구 정책이 명시적 사용자 승인 없이는 sub-agent 생성을 금지하면, `orchestra` 는 그 한계와 필요한 승인 또는 런타임 수정 지점을 사용자에게 보고하고 중단한다.
- 실제 sub-agent 생성 도구가 없으면 `orchestra` 는 그 한계를 사용자에게 알리고 중단한다. 같은 컨텍스트 안에서 역할을 흉내 내는 방식은 개발 업무의 fallback 구현으로 사용하지 않으며, 실제 sub-agent 사용으로 표현하지 않는다.
- 개발 업무 루프는 예시가 아니라 절대 고정 절차다. `planning` 뒤와 `codegen` 앞에는 사용자 보고/수정 게이트를 둔다. 역할 실행 순서는 `planning` → `codegen` → `review` → `codegen` → `test` → `analysis` → `codegen` → `review` → `codegen` → `test` → `analysis` → `codegen` → `test` → `planning` 보고다.
- 첫 `test` 가 문제 없이 프로그램 실행, 목적 완수, 종료까지 확인하면 문제 분석/수정 루프를 건너뛰고 `orchestra` 에 성공 보고한 뒤 결과 취합 단계로 넘어간다.
- 세 번째 `test` 이후에는 성공 여부와 관계없이 더 이상 코드 개발을 진행하지 않고, `orchestra` 가 결과를 취합해 `planning` 보고서로 사용자에게 전달한다.
- `planning` 보고서를 사용자에게 전달하고 사용자의 반응을 받은 뒤, `user-docs` 는 사용자용 문서를 갱신하고 `ai-docs` 는 AI 에이전트용 개발 문서를 갱신한다.
- 성공, 실패, 중단, blocked 를 포함해 모든 사이클 close candidate 뒤에는 `orchestra` 가 `cycle-report` 를 dispatch 해 `.xavi/reports/development_cycles/<cycle_id>/` artifact 를 만들게 한다.
- `cycle-report` 는 polling 으로 종료를 추정하지 않고, `orchestra` 의 명시 dispatch 로만 실행된다.
- 작업 cycle 중 사용자 프롬프트 수신 직후에는 `user_request_verbatim`, sub-agent spawn 직전에는 `agent_dispatch.prompt_verbatim`, sub-agent completion 직후에는 가능한 result 원문을 trace DB 에 즉시 append 한다.
- "자동 append" 는 `code-level automatic` 과 `orchestra-protocol automatic` 을 분리한다. repo 코드는 dev-console/CLI 내부 입력 저장 경로가 호출되었을 때 append 할 수 있지만, Codex `spawn_agent` tool invocation 경계를 스스로 자동 interception 하지는 못한다.
- Codex runtime 이 spawn 경계를 repo 코드에 자동 연결하지 않는 환경에서는 `orchestra` 가 `spawn_agent` 직전 `xavi-bootstrap trace append` 또는 동등 trace append 명령을 실행하고 성공을 확인한 뒤 spawn 해야 하며, completion 직후 result 원문도 같은 방식으로 append 해야 한다.
- runtime boundary append 실패 시 `orchestra` 는 해당 spawn 을 진행하지 않거나 cycle 을 fail-closed 로 표시한다. summary, derived, display, report 문자열은 원문 append evidence 를 대체하지 않는다.
- 모든 원문 evidence 는 `source_ref`, `hash_sha256`, `timestamp`, `order`, `role`, `agent_id` metadata 를 가져야 하며, 없는 값은 누락 사유를 남긴다.
- `cycle-report` 는 trace DB/context bundle 에서 받은 원문 evidence 를 복사/검증만 한다. 원문이 없으면 기존 summary, display, report 를 원문처럼 복원하거나 재생성하지 않는다.
- 파생 요약과 화면 투영은 `derived`, `summary`, `display` 이름과 UI 라벨을 가져야 하며, 원문 evidence 누락은 `audit.missing_evidence[]`, `audit.status=fail|warn`, `evidence_status=incomplete|fail`, 화면 경고로 표시한다.
- 명령 evidence 는 `command_verbatim`, `result_verbatim`, `output_verbatim` 을 독립 원문 필드로 취급한다. viewer 나 report model 이 첫 번째 non-empty 필드만 표시하면 회귀다.
- 각 작업 cycle 은 canonical `cycle_id` 와 별도로 사람이 부르는 `cycle_alias` 를 가질 수 있다. alias 원장은 SQLite `development_cycle_aliases` 이며, report root 의 `aliases.json` 은 viewer/index projection 이다.
- `orchestra` 는 cycle alias 를 `trace reserve-alias` 로 예약하고 `trace resolve-alias` 로 readback 한다. 충돌, malformed alias, malformed `aliases.json`, resolve mismatch 는 임의 alias fallback 없이 fail-closed 로 처리한다.
- dev-console 의 `/reports/by-alias/<alias>/` 와 `/api/reports/by-alias/<alias>/<file>` 는 `aliases.json` 을 통해 canonical report artifact 로 이동하는 viewer route 이며, trace DB fallback report 생성 경로가 아니다.
- `cycle-report` artifact 생성과 readback 뒤에는 `orchestra` 가 frontend report server 를 자동 실행하거나 이미 같은 설정으로 떠 있는 서버를 재사용하고, 해당 cycle report URL 을 브라우저로 자동 오픈한다.
- report server 재사용/readiness 는 작은 `/api/reports/<cycle_id>/ready` endpoint 로 판정하고 큰 `/reports/<cycle_id>/` HTML 전체를 준비 신호로 읽지 않는다. 대형 report 오류, stale report server, URL open 실패는 같은 cycle 안에서 `xavi-dev-console open-cycle`/report server 경로로 해결해야 하며, Python server 나 임의 포트 변경으로 우회하지 않는다.
- dev-console/report server 는 생성된 artifact 를 보여주는 viewer 다. trace DB 를 읽어 fallback 보고서를 새로 만들거나 누락된 `cycle-report` 를 대체하지 않는다.
- report server 의 지정 포트가 다른 프로세스에 점유되어 있으면 임의 포트로 fallback 하지 않고 fail-closed 로 실패를 보고한다.
- 보고서의 목적은 사용자가 각 역할군이 실제로 무엇을 지시받았고 무엇을 했는지 브라우저에서 시각적으로 확인하게 하는 것이다.
- `cycle-report` artifact 생성, readback, report server readiness, browser open 확인까지 끝나야 1회 개발 사이클이 종료된다. 실패 시 `orchestra` 는 직접 fallback 보고서를 쓰지 않고 실패 자체를 trace 에 남겨 보고한다.
- `codegen` 이 코드를 생성, 수정, 삭제했다면 `report.json.code_changes` 에 이번 cycle 에서 변경된 모든 hunk 를 파일명, 언어, 변경 종류, diff hunk, 변경 라인, 한국어 report annotation 과 함께 기록한다. 새 파일은 전체 신규 내용을 added hunk 로 보여주고, 대형/바이너리/민감 가능 파일은 `audit.json` 에 제외 이유를 남긴다.
- 모든 서브 에이전트는 종료 응답에 `Context Report` 를 포함해 자기 컨텍스트 상태를 `low`, `medium`, `high`, `near-limit` 중 하나로 자체 평가한다.
- `high` 또는 `near-limit` 인 서브 에이전트는 후속 작업에 재사용하지 않고, 요약을 흡수한 뒤 새 서브 에이전트로 이어간다.
- 컨텍스트 한계나 세션 재시작 때문에 작업 도중 종료하면 `interrupted-handoff-close` 로 처리하고, 각 활성 역할이 자기 `docs/agent/<role>/handoff/latest.md` 를 직접 남긴다.
- 이 중간 중단 인계에서는 `user-docs` 와 `ai-docs` 가 모든 역할을 대신 문서화하지 않는다.

## 역할별 이동

- `orchestra` → `docs/agent/orchestra/README.md`
- `planning` → `docs/agent/planning/README.md`
- `analysis` → `docs/agent/analysis/README.md`
- `codegen` → `docs/agent/codegen/README.md`
- `review` → `docs/agent/review/README.md`
- `test` → `docs/agent/test/README.md`
- `ai-docs` → `docs/agent/ai-docs/README.md`
- `ephemeral` → `docs/agent/ephemeral/README.md`
- `user-docs` → `docs/agent/user-docs/README.md`
- `cycle-report` → `docs/agent/cycle-report/README.md`
- `dev-console` → `docs/agent/dev-console/README.md`

사용자 문서 전담 AI 도 먼저 `docs/agent/user-docs/README.md` 로 이동해 역할 경계를 확정한 뒤, 정상 작업에서는 `README.md` 와 `docs/human/` 의 사용자용 문서를 관리한다. 기본 작업 공간은 `docs/human/user-docs/` 이며, 중간 중단 인계 종료에서는 자기 handoff 만 `docs/agent/user-docs/handoff/latest.md` 에 남긴다.

## 운영 메모

이 저장소는 폴더 구조와 문서 규칙으로 역할 분리를 표현한다.
현재 운영 방식은 런타임 차단이 아니라 최상위 `orchestra` 에는 `starter.md` 를, 서브 에이전트에는 자기 역할 README 경로와 압축 작업 context 만 짧게 주입해 역할 침범을 줄이는 방식이다.
`crates/xavi-harness/` 는 프로젝트 코드와 기능 검증용 하네스이며, 메인 에이전트나 서브 에이전트 실행을 통제하는 장치가 아니다.
작업 도중 세션을 재시작해야 할 때만 각 역할의 `handoff/` 폴더가 사용되며, 정상 1회 사이클 문서화는 `user-docs` 와 `ai-docs` 단계에서 수행하고 사실 기반 종료 artifact 는 `cycle-report` 단계에서 수행한다.
