# xavi_1

## 한국어

### 이 프로젝트는 무엇인가요?

`xavi_1`은 새 소프트웨어 프로젝트를 AI 협업 방식으로 시작하기 위한 Rust 기반 프로젝트 부트스트랩입니다.

일반적인 템플릿이 폴더와 예제 코드만 제공하는 데 비해, 이 저장소는 AI 에이전트가 프로젝트를 진행하는 방식까지 문서로 고정합니다. 사용자는 먼저 프로젝트 주제를 정하고, AI 에이전트는 `starter.md`, `inject_subject_once.md`, `ender.md`, `docs/agent/` 문서 체계를 따라 분석, 계획, 구현, 검수, 테스트, 문서화를 역할별로 나누어 진행합니다.

이 부트스트랩은 Codex, Claude Code 같은 AI coding tool을 오래 켜 두고 큰 작업을 맡기는 방식에서 생기는 문제를 줄이기 위해 만들어졌습니다. 긴 AI 작업이 제한 없이 이어지면 나중에 어느 지점에서 실수가 시작됐는지, 코드가 실제로 어떻게 동작하는지, 왜 디버깅이 부담스러워졌는지 찾기 어려워질 수 있습니다. 그래서 이 저장소는 개발을 끝없는 무인 AI 작업이 아니라, 사람이 검토할 수 있는 작은 `1회 작업 사이클` 단위로 끊습니다. 각 사이클은 특히 HTML 보고서를 통해 AI가 프롬프트를 어떻게 이해했는지, 어떤 서브 에이전트가 만들어졌는지, 그들이 어떤 지시와 컨텍스트를 받았고 무엇을 반환했는지, `orchestra`가 그 결과를 어떻게 받아들이거나 이어갔는지, 어떤 검증이 있었는지 개발자가 확인한 뒤 다음 지시로 넘어가는 흐름을 기준으로 합니다.

현재 코드는 작은 health-check 예제를 가진 Rust clean architecture 워크스페이스입니다. 즉, 완성된 제품이 아니라 새 제품을 안전하게 만들기 위한 시작점입니다.

### 무엇을 위한 것인가요?

이 부트스트랩의 목적은 다음과 같습니다.

- 새 프로젝트를 시작할 때 구조, 역할, 문서화 방식을 매번 다시 정하지 않게 합니다.
- AI 에이전트가 계획 없이 바로 코드를 쓰거나, 검수 없이 결과를 끝내는 흐름을 줄입니다.
- 구현, 리뷰, 테스트, 문서화를 서로 다른 역할로 분리해 작업 품질을 높입니다.
- 긴 작업 중 세션이 끊기거나 컨텍스트가 커져도 다음 세션이 이어받을 수 있게 합니다.
- 프로젝트 주제가 정해지면 현재 generic health-check 예제를 실제 도메인 코드와 문서로 점진적으로 바꾸게 합니다.

### 핵심 아이디어

이 저장소는 아래 핵심 키워드로 이해하면 됩니다.

1. **클린 아키텍처 구조:** 유지보수하기 쉬운 Rust clean architecture 골격으로 시작합니다.
2. **역할 기반 AI 협업:** 계획, 구현, 검수, 테스트, 문서화, 보고를 역할별로 나눕니다.
3. **문서 분리:** 사람용 개발 문서와 AI 에이전트용 재시작/개발 보조 문서를 따로 관리합니다.
4. **오케스트라 메인 에이전트:** `orchestra` 는 개발자와 소통하고 context를 아끼며 실제 작업은 sub-agent에 위임합니다.
5. **서브 에이전트 활용:** `planning`, `codegen`, `review`, `test`, `analysis`, `user-docs`, `ai-docs`, `cycle-report`가 각자 맡은 일만 수행합니다.
6. **1회 작업 사이클:** 끝없는 AI 작업 대신 개발자가 검토 가능한 작은 단위로 끊어 진행합니다.
7. **HTML 사이클 보고서:** 매 사이클의 증거와 결과를 로컬 DB에 남기고 HTML 보고서로 확인합니다.

### 왜 내부 DB와 프론트엔드 서버가 있나요?

이 저장소는 제품 기능이 아니라 개발 방식을 함께 제공하는 부트스트랩입니다.
그래서 내부 SQLite DB와 로컬 프론트엔드 report server는 사용자 제품의 백엔드/프론트엔드가 아니라, `1회 작업 사이클`을 검토하기 위한 부트스트랩 운영 도구입니다.

- **내부 DB:** 사용자 프롬프트 원문, sub-agent 지시, 반환 결과, 테스트 결과, 코드 변경 요약을 로컬 evidence로 저장합니다.
- **프론트엔드 report server:** 저장된 evidence와 cycle-report 산출물을 HTML 화면으로 열어 개발자가 빠르게 검토하게 합니다.
- **활용 방식:** 사이클이 끝날 때 자동으로 열린 HTML 보고서를 보고, AI가 무엇을 이해했고 무엇을 바꿨으며 어떤 검증이 있었는지 확인한 뒤 다음 지시를 내립니다.

코드 구조는 다음 레이어를 유지합니다.

- `crates/xavi-domain`: 핵심 엔티티, 값 객체, 도메인 규칙
- `crates/xavi-application`: 유스케이스와 port
- `crates/xavi-infrastructure`: 외부 시스템 adapter
- `crates/xavi-harness`: 시나리오 기반 검증 하네스
- `apps/xavi-bootstrap`: 실제 실행 진입점과 composition root
- `apps/xavi-dev-console`: `development_trace` 와 cycle-report artifact 를 로컬 화면과 report server 로 확인하는 부트스트랩 운영 지원 도구

문서 구조는 다음 역할을 유지합니다.

- `orchestra`: 사용자와 대화하고 전체 흐름을 조율하는 최상위 역할
- `planning`: 초기 계획과 최종 완성도 보고 담당
- `codegen`: clean architecture 를 유지하는 제품 코드 구현 담당
- `review`: 코드 검수와 하네스 기반 테스트 코드 작성/보강 담당
- `test`: 검증 명령 실행과 문제점 리스트 작성 담당. 테스트 코드는 직접 작성하지 않음
- `analysis`: 테스트 실패의 근본 원인 분석 담당
- `user-docs`: 사용자용 문서 작성 담당
- `ai-docs`: AI 에이전트용 내부 문서 관리 담당
- `cycle-report`: 1회 작업 사이클의 증거와 결과를 HTML/JSON/raw/audit/context 산출물로 고정하는 보고 담당
- `dev-console`: `development_trace` 와 cycle-report 산출물을 로컬에서 확인하는 지원 담당
- `ephemeral`: 일회성 조사나 보조 작업 담당

### 1회 작업 사이클 한눈에 보기

개발은 아래처럼 작은 사이클 단위로 진행합니다.

```text
사용자 요청 -> orchestra 이해/분해 -> planning -> 사용자 승인
-> codegen -> review -> test -> docs(user-docs/ai-docs)
-> cycle-report -> HTML 보고서 확인 -> 다음 사이클 지시
```

개발자 워크플로 관점에서 한 사이클은 HTML 보고서가 생성되고, 브라우저에서 열리고, 개발자가 내용을 확인해야 완료됩니다.
보고서를 보지 않고 다음 지시로 넘어가면 이 부트스트랩의 핵심 이유인 "AI 작업을 사람이 검토 가능한 증거 단위로 끊어 관리한다"는 장점이 사라집니다.

보고서에서 특히 확인할 내용은 다음과 같습니다.

- 원래 사용자 프롬프트
- AI가 요청을 이해하고 분해한 방식
- 각 sub-agent에게 전달된 지시와 컨텍스트
- 각 sub-agent가 반환한 결과
- `orchestra` 가 결과를 받아들인 방식과 다음 행동
- 테스트와 검증 결과
- 코드 변경 요약
- audit 상태와 evidence 누락 경고

### 현재 상태

현재 저장소는 작은 실행 가능한 샘플 상태입니다.

- Rust workspace가 구성되어 있습니다.
- clean architecture 레이어가 crate 단위로 분리되어 있습니다.
- `xavi-bootstrap` 실행 파일이 health-check service를 조립해 실행합니다.
- `xavi-harness`가 health-check scenario를 테스트합니다.
- AI 역할 문서와 세션 종료 문서가 포함되어 있습니다.
- `development_trace` DB는 작업 사이클의 증거 저장, trace audit/export, cycle-report 생성, dev-console 표시의 기반으로 쓰이도록 설계되어 있습니다.
- `cycle-report`, `xavi-dev-console`, 로컬 frontend report server 흐름이 포함되어 있습니다.
- 정상적인 1회 작업 사이클은 cycle-report 산출물의 `index.html` 보고서를 만들고, report server가 그 HTML 보고서를 브라우저에서 열어 확인하는 흐름까지 포함합니다.
- 작업 사이클을 사람이 짧게 부를 수 있는 별칭을 예약하고 조회할 수 있습니다.

현재 상태는 제품 완성본이 아니라, 프로젝트 주제를 주입하기 전의 안정적인 출발점입니다.

### 빠른 시작

필요한 도구:

- Rust 1.85 이상
- Cargo
- 로컬 파일을 읽고 수정할 수 있는 AI coding assistant

기본 확인 명령:

```bash
cargo check --workspace
cargo test --workspace
cargo run -p xavi-bootstrap
```

공개 전에는 아래 명령까지 확인하는 것을 권장합니다.

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

예상 실행 출력은 다음과 비슷합니다.

```text
xavi-bootstrap initialized: status=Healthy, message=bootstrap completed
```

### 증거와 보고서

한 사이클이 끝나면 두 가지가 남습니다.

- `.xavi/development_trace.sqlite3`: 사용자 요청, 역할 지시, 역할 반환, 검증 결과 원문
- `.xavi/reports/development_cycles/<cycle_id>/`: 사람이 확인하는 HTML/JSON 보고서 묶음

사이클 완료 기준은 단순합니다.

1. `cycle-report` 가 보고서를 만든다.
2. 로컬 report server가 브라우저에서 보고서를 연다.
3. 사용자가 내용을 확인한다.
4. 그 확인을 기준으로 다음 지시를 낸다.

보고서에서 보면 되는 것은 아래입니다.

- 사용자가 무엇을 요청했는지
- AI가 어떻게 이해했는지
- 어떤 역할에게 무엇을 지시했는지
- 각 역할이 무엇을 반환했는지
- 테스트와 검증이 통과했는지
- 코드가 어디서 어떻게 바뀌었는지
- 누락된 evidence나 audit 경고가 있는지

주의할 점도 간단합니다.

- 이 파일들은 기본적으로 로컬 프로젝트 폴더 안에만 남습니다.
- 부트스트랩 코드는 보고서를 외부 서버로 자동 업로드하지 않습니다.
- `.xavi` 를 직접 공유, 커밋, 업로드하면 evidence가 노출될 수 있습니다.
- 프롬프트에는 비밀 키, 토큰, 비밀번호, 민감한 개인정보를 넣지 않는 것이 좋습니다.

이미 생성된 보고서는 아래 명령으로 다시 열 수 있습니다.

```bash
cargo run -p xavi-dev-console -- open-cycle --cycle-id <cycle-id>
```

이 명령은 `127.0.0.1:4200` 의 로컬 report server를 사용해
`/reports/<cycle-id>/` 화면을 엽니다.

### AI와 시작하기

사용자는 내부 구조를 전부 외울 필요가 없습니다.
처음에는 아래 세 가지만 말하면 됩니다.

- 무엇을 만들고 싶은지
- 누가 쓸 것인지
- 처음에 꼭 필요한 기능이 무엇인지

역할은 이렇게 보면 됩니다.

- `orchestra`: 사용자와 대화하고 일을 역할별로 나눔
- `planning`: 계획과 완성도 보고를 작성
- `codegen`: 구현
- `review`: 코드 검수
- `test`: 실행과 검증
- `user-docs`, `ai-docs`, `cycle-report`: 문서와 보고서 정리

중요한 시작 규칙은 두 가지입니다.

- 최상위 세션만 `starter.md` 를 읽습니다.
- 역할 에이전트는 자기 `docs/agent/<role>/README.md` 부터 읽습니다.

실제 sub-agent 도구가 없거나 사용이 막힌 환경에서는
역할 흉내로 몰래 구현하지 않고, 그 제한을 사용자에게 보고합니다.

처음 시작할 때는 이렇게 요청합니다.

```text
starter.md 읽고 현재 문서 체계 숙지하고, 프로젝트 구조를 간단히 파악해줘.
```

프로젝트 주제를 처음 넣을 때는 이렇게 요청합니다.

```text
inject_subject_once.md 읽고, 다음 주제에 맞게 이 부트스트랩을 초기 프로젝트 구조로 바꿔줘.

주제:
<여기에 만들 프로젝트 설명>
```

파일명을 몰라도 됩니다.
새 프로젝트 주제를 말하면 최상위 `orchestra` 가 필요한 절차를 적용합니다.

예시는 아래처럼 짧게 말하면 됩니다.

```text
이 부트스트랩으로 로컬 문서 요약 앱을 시작하고 싶어.
사용자는 개인 작업자이고, 처음 기능은 문서 업로드, 요약 생성, 결과 저장이면 돼.
starter.md 기준으로 필요한 절차대로 진행해줘.
```

개발을 이어갈 때는 작게 맡깁니다.

```text
이번 사이클에서는 문서 업로드와 저장 구조까지만 구현해줘.
계획 먼저 보고하고, 내가 승인하면 진행해줘.
```

세션을 끝낼 때는 이렇게 요청합니다.

```text
ender.md 읽고 세션 종료해.
```

이후 개발도 한 번에 크게 맡기지 말고,
직전 HTML 보고서를 확인한 뒤 다음 작은 사이클을 지시하는 흐름이 가장 안정적입니다.

### 작업 사이클 별칭

긴 cycle id는 파일 경로와 자동화에서 계속 사용하고, 사람끼리 회의하거나 문서에서 언급할 때는 짧은 별칭을 붙일 수 있습니다.
별칭은 `분류-NNN` 형식입니다. 초기 문서에서는 특정 과거 cycle 대신 `<cycle-id>` 와 `<분류-NNN>` placeholder 를 사용합니다.

별칭을 확인하려면 아래 명령을 사용합니다.

```bash
cargo run -p xavi-bootstrap -- trace resolve-alias --alias <분류-NNN>
```

새 별칭은 정확한 이름을 직접 지정하거나, 분류만 주고 다음 번호를 받을 수 있습니다.

```bash
cargo run -p xavi-bootstrap -- trace reserve-alias --cycle <cycle_id> --alias <분류-NNN> --title "<짧은 제목>"
cargo run -p xavi-bootstrap -- trace reserve-alias --cycle <cycle_id> --category <분류> --title "<짧은 제목>"
```

보고서 서버가 켜져 있으면 `/reports/by-alias/<별칭>/` 경로로도 같은 사이클 보고서에 접근할 수 있습니다.
별칭 색인인 `aliases.json` 이 깨졌거나 필드가 맞지 않으면 보고서 서버는 임의로 추측하지 않고 오류를 보여줍니다.

### 권장 작업 흐름

1. `starter.md`로 현재 세션 역할과 문서 체계를 확인합니다.
2. 만들 프로젝트의 주제, 사용자, 핵심 기능, 실행 형태를 정합니다.
3. `inject_subject_once.md`를 통해 주제를 코드와 문서에 처음 주입합니다.
4. 이후 개발 요청은 한 번에 끝낼 수 있는 작은 1회 작업 사이클로 나눕니다.
5. `planning` 역할이 해당 사이클의 계획을 작성하고, 사용자가 승인하거나 수정합니다.
6. 실제 sub-agent 도구가 있는 환경에서는 `codegen` 역할의 서브 에이전트가 승인된 계획에 따라 구현합니다.
7. `review` 역할이 구현 결과를 검수합니다.
8. `test` 역할이 하네스와 Cargo 명령으로 검증합니다.
9. 실패가 있으면 `analysis` 역할이 원인을 분석하고 다시 수정 루프로 보냅니다.
10. 작업이 끝나면 `user-docs`와 `ai-docs`가 각각 사용자 문서와 내부 에이전트 문서를 정리합니다.
11. `cycle-report`가 해당 1회 작업 사이클의 HTML/JSON 보고서 산출물을 만들고, `xavi-dev-console` report server가 해당 HTML 보고서를 브라우저에서 열어 확인합니다.
12. 사용자는 보고서의 요청 원문, 역할별 반환, 테스트 결과, 코드 변경 요약, audit/evidence 경고를 확인한 뒤 다음 사이클 지시를 냅니다.
13. 종료 시 `ender.md` 규칙에 따라 정상 종료인지 중간 인계 종료인지 나눕니다.

### 장점

#### 1. 역할이 섞이지 않습니다

AI가 혼자 계획, 구현, 검수, 테스트, 문서화를 모두 처리하면 판단이 쉽게 섞입니다. 이 저장소는 역할별 문서와 접근 범위를 나누어 각 에이전트가 자기 일에 집중하게 만듭니다.

#### 2. 계획 없는 구현을 줄입니다

개발 루프는 `planning` 이후 사용자 보고와 승인 단계를 거치도록 설계되어 있습니다. 사용자는 구현 전에 범위와 성공 기준을 확인할 수 있습니다.

#### 3. 테스트와 검수가 기본 흐름에 들어갑니다

`review`와 `test`가 별도 역할로 존재합니다. 특히 `crates/xavi-harness`는 기능을 시나리오 단위로 검증하기 위한 외부 레이어 역할을 합니다.

#### 4. 긴 작업을 이어가기 쉽습니다

`ender.md`는 작업이 끝난 정상 종료와 작업 도중 끊기는 중간 인계를 구분합니다. 중간 인계에서는 각 활성 역할이 자기 `handoff/latest.md`를 남기는 방식으로 다음 세션이 이어받기 쉽게 합니다.

#### 5. 프로젝트가 커져도 운영 방식이 유지됩니다

초기에는 health-check 예제뿐이지만, 주제를 주입하면 domain, application, infrastructure, harness가 실제 기능 중심으로 확장됩니다. 문서 체계는 프로젝트가 커져도 같은 방식으로 남습니다.

### 장점을 극대화하는 방법

- 작업을 작게 나누고, 한 사이클에 너무 많은 기능을 넣지 마세요.
- AI에게 바로 구현을 시키기보다 최상위 세션에는 `starter.md`, 서브 에이전트에는 각 역할 README를 읽게 하세요.
- 프로젝트 주제 주입은 `inject_subject_once.md`를 통해 한 번에 기준을 잡으세요.
- 계획 단계에서 목표, 제외 범위, 성공 기준을 분명히 승인하세요.
- 구현 후에는 반드시 `cargo test --workspace`와 하네스 테스트를 확인하세요.
- 테스트 실패를 바로 고치기보다, `analysis` 역할에 원인을 먼저 정리하게 하세요.
- 세션이 길어지면 `ender.md`로 중간 인계를 남기고 새 세션에서 이어가세요.
- Codex 세션이 길어져 context compact 이후 이어갈 때는, 문제가 생겨도 fallback 방식으로 우회하지 말고 먼저 보고하라는 조건을 짧게 다시 붙여 주는 것이 좋습니다.
- 사용자용 문서와 AI 내부 문서를 섞지 마세요. 사람에게 필요한 문서는 `docs/human/`, 에이전트 운영 문서는 `docs/agent/`에 둡니다.

context compact 이후 이어서 작업할 때는 아래처럼 덧붙일 수 있습니다.

```text
구현 중 문제가 생기면 fallback 방식으로 우회하지 말고 즉시 보고해줘. 내가 승인하기 전에는 다른 방식으로 대체 구현하지 마.
```

### 기대할 수 있는 효과

이 부트스트랩을 제대로 사용하면 다음 효과를 기대할 수 있습니다.

- 프로젝트 시작 시 구조 결정 시간이 줄어듭니다.
- AI 에이전트의 역할 혼선과 컨텍스트 오염이 줄어듭니다.
- 구현 전 계획 검토가 자연스러운 절차가 됩니다.
- 검수와 테스트가 선택 사항이 아니라 기본 루프가 됩니다.
- 작업 중단 후 재개가 쉬워집니다.
- 코드, 테스트, 사용자 문서, AI 내부 문서가 함께 성장합니다.
- 작은 실험 프로젝트를 실제 제품 구조로 확장하기 쉬워집니다.

### 한계와 주의점

- 이 저장소는 AI 에이전트를 실제로 실행하는 런타임 플랫폼이 아닙니다.
- 역할 분리는 문서 규약과 운영 절차로 강제됩니다. 별도 프로세스 격리나 권한 차단 런타임은 포함되어 있지 않습니다.
- 실제 서브 에이전트 생성 가능 여부는 사용 중인 AI 런타임과 상위 정책에 달려 있습니다. 이 저장소의 문서 규약은 도구가 있는 환경에서 실제 서브 에이전트 사용을 요구하지만, 없는 도구를 만들어내지는 못합니다.
- 도구나 정책이 막는 경우 이 저장소의 규칙은 fallback 구현을 허용하지 않습니다. 제한을 보고하고 멈춘 뒤 운영 방식이나 도구 사용 문제를 수정 대상으로 삼아야 합니다.
- 현재 예제 코드는 health-check 수준입니다. 실제 프로젝트 주제는 사용자가 주입해야 합니다.
- 이 부트스트랩은 브라우저 제어 보조 구현이나 브라우저 자동화 기능을 포함하지 않습니다. 현재 사용자-facing 흐름의 기준은 역할별 작업 증거, cycle-report 산출물, 로컬 report server와 HTML 보고서 viewer 확인입니다.
- 로컬 HTML report viewer와 report server 기능을 포함하기 때문에 초기 파일 수와 용량은 단순 health-check 부트스트랩보다 큽니다. 이 증가는 사이클 증거를 사람이 확인하기 위한 보고서 기능 때문이며, 브라우저 제어 자동화 기능을 뜻하지 않습니다.
- 공개 저장소로 사용할 때는 개인/세션별 임시 로컬 URL, 포트, PID, machine-specific endpoint, 개인 경로, 비밀 키, `.env` 파일, 그리고 `.xavi/` 와 `target/` 같은 로컬 산출물이 소스 배포물에 섞이지 않게 확인해야 합니다. 문서화된 기본 `127.0.0.1:4200` report server는 정상 기능이며 제거하거나 배포 제외 대상으로 볼 항목이 아닙니다. 이는 기능 제거가 아니라 배포 패키징과 ignore 상태를 확인하는 주의사항입니다.
- 이 저장소는 실행 바이너리를 포함하므로 `Cargo.lock` 을 함께 커밋하는 것을 권장합니다. 공개 전에 lockfile이 누락되지 않았는지 확인하세요.

### 저장소 구조

```text
.
├── apps/
│   ├── xavi-bootstrap/          # 실행 진입점
│   └── xavi-dev-console/        # development_trace/report viewer 운영 지원 도구
├── crates/
│   ├── xavi-domain/             # 도메인 모델
│   ├── xavi-application/        # 유스케이스와 port
│   ├── xavi-infrastructure/     # adapter 구현
│   └── xavi-harness/            # scenario 기반 테스트 하네스
├── docs/
│   ├── agent/                   # AI 역할별 내부 문서
│   └── human/                   # 사람용 문서
├── starter.md                   # 세션 시작 규약
├── inject_subject_once.md       # 프로젝트 주제 최초 주입 규약
├── ender.md                     # 세션 종료와 인계 규약
├── Cargo.lock                   # 재현 가능한 Cargo 해석 결과
└── README.md
```

### 이 부트스트랩이 잘 맞는 경우

- AI와 함께 새 Rust 프로젝트를 시작하려는 경우
- 여러 단계의 구현, 검수, 테스트, 문서화를 반복해야 하는 경우
- 긴 작업을 여러 세션에 걸쳐 이어가야 하는 경우
- 프로젝트 구조와 AI 작업 절차를 함께 표준화하고 싶은 경우
- 처음부터 domain, application, infrastructure, harness 분리를 유지하고 싶은 경우

### 이 부트스트랩이 잘 맞지 않는 경우

- 단일 파일 스크립트처럼 아주 작은 작업
- 역할 분리나 문서화가 부담스러운 일회성 실험
- Rust가 아닌 언어로 바로 시작해야 하는 프로젝트
- 별도 AI 런타임, 큐, 웹 UI, 멀티 프로세스 에이전트 실행기가 이미 필요한 프로젝트

---

## English

### What Is This Project?

`xavi_1` is a Rust-based project bootstrap for starting new software projects with an AI collaboration workflow.

Unlike a normal template that only provides folders and example code, this repository also defines how AI agents should work. The user first defines a project subject, then AI agents follow `starter.md`, `inject_subject_once.md`, `ender.md`, and the `docs/agent/` role documentation to split work across analysis, planning, implementation, review, testing, and documentation.

This bootstrap exists to reduce a common problem with Codex, Claude Code, and similar AI coding tools: long unconstrained runs can make it hard to later find where a mistake began, how the code actually works, and why debugging became stressful. The repository introduces the idea of a `single work cycle` so development does not turn into endless unattended AI work. Each cycle should end with developer review, especially through the HTML report, showing how the AI understood the prompt, which sub-agents were created, what instructions and context they received, what they returned, how `orchestra` accepted or continued the work, and what verification happened.

The current codebase is a small Rust clean architecture workspace with a health-check example. It is not a finished product. It is a starting point for safely growing a real product.

### What Is It For?

This bootstrap is designed to:

- Avoid redefining project structure, roles, and documentation rules every time a new project starts.
- Reduce AI workflows where code is written before planning or finished without review.
- Improve quality by separating implementation, review, testing, and documentation into distinct roles.
- Preserve handoff context when a long session is interrupted or the context window becomes too large.
- Turn the current generic health-check example into real domain code and documentation once a project subject is chosen.

### Core Idea

You can understand this repository through these core keywords.

1. **Clean Architecture Skeleton:** Start with a maintainable Rust clean architecture structure.
2. **Role-Based AI Collaboration:** Split planning, implementation, review, testing, documentation, and reporting by role.
3. **Separated Docs:** Keep human-facing development docs separate from AI-agent restart/development helper docs.
4. **Orchestra Main Agent:** `orchestra` talks with the developer, preserves context, and delegates real work to sub-agents.
5. **Sub-Agent Workflow:** `planning`, `codegen`, `review`, `test`, `analysis`, `user-docs`, `ai-docs`, and `cycle-report` each handle one responsibility.
6. **Single Work Cycle:** Replace endless AI work with small units that a developer can review.
7. **HTML Cycle Report:** Store each cycle's evidence locally and inspect the result through an HTML report.

### Why Include An Internal DB And Frontend Server?

This repository is a bootstrap for a development workflow, not a finished product feature set.
The internal SQLite DB and local frontend report server are not your product backend or product frontend. They are bootstrap operation tools for reviewing each `single work cycle`.

- **Internal DB:** Stores user prompt originals, sub-agent instructions, returned results, test results, and code-change summaries as local evidence.
- **Frontend report server:** Opens the stored evidence and cycle-report artifacts as an HTML screen for fast developer review.
- **How to use it:** When a cycle ends, inspect the automatically opened HTML report, confirm what the AI understood, what changed, and what was verified, then give the next instruction.

The code keeps these layers:

- `crates/xavi-domain`: core entities, value objects, and domain rules
- `crates/xavi-application`: use cases and ports
- `crates/xavi-infrastructure`: external system adapters
- `crates/xavi-harness`: scenario-based verification harness
- `apps/xavi-bootstrap`: executable entrypoint and composition root
- `apps/xavi-dev-console`: bootstrap operations support tool for viewing `development_trace` and cycle-report artifacts through a local UI and report server

The documentation keeps these roles:

- `orchestra`: top-level coordinator that talks to the user and manages the workflow
- `planning`: initial plan and final completion report
- `codegen`: product implementation while preserving clean architecture
- `review`: code review and harness-based test code creation or reinforcement
- `test`: verification commands and issue lists, but not test code authoring
- `analysis`: root-cause analysis for test failures
- `user-docs`: human-facing documentation
- `ai-docs`: internal AI-agent documentation
- `cycle-report`: report role that records one work cycle's evidence and result as HTML/JSON/raw/audit/context artifacts
- `dev-console`: support role for local viewing of `development_trace` and cycle-report artifacts
- `ephemeral`: temporary research or support tasks

### Single Work Cycle At A Glance

Development is meant to move through small cycles like this:

```text
User request -> orchestra understanding/breakdown -> planning -> user approval -> codegen -> review -> test -> docs(user-docs/ai-docs) -> cycle-report -> HTML report review -> next cycle instruction
```

From the developer workflow perspective, a cycle is not complete until the HTML report has been generated, opened in the browser, and inspected by the developer.
If the developer ignores the report and jumps straight to the next request, the main reason for this workflow is lost: keeping AI work in evidence-backed units that a person can review.

In the report, inspect:

- The original user prompt
- How the AI interpreted and broke down the request
- The instructions and context sent to each sub-agent
- Each sub-agent result
- How `orchestra` accepted the result and chose the next action
- Test and verification results
- Code-change summary
- Audit status and missing-evidence warnings

### Current State

The repository is currently a small runnable scaffold.

- A Rust workspace is configured.
- Clean architecture layers are separated as crates.
- `xavi-bootstrap` composes and runs a health-check service.
- `xavi-harness` tests health-check scenarios.
- AI role documents and session shutdown documents are included.
- The `development_trace` DB is the intended foundation for work-cycle evidence storage, trace audit/export, cycle-report generation, and dev-console display.
- The repository includes the `cycle-report` flow, `xavi-dev-console`, and a local frontend report server.
- A normal single work cycle includes generating the cycle-report `index.html` report and opening that HTML report in the browser through the report server.
- Development cycles can reserve and resolve short human-readable aliases.

This is the stable starting point before injecting a concrete project subject.

### Quick Start

Required tools:

- Rust 1.85 or newer
- Cargo
- An AI coding assistant that can read and edit local files

Basic verification commands:

```bash
cargo check --workspace
cargo test --workspace
cargo run -p xavi-bootstrap
```

Before publishing, the stricter verification pass is:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Expected runtime output should look similar to this:

```text
xavi-bootstrap initialized: status=Healthy, message=bootstrap completed
```

### Work-Cycle Evidence And HTML Reports

A single work cycle is the unit for one small development goal: planning, user approval, implementation, review, testing, documentation cleanup, cycle-report generation, and HTML report review.
Development requests should be scoped to these small work cycles whenever possible. Large requests should normally be split across multiple cycles.
From the developer workflow perspective, the cycle is not considered finished until the HTML report has been opened and inspected.

The intended cycle record includes the original user request, role dispatches and returns, verification results, and change summaries in the `development_trace` DB.
Original user prompts are also stored automatically as work-cycle evidence in the internal local SQLite DB at `.xavi/development_trace.sqlite3`.
This DB is not a throwaway log. It is the intended foundation for building each cycle's HTML/JSON report and dev-console view from the same source evidence.

When one work cycle closes, `cycle-report` writes HTML/JSON report artifacts under `.xavi/reports/development_cycles/<cycle_id>/`.
The primary human-facing report is `index.html`, alongside structured files such as `report.json`, `raw.json`, `audit.json`, and `context.md`.
In the normal flow, the local report server may automatically open this HTML report in your browser. If a local report page appears, treat it as part of the work. It is not an advertisement page or an external upload screen; it is a local artifact viewer for the already generated cycle-report files.

By default, the trace DB and cycle-report artifacts stay inside local project folders such as `.xavi/development_trace.sqlite3` and `.xavi/reports/development_cycles/<cycle_id>/`.
The bootstrap code does not automatically upload or transmit them to an external server.
Those artifacts can still be exposed if you directly share, commit, or upload `.xavi`, use export/copy commands such as `export --out`, share trace export output, or bind the report server to an externally reachable address instead of local `127.0.0.1`.
Avoid putting secrets, tokens, passwords, or private personal data into prompts unless you accept local evidence retention.

Use the report to check the original user request, AI interpretation, role dispatches and returns, `orchestra` acceptance and next action, test results, code-change summaries, audit status, and missing-evidence warnings.
Before giving the next change request, refer to what the report got right, what it got wrong, and what needs more explanation.
The report is not decorative. It is the shared evidence screen that helps the user and AI choose the next cycle.

To open an existing cycle report, use:

```bash
cargo run -p xavi-dev-console -- open-cycle --cycle-id <cycle-id>
```

By default, this command reuses or starts the local report server at `127.0.0.1:4200`, then opens `/reports/<cycle-id>/` in the browser.
The report server is a viewer for existing cycle-report artifacts. It is not a fallback path that guesses or synthesizes a missing report from the trace DB.

### How To Use It With An AI Agent

You can read and edit this repository manually, but the intended workflow is to ask an AI agent to follow the operating documents and work by role.
Only the top-level session reads `starter.md`; for development work, that top-level session acts as `orchestra` and communicates with the user.
`orchestra` is not the role that directly implements product code. It creates and manages the planning, implementation, review, test, and documentation sub-agents, checks their returned results, and reports verified state back to the user.
Sub-agents created by `orchestra` start from their own `docs/agent/<role>/README.md`.
Treat `starter.md` as the authoritative document for the full fixed loop and exceptions.
In coding-agent environments that automatically load `AGENTS.md`, the root `AGENTS.md` acts as a short pre-boot contract, so the user should not need to repeat "create sub-agents" on every request.
When the runtime exposes a real sub-agent tool such as `spawn_agent` and higher-priority policy allows it, development work must use real sub-agents.
If no real sub-agent tool is available, or if higher-priority policy blocks automatic spawning, the agent must not switch into a single-context fallback implementation. It should report the limitation, stop the development work, and leave the operating mode, documentation, tool usage, or approval point as the thing that must be fixed.
If the implementation method from the approved initial plan turns out to be broken, the agent should not silently implement the feature through a different workaround. The method, document, or tool-use problem itself should be corrected so the approved path can be used.
Once the top-level session has read `starter.md` once, later project-development prompts in the same session should continue to inherit the same `orchestra` contract.

If you are new to this repository, use this simple flow.

1. Copy this repository into a new project folder.
2. Check whether a coding agent such as Codex automatically reads `AGENTS.md`; if not, ask it to read `starter.md`.
3. Describe the project you want to build in plain language.
4. Review the agent's initial plan, then approve or revise it.
5. In an environment with real sub-agent tools, let the agent follow the role-based loop for implementation.
6. Check the test result and final report.
7. When the session gets long or ends, ask the agent to use `ender.md`.

You do not need to understand the harness, role documents, or clean architecture details upfront.
At the beginning, just explain what you want to build, who will use it, and which first features matter most.

At the beginning, ask the agent:

```text
Read starter.md, understand the current documentation system, and roughly inspect the project structure.
```

When injecting the first real project subject, ask:

```text
Read inject_subject_once.md and turn this bootstrap into an initial project structure for the following subject.

Subject:
<describe the project you want to build>
```

New users do not need to know the `inject_subject_once.md` filename.
Once the top-level agent has read `starter.md`, a new project subject or product idea should be treated as a subject-injection trigger when the repository is still in the generic health-check bootstrap state.

For example, you can say:

```text
I want to start a local document summarizer with this bootstrap.
The user is an individual knowledge worker, and the first features are document upload, summary generation, and saving results.
Follow the process defined by starter.md.
```

For later development cycles, keep each request small:

```text
For this cycle, implement only document upload and the storage structure.
Report the plan first, then proceed after I approve it.
```

When ending a session, ask:

```text
Read ender.md and close the session.
```

This keeps startup, subject injection, implementation, review, testing, and handoff aligned with the repository rules.
For ongoing development, keep working in one-work-cycle units rather than one large batch, and give the next instruction after checking the previous cycle's HTML report.

### Cycle Aliases

Long cycle ids remain the stable value for files and automation, but each work cycle can also have a short name for meetings and human-facing notes.
Aliases use the `category-NNN` format. Baseline documentation uses placeholders such as `<cycle-id>` and `<category-NNN>` instead of a specific past cycle.

To resolve an alias, run:

```bash
cargo run -p xavi-bootstrap -- trace resolve-alias --alias <category-NNN>
```

To reserve a new alias, either provide the exact alias or provide only the category and let the tool allocate the next sequence:

```bash
cargo run -p xavi-bootstrap -- trace reserve-alias --cycle <cycle_id> --alias <category-NNN> --title "<short title>"
cargo run -p xavi-bootstrap -- trace reserve-alias --cycle <cycle_id> --category <category> --title "<short title>"
```

When the report server is running, `/reports/by-alias/<alias>/` opens the same cycle report through the alias.
If `aliases.json` is malformed, the report server fails closed and shows an error instead of guessing the target cycle.

### Recommended Workflow

1. Use `starter.md` to identify the current session role and documentation system.
2. Define the project subject, users, core features, and runtime shape.
3. Use `inject_subject_once.md` to inject the subject into code and documentation.
4. Split later development requests into small single-work-cycle units.
5. Let the `planning` role write the plan for that cycle, then approve or revise it.
6. When real sub-agent tools are available, let the `codegen` sub-agent implement the approved plan.
7. Let the `review` role inspect the implementation.
8. Let the `test` role run harness and Cargo verification.
9. If tests fail, let the `analysis` role identify the root cause before another fix.
10. When the cycle is complete, let `user-docs` and `ai-docs` update human-facing and internal documentation.
11. Let `cycle-report` create the HTML/JSON report artifacts for that single work cycle, then let the `xavi-dev-console` report server open the HTML report in the browser.
12. Check the report's original request, role returns, test results, code-change summary, and audit/evidence warnings before giving the next cycle instruction.
13. At shutdown, use `ender.md` to classify the session as a normal close or an interrupted handoff.

### Advantages

#### 1. Roles Do Not Get Mixed

When one AI session plans, implements, reviews, tests, and documents everything, judgments can blur. This repository separates role documents and boundaries so each agent focuses on one job.

#### 2. Implementation Does Not Start Without A Plan

The development loop is designed to report the plan to the user before code generation. The user can confirm scope and success criteria before implementation begins.

#### 3. Review And Testing Are Part Of The Default Flow

`review` and `test` are separate roles. `crates/xavi-harness` provides an outer-layer scenario harness for feature verification.

#### 4. Long Work Is Easier To Resume

`ender.md` separates a completed cycle from an interrupted handoff. During an interrupted handoff, active roles write their own `handoff/latest.md` files so the next session can continue with less context loss.

#### 5. The Operating Model Scales With The Project

The initial code is only a health-check example, but once a subject is injected, the domain, application, infrastructure, and harness layers can grow around real features while the same operating model remains in place.

### How To Maximize The Benefits

- Keep each work cycle small enough to review and test.
- Ask the AI agent to read `starter.md` and the relevant role documents before implementation.
- Use `inject_subject_once.md` to establish the first real project subject.
- Approve goals, non-goals, and success criteria during planning.
- Always run `cargo test --workspace` and harness-level checks after implementation.
- Do not rush from a failing test directly into a fix. Ask the `analysis` role to summarize the cause first.
- Use `ender.md` for handoff when a session gets long.
- When a Codex session gets long and resumes after context compaction, restate that implementation problems must be reported instead of bypassed through a fallback path.
- Keep human-facing documents in `docs/human/` and AI operating documents in `docs/agent/`.

After context compaction, you can add a short prompt like this:

```text
If implementation runs into a problem, do not switch to a fallback or workaround. Report it immediately, and do not implement a different approach until I approve it.
```

### Expected Effects

When used properly, this bootstrap should help you:

- Reduce the time spent deciding project structure.
- Reduce role confusion and context pollution in AI-assisted work.
- Make planning review a natural step before implementation.
- Make review and testing part of the default development loop.
- Resume interrupted work more easily.
- Grow code, tests, human documentation, and AI documentation together.
- Turn a small experiment into a product-shaped Rust project more safely.

### Limitations And Notes

- This repository is not an AI-agent runtime platform.
- Role separation is enforced by documentation and operating procedure. It does not include process isolation or a permission runtime.
- Whether real sub-agents can be created depends on the AI runtime and higher-priority policy in use. This repository's operating documents require real sub-agents when the tool exists, but they cannot create a missing runtime capability.
- If tools or policy block real sub-agent creation, this repository's rules do not allow fallback implementation. The agent should report the limitation, stop, and treat the operating mode or tool-use problem as the thing to fix.
- The current sample code is only a health-check flow. The real project subject must be injected by the user.
- This bootstrap does not include browser-control support code or browser automation features. The current human-facing workflow centers on role-based work evidence, cycle-report artifacts, and viewing HTML reports through the local report server.
- Because this bootstrap includes a local HTML report viewer and report server, the initial file count and repository size are larger than a simple health-check-only bootstrap. That increase exists to let people inspect cycle evidence; it does not mean the repository includes browser-control automation.
- Before publishing, check that personal or session-specific temporary local URLs, ports, PIDs, machine-specific endpoints, private paths, secrets, `.env` files, and local artifacts such as `.xavi/` and `target/` are not mixed into the source distribution. The documented default `127.0.0.1:4200` report server is a normal feature, not something to remove or exclude from distribution. This is a packaging and ignore-state check, not a reason to remove runtime features.
- This repository includes an executable binary, so committing `Cargo.lock` is recommended. Before publishing, make sure the lockfile is included.

### Repository Structure

```text
.
├── apps/
│   ├── xavi-bootstrap/          # executable entrypoint
│   └── xavi-dev-console/        # development_trace/report viewer operations support tool
├── crates/
│   ├── xavi-domain/             # domain model
│   ├── xavi-application/        # use cases and ports
│   ├── xavi-infrastructure/     # adapter implementations
│   └── xavi-harness/            # scenario-based test harness
├── docs/
│   ├── agent/                   # internal AI role documents
│   └── human/                   # human-facing documents
├── starter.md                   # session startup protocol
├── inject_subject_once.md       # first subject injection protocol
├── ender.md                     # session shutdown and handoff protocol
├── Cargo.lock                   # reproducible Cargo resolution
└── README.md
```

### Good Fit

This bootstrap is a good fit when:

- You want to start a new Rust project with AI assistance.
- You expect repeated planning, implementation, review, testing, and documentation cycles.
- You need to continue long work across multiple sessions.
- You want to standardize both project structure and AI working procedure.
- You want domain, application, infrastructure, and harness separation from the beginning.

### Not A Good Fit

This bootstrap may be too heavy when:

- The task is a tiny one-file script.
- The experiment does not need role separation or documentation.
- You need to start immediately in a language other than Rust.
- You already need a full AI runtime, queue, web UI, or multi-process agent runner.
