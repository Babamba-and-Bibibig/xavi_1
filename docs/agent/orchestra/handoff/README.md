# Orchestra Handoff

이 폴더는 `ender.md` 가 종료 유형을 `interrupted-handoff-close` 로 분류했을 때만 사용한다.
정상 사이클 종료에서는 이 폴더를 갱신하지 않는다.

작성 대상:

- `docs/agent/orchestra/handoff/latest.md`

모든 `latest.md` 는 루트 `ender.md` 의 표준 스키마를 따른다.
필수 필드는 `role`, `handoff_type`, `last_updated`, `trigger`, `current_task`, `current_loop_step`, `completed`, `in_progress`, `next_steps`, `touched_files`, `unverified_changes`, `blocking_questions`, `context_report` 이다.
아래 항목은 이 역할이 특히 채워야 할 역할별 내용이다.

`orchestra` 는 자기 인계에 아래를 남긴다.

- 사용자와 합의한 현재 목표
- 현재 개발 루프 단계
- 생성했거나 대기 중인 서브 에이전트 상태
- 받은 반환물과 아직 받지 못한 반환물
- 다음 세션의 첫 조율 작업
- 닫히기 전에 자기 인계를 남기지 못한 서브 에이전트 목록
- 자기 `Context Report`
