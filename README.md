# xavi_1

## 한국어

### 이 프로젝트는 무엇인가요?

`xavi_1`은 새 소프트웨어 프로젝트를 AI 협업 방식으로 시작하기 위한 Rust 기반 프로젝트 부트스트랩입니다.

일반적인 템플릿이 폴더와 예제 코드만 제공하는 데 비해, 이 저장소는 AI 에이전트가 프로젝트를 진행하는 방식까지 문서로 고정합니다. 사용자는 먼저 프로젝트 주제를 정하고, AI 에이전트는 `starter.md`, `inject_subject_once.md`, `ender.md`, `docs/agent/` 문서 체계를 따라 분석, 계획, 구현, 검수, 테스트, 문서화를 역할별로 나누어 진행합니다.

현재 코드는 작은 health-check 예제를 가진 Rust clean architecture 워크스페이스입니다. 즉, 완성된 제품이 아니라 새 제품을 안전하게 만들기 위한 시작점입니다.

### 무엇을 위한 것인가요?

이 부트스트랩의 목적은 다음과 같습니다.

- 새 프로젝트를 시작할 때 구조, 역할, 문서화 방식을 매번 다시 정하지 않게 합니다.
- AI 에이전트가 계획 없이 바로 코드를 쓰거나, 검수 없이 결과를 끝내는 흐름을 줄입니다.
- 구현, 리뷰, 테스트, 문서화를 서로 다른 역할로 분리해 작업 품질을 높입니다.
- 긴 작업 중 세션이 끊기거나 컨텍스트가 커져도 다음 세션이 이어받을 수 있게 합니다.
- 프로젝트 주제가 정해지면 현재 generic health-check 예제를 실제 도메인 코드와 문서로 점진적으로 바꾸게 합니다.

### 핵심 아이디어

이 저장소는 두 가지를 함께 제공합니다.

1. Rust clean architecture 코드 골격
2. AI 협업을 위한 역할 기반 운영 문서

코드 구조는 다음 레이어를 유지합니다.

- `crates/xavi-domain`: 핵심 엔티티, 값 객체, 도메인 규칙
- `crates/xavi-application`: 유스케이스와 port
- `crates/xavi-infrastructure`: 외부 시스템 adapter
- `crates/xavi-harness`: 시나리오 기반 검증 하네스
- `apps/xavi-bootstrap`: 실제 실행 진입점과 composition root

문서 구조는 다음 역할을 유지합니다.

- `orchestra`: 사용자와 대화하고 전체 흐름을 조율하는 최상위 역할
- `planning`: 초기 계획과 최종 완성도 보고 담당
- `codegen`: clean architecture 를 유지하는 제품 코드 구현 담당
- `review`: 코드 검수와 하네스 기반 테스트 코드 작성/보강 담당
- `test`: 검증 명령 실행과 문제점 리스트 작성 담당. 테스트 코드는 직접 작성하지 않음
- `analysis`: 테스트 실패의 근본 원인 분석 담당
- `user-docs`: 사용자용 문서 작성 담당
- `ai-docs`: AI 에이전트용 내부 문서 관리 담당
- `ephemeral`: 일회성 조사나 보조 작업 담당

### 현재 상태

현재 저장소는 작은 실행 가능한 샘플 상태입니다.

- Rust workspace가 구성되어 있습니다.
- clean architecture 레이어가 crate 단위로 분리되어 있습니다.
- `xavi-bootstrap` 실행 파일이 health-check service를 조립해 실행합니다.
- `xavi-harness`가 health-check scenario를 테스트합니다.
- AI 역할 문서와 세션 종료 문서가 포함되어 있습니다.

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

### AI 에이전트와 사용하는 방법

이 저장소는 사람이 직접 코드를 읽고 수정할 수도 있지만, 주된 사용 방식은 AI 에이전트에게 문서 규약을 읽히고 역할에 따라 진행하게 하는 것입니다.
최상위 세션만 `starter.md` 를 읽고, `orchestra` 가 만드는 서브 에이전트는 각자의 `docs/agent/<role>/README.md` 만 먼저 읽습니다.
상세한 고정 루프와 예외 규칙은 `starter.md` 를 권위 문서로 봅니다.
`AGENTS.md` 를 자동으로 읽는 코딩 에이전트 환경에서는 루트 `AGENTS.md` 가 짧은 사전 부팅 계약 역할을 하며, 사용자가 매번 "서브 에이전트를 생성해라" 라고 말하지 않아도 기본 `orchestra` 모드로 시작해야 합니다.
단, 실제 sub-agent 생성은 런타임이 `spawn_agent` 같은 도구를 제공하고 상위 정책이 사용을 허용할 때만 가능합니다. 그런 도구가 없거나 정책상 자동 생성이 금지되면 에이전트는 그 한계를 말하고 제한된 fallback 으로 진행해야 합니다.
한 세션에서 `starter.md` 를 한 번 읽었다면, 이후 프로젝트 개발 프롬프트들은 별도 설명이 없어도 같은 `orchestra` 계약을 계속 상속해야 합니다.

처음 보는 사용자는 아래 순서만 따르면 됩니다.

1. 이 저장소를 새 프로젝트 폴더로 복사합니다.
2. Codex 같은 코딩 에이전트가 `AGENTS.md` 를 자동으로 읽는지 확인하고, 아니면 `starter.md` 를 읽게 합니다.
3. 만들고 싶은 프로젝트 주제를 자연어로 설명합니다.
4. 에이전트가 내는 초기 계획을 읽고 승인하거나 수정합니다.
5. 구현은 에이전트가 역할별 루프에 따라 진행하게 둡니다.
6. 테스트 결과와 최종 보고를 확인합니다.
7. 세션이 길어지거나 끝낼 때는 `ender.md` 로 정리하게 합니다.

사용자는 하네스, 역할 문서, 클린 아키텍처 세부 구조를 미리 알 필요가 없습니다.
처음에는 "무엇을 만들고 싶은지", "누가 쓸 것인지", "처음에 꼭 필요한 기능이 무엇인지"만 말하면 됩니다.

처음 시작할 때는 AI 에이전트에게 이렇게 요청합니다.

```text
starter.md 읽고 현재 문서 체계 숙지하고, 프로젝트 구조를 간단히 파악해줘.
```

프로젝트 주제를 처음 넣을 때는 이렇게 요청합니다.

```text
inject_subject_once.md 읽고, 다음 주제에 맞게 이 부트스트랩을 초기 프로젝트 구조로 바꿔줘.

주제:
<여기에 만들 프로젝트 설명>
```

처음 쓰는 사용자는 `inject_subject_once.md` 파일명을 몰라도 됩니다.
최상위 에이전트가 `starter.md` 를 읽은 상태에서 새 프로젝트 주제나 제품 아이디어를 받으면, 아직 범용 health-check 부트스트랩 상태인지 확인한 뒤 `inject_subject_once.md` 를 자동으로 적용해야 합니다.

예를 들어 이렇게 말하면 됩니다.

```text
이 부트스트랩으로 로컬 문서 요약 앱을 시작하고 싶어.
사용자는 개인 작업자이고, 처음 기능은 문서 업로드, 요약 생성, 결과 저장이면 돼.
starter.md 기준으로 필요한 절차대로 진행해줘.
```

개발을 이어갈 때는 작업을 작게 맡기는 것이 좋습니다.

```text
이번 사이클에서는 문서 업로드와 저장 구조까지만 구현해줘.
계획 먼저 보고하고, 내가 승인하면 진행해줘.
```

작업 중 세션을 끝내야 할 때는 이렇게 요청합니다.

```text
ender.md 읽고 세션 종료해.
```

이 흐름을 따르면 에이전트는 시작, 주제 주입, 구현, 검수, 테스트, 종료 인계를 문서 규약에 맞춰 진행하게 됩니다.

### 권장 작업 흐름

1. `starter.md`로 현재 세션 역할과 문서 체계를 확인합니다.
2. 만들 프로젝트의 주제, 사용자, 핵심 기능, 실행 형태를 정합니다.
3. `inject_subject_once.md`를 통해 주제를 코드와 문서에 처음 주입합니다.
4. `planning` 역할이 계획을 작성하고, 사용자가 승인하거나 수정합니다.
5. `codegen` 역할이 승인된 계획에 따라 구현합니다.
6. `review` 역할이 구현 결과를 검수합니다.
7. `test` 역할이 하네스와 Cargo 명령으로 검증합니다.
8. 실패가 있으면 `analysis` 역할이 원인을 분석하고 다시 수정 루프로 보냅니다.
9. 작업이 끝나면 `user-docs`와 `ai-docs`가 각각 사용자 문서와 내부 에이전트 문서를 정리합니다.
10. 종료 시 `ender.md` 규칙에 따라 정상 종료인지 중간 인계 종료인지 나눕니다.

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
- 사용자용 문서와 AI 내부 문서를 섞지 마세요. 사람에게 필요한 문서는 `docs/human/`, 에이전트 운영 문서는 `docs/agent/`에 둡니다.

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
- 현재 예제 코드는 health-check 수준입니다. 실제 프로젝트 주제는 사용자가 주입해야 합니다.
- 공개 저장소로 사용할 때는 개인 경로, 로컬 서버 주소, 비밀 키, `.env` 파일이 들어가지 않게 확인해야 합니다.
- 이 저장소는 실행 바이너리를 포함하므로 `Cargo.lock` 을 함께 커밋하는 것을 권장합니다. 공개 전에 lockfile이 누락되지 않았는지 확인하세요.

### 저장소 구조

```text
.
├── apps/
│   └── xavi-bootstrap/          # 실행 진입점
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

The current codebase is a small Rust clean architecture workspace with a health-check example. It is not a finished product. It is a starting point for safely growing a real product.

### What Is It For?

This bootstrap is designed to:

- Avoid redefining project structure, roles, and documentation rules every time a new project starts.
- Reduce AI workflows where code is written before planning or finished without review.
- Improve quality by separating implementation, review, testing, and documentation into distinct roles.
- Preserve handoff context when a long session is interrupted or the context window becomes too large.
- Turn the current generic health-check example into real domain code and documentation once a project subject is chosen.

### Core Idea

This repository provides two things together.

1. A Rust clean architecture code skeleton
2. Role-based operating documents for AI collaboration

The code keeps these layers:

- `crates/xavi-domain`: core entities, value objects, and domain rules
- `crates/xavi-application`: use cases and ports
- `crates/xavi-infrastructure`: external system adapters
- `crates/xavi-harness`: scenario-based verification harness
- `apps/xavi-bootstrap`: executable entrypoint and composition root

The documentation keeps these roles:

- `orchestra`: top-level coordinator that talks to the user and manages the workflow
- `planning`: initial plan and final completion report
- `codegen`: product implementation while preserving clean architecture
- `review`: code review and harness-based test code creation or reinforcement
- `test`: verification commands and issue lists, but not test code authoring
- `analysis`: root-cause analysis for test failures
- `user-docs`: human-facing documentation
- `ai-docs`: internal AI-agent documentation
- `ephemeral`: temporary research or support tasks

### Current State

The repository is currently a small runnable scaffold.

- A Rust workspace is configured.
- Clean architecture layers are separated as crates.
- `xavi-bootstrap` composes and runs a health-check service.
- `xavi-harness` tests health-check scenarios.
- AI role documents and session shutdown documents are included.

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

### How To Use It With An AI Agent

You can read and edit this repository manually, but the intended workflow is to ask an AI agent to follow the operating documents and work by role.
Only the top-level session reads `starter.md`; sub-agents created by `orchestra` start from their own `docs/agent/<role>/README.md`.
Treat `starter.md` as the authoritative document for the full fixed loop and exceptions.
In coding-agent environments that automatically load `AGENTS.md`, the root `AGENTS.md` acts as a short pre-boot contract, so the user should not need to repeat "create sub-agents" on every request.
Real sub-agent creation still depends on the runtime exposing a tool such as `spawn_agent` and allowing it under higher-priority policy. If no real sub-agent tool is available or automatic spawning is forbidden, the agent must say so and only use a limited single-context fallback.
Once the top-level session has read `starter.md` once, later project-development prompts in the same session should continue to inherit the same `orchestra` contract.

If you are new to this repository, use this simple flow.

1. Copy this repository into a new project folder.
2. Check whether a coding agent such as Codex automatically reads `AGENTS.md`; if not, ask it to read `starter.md`.
3. Describe the project you want to build in plain language.
4. Review the agent's initial plan, then approve or revise it.
5. Let the agent follow the role-based loop for implementation.
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

### Recommended Workflow

1. Use `starter.md` to identify the current session role and documentation system.
2. Define the project subject, users, core features, and runtime shape.
3. Use `inject_subject_once.md` to inject the subject into code and documentation.
4. Let the `planning` role write a plan, then approve or revise it.
5. Let the `codegen` role implement the approved plan.
6. Let the `review` role inspect the implementation.
7. Let the `test` role run harness and Cargo verification.
8. If tests fail, let the `analysis` role identify the root cause before another fix.
9. When the cycle is complete, let `user-docs` and `ai-docs` update human-facing and internal documentation.
10. At shutdown, use `ender.md` to classify the session as a normal close or an interrupted handoff.

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
- Keep human-facing documents in `docs/human/` and AI operating documents in `docs/agent/`.

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
- The current sample code is only a health-check flow. The real project subject must be injected by the user.
- Before publishing, check that no private paths, local server addresses, secrets, or `.env` files are included.
- This repository includes an executable binary, so committing `Cargo.lock` is recommended. Before publishing, make sure the lockfile is included.

### Repository Structure

```text
.
├── apps/
│   └── xavi-bootstrap/          # executable entrypoint
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
