# Test Handoff

이 폴더는 `ender.md` 가 종료 유형을 `interrupted-handoff-close` 로 분류했을 때만 사용한다.
정상 사이클 종료에서는 이 폴더를 갱신하지 않는다.

작성 대상:

- `docs/agent/test/handoff/latest.md`

모든 `latest.md` 는 루트 `ender.md` 의 표준 스키마를 따른다.
필수 필드는 `role`, `handoff_type`, `last_updated`, `trigger`, `current_task`, `current_loop_step`, `completed`, `in_progress`, `next_steps`, `touched_files`, `unverified_changes`, `blocking_questions`, `context_report` 이다.
아래 항목은 이 역할이 특히 채워야 할 역할별 내용이다.

`test` 는 자기 인계에 아래를 남긴다.

- 실행한 검증 명령
- 통과한 명령과 실패한 명령
- 실패 재현 방법과 핵심 오류
- 아직 실행하지 못한 테스트
- `analysis` 로 넘겨야 할 문제점 리스트
- 자기 `Context Report`
