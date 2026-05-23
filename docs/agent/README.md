# Agent Documentation Systems

이 디렉터리는 내부 AI 아홉 역할을 위한 문서 시스템 루트다.
최상위 `orchestra` 세션은 `starter.md` 를 읽은 뒤 이 파일을 읽고 전체 역할 경로를 확인한다.
`orchestra` 가 생성한 서브 에이전트는 `starter.md` 를 다시 읽지 않고, 배정된 자기 역할 폴더의 `README.md` 를 바로 읽는다.
세션을 마무리할 때는 루트의 `ender.md` 를 현재 역할 기준 종료 규약으로 적용해야 한다.
`ender.md` 는 정상 사이클 종료와 중간 중단 인계 종료를 먼저 분류한다.

## 내부 역할

- `orchestra`: 최상위 사용자 대화, 서브 에이전트 관리, 반환 검수 전담
- `planning`: 초기 계획 초안과 최종 완성도 보고서 전담
- `analysis`: 테스트 문제점의 근본 원인 분석 전담
- `codegen`: 코드 생성 전담
- `review`: 생성된 코드 검수와 하네스 보강 전담
- `test`: 검증 명령 실행, 문제점 리스트 작성, 재테스트 전담
- `ai-docs`: AI 에이전트 전용 개발 문서와 인계 정보 관리 전담
- `user-docs`: 사용자 문서 전담 역할의 에이전트 진입점과 사용자 문서 갱신 전담
- `ephemeral`: 임시 테스트포스 전담

## 문서/컨텍스트 규칙

- 최상위 `orchestra` 는 `starter.md` → `docs/agent/README.md` → `docs/agent/orchestra/README.md` 순서로 읽고 자기 역할을 확정한다.
- 서브 에이전트는 `starter.md` 와 이 공통 인덱스를 반복해서 읽지 않고, `docs/agent/<role>/README.md` 를 바로 읽고 자기 역할을 확정한다.
- 각 서브 에이전트는 자기 역할 시작 폴더를 먼저 읽고 숙지한 뒤에만 실제 작업을 시작한다.
- `orchestra` 는 서브 에이전트 생성 때 역할 설명 전문을 반복하지 않고, 역할명과 먼저 읽을 역할 문서 경로만 짧게 전달한다.
- 반복되는 금지사항, 체크리스트, 산출물 규칙은 각 역할의 `README.md` 가 담당한다.
- `user-docs` 는 `docs/agent/user-docs/README.md` 를 먼저 읽고 역할 경계를 확정한 뒤, 정상 작업에서는 실제 사용자 문서를 `docs/human/user-docs/` 에서만 관리한다. 중간 중단 인계 종료에서는 자기 handoff 만 `docs/agent/user-docs/handoff/latest.md` 에 남긴다.
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
- 사용자가 개발 업무를 주면 `orchestra` 는 기본 7개 서브 에이전트 `planning`, `codegen`, `review`, `test`, `analysis`, `user-docs`, `ai-docs` 를 생성하거나 실행 준비한다.
- 개발 업무 루프는 예시가 아니라 절대 고정 절차다. `planning` 뒤와 `codegen` 앞에는 사용자 보고/수정 게이트를 둔다. 역할 실행 순서는 `planning` → `codegen` → `review` → `codegen` → `test` → `analysis` → `codegen` → `review` → `codegen` → `test` → `analysis` → `codegen` → `test` → `planning` 보고다.
- 첫 `test` 가 문제 없이 프로그램 실행, 목적 완수, 종료까지 확인하면 문제 분석/수정 루프를 건너뛰고 `orchestra` 에 성공 보고한 뒤 결과 취합 단계로 넘어간다.
- 세 번째 `test` 이후에는 성공 여부와 관계없이 더 이상 코드 개발을 진행하지 않고, `orchestra` 가 결과를 취합해 `planning` 보고서로 사용자에게 전달한다.
- `planning` 보고서를 사용자에게 전달하고 사용자의 반응을 받은 뒤, `user-docs` 는 사용자용 문서를 갱신하고 `ai-docs` 는 AI 에이전트용 개발 문서를 갱신한다. 두 문서 갱신까지 끝나야 1회 개발 사이클이 종료된다.
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

사용자 문서 전담 AI 도 먼저 `docs/agent/user-docs/README.md` 로 이동해 역할 경계를 확정한 뒤, 정상 작업에서는 실제 사용자 문서를 `docs/human/user-docs/` 에서만 관리한다. 중간 중단 인계 종료에서는 자기 handoff 만 `docs/agent/user-docs/handoff/latest.md` 에 남긴다.

## 운영 메모

이 저장소는 폴더 구조와 문서 규칙으로 역할 분리를 표현한다.
현재 운영 방식은 런타임 차단이 아니라 `starter.md` 와 역할별 README 경로를 서브 에이전트 컨텍스트에 짧게 주입해 역할 침범을 줄이는 방식이다.
`crates/xavi-harness/` 는 프로젝트 코드와 기능 검증용 하네스이며, 메인 에이전트나 서브 에이전트 실행을 통제하는 장치가 아니다.
작업 도중 세션을 재시작해야 할 때만 각 역할의 `handoff/` 폴더가 사용되며, 정상 1회 사이클 문서화는 여전히 `user-docs` 와 `ai-docs` 단계에서 수행한다.
