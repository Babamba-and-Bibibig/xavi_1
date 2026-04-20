# Ephemeral Agent Docs

이 폴더는 필요에 따라 임시로 생성되는 서브 AI 에이전트용 문서 시스템이다.

## 담당 역할

- 특정 주제만 빠르게 조사
- 국소 코드 탐색
- 보조 검증
- 일시적인 토론 또는 정리

## 접근 허용 범위

- 이 폴더 내부에서 자신에게 배정된 세션 문서
- 공통 진입 문서인 `starter.md`
- 공통 인덱스인 `docs/agent/README.md`

## 접근 금지 범위

- `docs/agent/codegen/`
- `docs/agent/review/`
- `docs/agent/planning/`
- `docs/agent/ephemeral/` 내 다른 임시 세션 문서
- `docs/human/`

## 관리 방식

- 임시 에이전트마다 개별 세션 폴더를 만든다.
- 세션 폴더는 `sessions/<session-id>/` 형식으로 만든다.
- 작업 종료 후 결과만 남기고 세션을 닫는다.
- 자기 세션에서 한 일과 남은 일을 스스로 문서화한다.

## 기본 문서

- `SESSION_TEMPLATE.md`: 새 임시 세션 생성 템플릿
- `RESULT_TEMPLATE.md`: 세션 종료 시 반환 형식 템플릿
- `sessions/README.md`: 세션 폴더 운영 규칙

## 원칙

- 왜 실행되었는지 적는다.
- 무엇을 했는지 적는다.
- 무엇이 남았는지 적는다.
- 다른 역할 문서는 건드리지 않는다.
- 종료 후 불필요한 탐색 메모는 정리하고 반환에 필요한 내용만 남긴다.
