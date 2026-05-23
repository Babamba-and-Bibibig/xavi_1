# Codegen Handoff

이 폴더는 `ender.md` 가 종료 유형을 `interrupted-handoff-close` 로 분류했을 때만 사용한다.
정상 사이클 종료에서는 이 폴더를 갱신하지 않는다.

작성 대상:

- `docs/agent/codegen/handoff/latest.md`

이 경로는 `codegen` 의 유일한 문서 작성 예외다.
`codegen` 은 중간 중단 인계 종료에서만 이 파일을 작성하거나 갱신할 수 있고, 다른 문서는 수정하지 않는다.

`codegen` 은 자기 인계에 아래를 남긴다.

- 구현 목표
- 수정한 코드 파일
- 작성 중이던 코드 상태
- 아직 검수나 테스트가 끝나지 않은 변경
- 다음 `codegen` 세션이 바로 이어야 할 작업
- 자기 `Context Report`
