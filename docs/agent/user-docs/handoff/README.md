# User Docs Handoff

이 폴더는 `ender.md` 가 종료 유형을 `interrupted-handoff-close` 로 분류했을 때만 사용한다.
정상 사이클 종료에서는 이 폴더를 갱신하지 않는다.

작성 대상:

- `docs/agent/user-docs/handoff/latest.md`

모든 `latest.md` 는 루트 `ender.md` 의 표준 스키마를 따른다.
필수 필드는 `role`, `handoff_type`, `last_updated`, `trigger`, `current_task`, `current_loop_step`, `completed`, `in_progress`, `next_steps`, `touched_files`, `unverified_changes`, `blocking_questions`, `context_report` 이다.
아래 항목은 이 역할이 특히 채워야 할 역할별 내용이다.

중간 중단 인계 종료에서 `user-docs` 는 `docs/human/user-docs/` 를 갱신하지 않는다.
자기 작업 상태만 이 파일에 남긴다.

`user-docs` 는 자기 인계에 아래를 남긴다.

- 사용자 문서로 정리하려던 입력
- 아직 사용자 문서에 반영하지 못한 확인된 사실
- 확인되지 않아 쓰지 않은 내용
- 다음 `user-docs` 세션이 이어야 할 문서 작업
- 자기 `Context Report`
