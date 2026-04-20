# xavi_1

클린 아키텍처를 전제로 한 Rust 워크스페이스 시작점입니다.

## 레이어

- `crates/xavi-domain`: 엔티티, 값 객체, 도메인 규칙
- `crates/xavi-application`: 유스케이스, inbound/outbound port
- `crates/xavi-infrastructure`: 외부 시스템 어댑터 구현
- `crates/xavi-harness`: 통합 테스트와 시나리오 실행용 하네스
- `apps/xavi-bootstrap`: 실제 실행 진입점과 composition root

## 의존 방향

`bootstrap -> infrastructure -> application -> domain`

`harness -> infrastructure -> application -> domain`

## 시작 명령

```bash
cargo check --workspace
cargo test --workspace
cargo run -p xavi-bootstrap
```
