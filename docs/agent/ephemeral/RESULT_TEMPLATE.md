# Ephemeral Result Template

임시 세션이 끝날 때는 아래 형식으로 결과를 정리한다.
결과는 짧고 명확해야 하며, 다음 역할이 바로 이어받을 수 있어야 한다.

## Template

### Mission

- 왜 이 세션이 실행되었는가

### Done

- 실제로 수행한 작업

### Findings

- 확인한 사실
- 중요한 관찰점

### Remaining

- 아직 남은 작업
- 확정하지 못한 항목

### Next Owner

- 다음에 누가 이어야 하는가
- `orchestra`, `analysis`, `review`, `test`, `planning`, `codegen`, `ai-docs`, `user-docs` 등

### Return Summary

- 사용자나 상위 세션에 바로 반환할 짧은 요약

### Context Report

- `context_level`: `low` | `medium` | `high` | `near-limit`
- `basis`: 이렇게 판단한 이유
- `read_files`: 이번 작업에서 읽은 주요 파일
- `carryover_summary`: 다음 에이전트가 이어받아야 할 5줄 이하 요약
- `recommended_action`: `continue` | `close_and_respawn`

## Cleanup Rule

- 세션 종료 시 불필요한 탐색 메모는 지운다.
- 최종 반환에 필요한 정보만 남긴다.
