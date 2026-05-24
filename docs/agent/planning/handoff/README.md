# Planning Handoff

이 폴더는 `ender.md` 가 종료 유형을 `interrupted-handoff-close` 로 분류했을 때만 사용한다.
정상 사이클 종료에서는 이 폴더를 갱신하지 않는다.

작성 대상:

- `docs/agent/planning/handoff/latest.md`

모든 `latest.md` 는 루트 `ender.md` 의 표준 스키마를 따른다.
필수 필드는 `role`, `handoff_type`, `last_updated`, `trigger`, `current_task`, `current_loop_step`, `completed`, `in_progress`, `next_steps`, `touched_files`, `unverified_changes`, `blocking_questions`, `context_report` 이다.
아래 항목은 이 역할이 특히 채워야 할 역할별 내용이다.

`planning` 은 자기 인계에 아래를 남긴다.

- 전달받은 목표와 구현 범위
- 작성 중이던 초기 계획 또는 최종 완성도 보고서 상태
- 확정된 결정과 아직 열린 질문
- 다음 계획 세션이 바로 이어야 할 항목
- 자기 `Context Report`
