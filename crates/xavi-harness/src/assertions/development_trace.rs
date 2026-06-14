//! Assertions for development trace scenarios.

use xavi_domain::development_trace::{
    DevelopmentTraceEntry, DevelopmentTraceKind, trace_text_sha256_hex,
};

/// Asserts that a user-query development trace entry was stored.
///
/// # Panics
///
/// Panics when the entry does not match the expected fixture values.
pub fn assert_user_query_entry(entry: &DevelopmentTraceEntry) {
    assert_eq!(entry.kind, DevelopmentTraceKind::UserQuery);
    assert_eq!(entry.cycle_id, "cycle-dev-test-001");
    assert!(entry.summary.contains("SQLite3"));
    assert!(entry.body.contains("development_trace"));
    assert!(entry.metadata_json.contains("\"user_request\""));
    assert!(entry.metadata_json.contains("\"acceptance_criteria\""));
    assert!(entry.metadata_json.contains("\"trace_contract_version\":2"));
    assert!(entry.metadata_json.contains("\"user_request_verbatim\""));
    assert!(
        entry.metadata_json.contains(&trace_text_sha256_hex("SQLite3 자연어 개발 trace 원장 MVP"))
    );
}

/// Asserts that an agent-dispatch development trace entry was stored.
///
/// # Panics
///
/// Panics when the entry does not match the expected fixture values.
pub fn assert_agent_dispatch_entry(entry: &DevelopmentTraceEntry) {
    assert_eq!(entry.kind, DevelopmentTraceKind::AgentDispatch);
    assert_eq!(entry.role_name.as_deref(), Some("codegen"));
    assert!(entry.summary.contains("orchestra"));
    assert!(entry.metadata_json.contains("\"trace_contract_version\":2"));
    assert!(entry.metadata_json.contains("\"prompt_verbatim\""));
    assert!(entry.metadata_json.contains("\"source_ref\":\"dispatch-codegen-harness\""));
    assert!(
        entry
            .metadata_json
            .contains(&trace_text_sha256_hex("codegen은 development_trace 구현 결과를 반환해라."))
    );
}

/// Asserts that an agent-return development trace entry was stored.
///
/// # Panics
///
/// Panics when the entry does not match the expected fixture values.
pub fn assert_agent_return_entry(entry: &DevelopmentTraceEntry) {
    assert_eq!(entry.kind, DevelopmentTraceKind::AgentReturn);
    assert_eq!(entry.role_name.as_deref(), Some("codegen"));
    assert!(entry.summary.contains("codegen"));
    assert!(entry.metadata_json.contains("\"trace_contract_version\":2"));
    assert!(entry.metadata_json.contains("\"role\":\"codegen\""));
    assert!(entry.metadata_json.contains("\"result\":\"success\""));
    assert!(entry.metadata_json.contains("\"response_verbatim\""));
    assert!(entry.metadata_json.contains(&trace_text_sha256_hex(
        "domain/application/infrastructure/bootstrap 레이어로 SQLite 원장을 배치했다."
    )));
}

/// Asserts that markdown export contains natural-language trace evidence.
///
/// # Panics
///
/// Panics when required markdown fragments are missing.
pub fn assert_development_trace_markdown_export(exported: &str) {
    assert!(exported.contains("Development Trace Export"));
    assert!(exported.contains("cycle-dev-test-001"));
    assert!(exported.contains("사용자가 SQLite3"));
    assert!(exported.contains("codegen 이 development_trace"));
}

/// Asserts that JSONL export contains escaped trace rows.
///
/// # Panics
///
/// Panics when required JSONL fragments are missing.
pub fn assert_development_trace_jsonl_export(exported: &str) {
    assert!(exported.contains("\"kind\":\"user_query\""));
    assert!(exported.contains("\"kind\":\"agent_return\""));
    assert!(exported.contains("\"kind\":\"agent_dispatch\""));
    assert!(exported.contains("\\\"user_request\\\""));
    assert!(exported.contains("\\\"user_request_verbatim\\\""));
    assert!(exported.contains("\\\"prompt_verbatim\\\""));
    assert!(exported.contains("\\\"response_verbatim\\\""));
    assert!(exported.contains("\\\"acceptance_criteria\\\""));
    assert!(exported.contains("\\\"trace_contract_version\\\":2"));
}
