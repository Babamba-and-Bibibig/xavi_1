# Planning Decision Log

이 문서는 기획 관련 확정 사항과 미결 사항을 기록한다.
현재 저장소는 특정 사용자 프로젝트가 아니라 AI 협업 부트스트랩 초기 상태이므로, 과거 cycle 결정 로그는 남기지 않는다.

## 기록 규칙

- 확정된 결정은 날짜 또는 cycle placeholder 와 함께 적는다.
- 아직 미결이면 미결 상태를 명시한다.
- 구현 담당자에게 넘길 기준이 되도록 짧고 명확하게 적는다.
- 실제 과거 cycle id, 로컬 실행 이력, 임시 alias 는 초기 템플릿에 남기지 않는다.

## 부트스트랩 기준 결정

- canonical cycle id 는 자동화와 artifact 경로의 기준으로 유지한다.
- 사람이 읽는 짧은 이름은 별도 `cycle_alias` 로 둘 수 있으며, 형식은 `<category-NNN>` 이다.
- alias 충돌, malformed alias, malformed alias index, resolve mismatch 는 fallback 없이 실패로 다룬다.
- alias 표시와 report projection 은 원문 trace evidence 를 대체하지 않는다.

## 초기 상태

- 현재 사용자 프로젝트 기획 결정은 없다.
