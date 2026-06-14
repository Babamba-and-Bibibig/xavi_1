//! Fixtures for development trace scenarios.

use xavi_domain::development_trace::{
    DevelopmentTraceKind, NewDevelopmentTraceEntry, trace_text_sha256_hex,
};

const CYCLE_ID: &str = "cycle-dev-test-001";
const USER_TURN_ID: &str = "user-turn-001";

/// Creates a sample user-query trace entry.
#[must_use]
pub fn sample_user_query_entry() -> NewDevelopmentTraceEntry {
    let text = "SQLite3 자연어 개발 trace 원장 MVP";
    let hash = trace_text_sha256_hex(text);
    NewDevelopmentTraceEntry {
        event_id: format!("{CYCLE_ID}/{USER_TURN_ID}/event-001"),
        cycle_id: CYCLE_ID.to_owned(),
        user_turn_id: Some(USER_TURN_ID.to_owned()),
        kind: DevelopmentTraceKind::UserQuery,
        role_name: None,
        summary: "사용자가 SQLite3 자연어 개발 trace 원장 MVP를 요청했다.".to_owned(),
        body: "사용자 범위는 SQLite3 기반 development_trace 원장과 trace CLI다.".to_owned(),
        metadata_json: format!(
            r#"{{"trace_contract_version":2,"phase_id":"user-query","cycle_step":1,"role":"user","status":"requested","content_json":{{"user_request":"{text}","constraints":["SQLite3 기반 development_trace 원장","trace CLI"],"acceptance_criteria":["append/list/show/export flow works"],"user_request_verbatim":{{"text":"{text}","source_type":"user_prompt","source_ref":"harness-user-request-1","role":"user","agent_id":null,"hash_sha256":"{hash}","timestamp":"2026-06-04T21:10:01+09:00","order":1}}}}}}"#
        ),
        created_at: "2026-06-04T21:10:01+09:00".to_owned(),
    }
}

/// Creates a sample agent-dispatch trace entry.
#[must_use]
pub fn sample_agent_dispatch_entry() -> NewDevelopmentTraceEntry {
    let text = "codegen은 development_trace 구현 결과를 반환해라.";
    let hash = trace_text_sha256_hex(text);
    NewDevelopmentTraceEntry {
        event_id: format!("{CYCLE_ID}/{USER_TURN_ID}/event-002"),
        cycle_id: CYCLE_ID.to_owned(),
        user_turn_id: Some(USER_TURN_ID.to_owned()),
        kind: DevelopmentTraceKind::AgentDispatch,
        role_name: Some("codegen".to_owned()),
        summary: "orchestra 가 codegen 에 구현 지시를 보냈다.".to_owned(),
        body: text.to_owned(),
        metadata_json: format!(
            r#"{{"trace_contract_version":2,"phase_id":"codegen-dispatch","cycle_step":2,"role":"codegen","agent_id":"agent-codegen-harness","status":"dispatched","expected_next_kind":"agent_return","content_json":{{"injected_context":"cycle context","instructions":"{text}","constraints":["stay in scope"],"expected_outputs":["implementation summary"],"context_report_requirement":"required","prompt_verbatim":{{"text":"{text}","source_type":"orchestra_dispatch","source_ref":"dispatch-codegen-harness","role":"codegen","agent_id":"agent-codegen-harness","hash_sha256":"{hash}","timestamp":"2026-06-04T21:10:02+09:00","order":2}}}}}}"#
        ),
        created_at: "2026-06-04T21:10:02+09:00".to_owned(),
    }
}

/// Creates a sample agent-return trace entry.
#[must_use]
pub fn sample_agent_return_entry() -> NewDevelopmentTraceEntry {
    let text = "domain/application/infrastructure/bootstrap 레이어로 SQLite 원장을 배치했다.";
    let hash = trace_text_sha256_hex(text);
    NewDevelopmentTraceEntry {
        event_id: format!("{CYCLE_ID}/{USER_TURN_ID}/event-003"),
        cycle_id: CYCLE_ID.to_owned(),
        user_turn_id: Some(USER_TURN_ID.to_owned()),
        kind: DevelopmentTraceKind::AgentReturn,
        role_name: Some("codegen".to_owned()),
        summary: "codegen 이 development_trace 구현 결과를 반환했다.".to_owned(),
        body: text.to_owned(),
        metadata_json: format!(
            r#"{{"trace_contract_version":2,"phase_id":"codegen-1","cycle_step":3,"role":"codegen","agent_id":"agent-codegen-harness","parent_event_id":"{CYCLE_ID}/{USER_TURN_ID}/event-002","status":"returned","result":"success","content_json":{{"returned_summary":"development_trace 구현 결과 반환","changed_files_or_scope":["domain","application","infrastructure","bootstrap"],"result":"success","context_report":"low","response_verbatim":{{"text":"{text}","source_type":"agent_result","source_ref":"return-codegen-harness","role":"codegen","agent_id":"agent-codegen-harness","hash_sha256":"{hash}","timestamp":"2026-06-04T21:10:03+09:00","order":3}}}}}}"#
        ),
        created_at: "2026-06-04T21:10:03+09:00".to_owned(),
    }
}

/// Returns the shared sample cycle id.
#[must_use]
pub fn sample_development_trace_cycle_id() -> &'static str {
    CYCLE_ID
}
