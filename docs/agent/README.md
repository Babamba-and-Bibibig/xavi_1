# Agent Documentation Systems

이 디렉터리는 내부 AI 네 역할을 위한 문서 시스템 루트다.
모든 내부 AI 세션은 `starter.md` 를 읽은 뒤 이 파일을 읽고, 그 다음 자기 역할 폴더로만 이동해야 한다.
세션을 마무리할 때는 루트의 `ender.md` 를 현재 역할 기준 종료 규약으로 적용해야 한다.

## 내부 역할

- `codegen`: 코드 생성 전담
- `review`: 수정과 검수 전담
- `planning`: 기획 대화와 결정 정리 전담
- `ephemeral`: 임시 테스트포스 전담

## 강제 규칙

- 각 세션은 자기 역할 폴더만 읽고 관리한다.
- 역할 폴더 사이의 교차 읽기와 교차 수정은 금지한다.
- 역할이 바뀌면 기존 세션을 재사용하지 않고 새 세션을 연다.
- `codegen` 은 문서화를 하지 않는다.
- `review`, `planning`, `ephemeral` 은 자기 역할 문서를 스스로 갱신해야 한다.
- 문서가 불필요하게 누적되지 않게 항상 정리하고 압축해야 한다.

## 역할별 이동

- `codegen` → `docs/agent/codegen/README.md`
- `review` → `docs/agent/review/README.md`
- `planning` → `docs/agent/planning/README.md`
- `ephemeral` → `docs/agent/ephemeral/README.md`

별도로 사용자 문서 전담 AI 는 이 디렉터리를 사용하지 않고 `docs/human/user-docs/` 로 이동한다.

## 운영 메모

이 저장소는 폴더 구조와 문서 규칙으로 역할 분리를 표현한다.
완전한 기술적 차단은 별도 런타임 정책이 있어야 하지만, 현재 운영 규칙은 역할 침범 금지를 전제로 설계되어 있다.
