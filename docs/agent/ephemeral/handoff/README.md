# Ephemeral Handoff

이 폴더는 `ender.md` 가 종료 유형을 `interrupted-handoff-close` 로 분류했을 때만 사용한다.
정상 사이클 종료에서는 이 폴더를 갱신하지 않는다.

작성 대상:

- `docs/agent/ephemeral/handoff/latest.md`

모든 `latest.md` 는 루트 `ender.md` 의 표준 스키마를 따른다.
필수 필드는 `role`, `handoff_type`, `last_updated`, `trigger`, `current_task`, `current_loop_step`, `completed`, `in_progress`, `next_steps`, `touched_files`, `unverified_changes`, `blocking_questions`, `context_report` 이다.
아래 항목은 이 역할이 특히 채워야 할 역할별 내용이다.

`ephemeral` 은 자기 인계에 아래를 남긴다.

- 임시 작업의 목적
- 실제로 확인한 내용
- 아직 확인하지 못한 내용
- 다음에 넘겨야 할 역할
- 자기 세션 폴더가 있다면 해당 경로
- 자기 `Context Report`
