# Documentation System

이 프로젝트의 문서는 크게 두 축으로 나뉜다.

- `human/`: 사람이 읽고 판단하는 문서
- `agent/`: AI 에이전트용 문서 시스템 루트

다만 `agent/` 는 단일 문서 폴더가 아니다.
그 내부에는 역할별로 분리된 아홉 개의 독립 문서 시스템이 존재한다.

- `orchestra/`: 최상위 사용자 대화와 서브 에이전트 조율 담당 AI
- `planning/`: 계획 초안과 최종 완성도 보고 담당 AI
- `analysis/`: 테스트 문제점의 근본 원인 분석 담당 AI
- `codegen/`: 코드 생성 담당 AI
- `review/`: 생성 코드 검수와 하네스 보강 담당 AI
- `test/`: 테스트 실행과 문제점 리스트 작성 담당 AI
- `ai-docs/`: AI 에이전트용 개발 문서와 인계 정보 관리 담당 AI
- `user-docs/`: 사용자 문서 담당 AI 의 역할 진입 문서
- `ephemeral/`: 필요 시 임시로 생성되는 서브 AI

그리고 `human/` 아래에는 사람 독자를 위한 문서를 실시간으로 정리하는 전담 AI 세션용 영역이 따로 존재한다.

- `human/user-docs/`: 사용자 문서 전담 AI 가 관리하는 실시간 사용자 문서

공통 원칙은 다음과 같다.

- 사람용 문서와 에이전트용 문서는 목적이 다르다.
- 에이전트용 문서 시스템은 역할별로 서로 침범하지 않는다.
- 최상위 `orchestra` 세션만 `starter.md` 를 부팅 문서로 읽는다.
- 서브 에이전트는 생성 시 자기 역할의 `docs/agent/<role>/README.md` 만 첫 읽기로 받고, 현재 문제 상황, 필요한 압축 context, 기대 산출물은 생성 프롬프트로 전달받는다.
- 사용자 문서 전담 AI 는 정상 작업에서 `README.md` 와 `docs/human/` 을 문서 작업 영역으로 사용한다. 기본 작업 공간은 `docs/human/user-docs/` 이며, 프로젝트 주제 주입이나 공개 문서 갱신처럼 명시된 경우에는 `docs/human/` 일반 문서도 관리한다. 중간 중단 인계 종료에서는 자기 인계만 `docs/agent/user-docs/handoff/latest.md` 에 남긴다.
- AI 에이전트용 개발 문서는 `ai-docs` 가 관리하고, 사용자용 문서는 `user-docs` 가 관리한다.
