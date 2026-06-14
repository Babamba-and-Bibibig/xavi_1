//! Integration tests for the SQLite-backed development trace scenario.

use xavi_domain::development_trace::{
    DevelopmentTraceExportFormat, DevelopmentTraceFilter, DevelopmentTraceKind,
    NewDevelopmentTraceEntry, audit_development_trace_cycle, trace_text_sha256_hex,
};
use xavi_harness::TestHarness;
use xavi_harness::assertions::development_trace::{
    assert_agent_dispatch_entry, assert_agent_return_entry, assert_development_trace_jsonl_export,
    assert_development_trace_markdown_export, assert_user_query_entry,
};
use xavi_harness::fixtures::development_trace::{
    sample_agent_dispatch_entry, sample_agent_return_entry, sample_development_trace_cycle_id,
    sample_user_query_entry,
};

#[test]
fn development_trace_scenario_appends_lists_and_shows_entries() {
    let harness = TestHarness::new();
    let trace = harness.development_trace();

    let first = trace.append_entry(&sample_user_query_entry()).unwrap();
    let second = trace.append_entry(&sample_agent_dispatch_entry()).unwrap();
    let third = trace.append_entry(&sample_agent_return_entry()).unwrap();
    let listed = trace
        .list_entries(&DevelopmentTraceFilter {
            cycle_id: Some(sample_development_trace_cycle_id().to_owned()),
            kind: None,
            limit: Some(10),
        })
        .unwrap();
    let filtered = trace
        .list_entries(&DevelopmentTraceFilter {
            cycle_id: Some(sample_development_trace_cycle_id().to_owned()),
            kind: Some(DevelopmentTraceKind::AgentReturn),
            limit: Some(1),
        })
        .unwrap();
    let shown = trace.show_entry(&first.event_id).unwrap().unwrap();

    assert_user_query_entry(&first);
    assert_agent_dispatch_entry(&second);
    assert_agent_return_entry(&third);
    assert_eq!(listed.len(), 3);
    assert_eq!(listed[0].id, first.id);
    assert_eq!(listed[1].id, second.id);
    assert_eq!(listed[2].id, third.id);
    assert_eq!(shown.body, first.body);
    assert_eq!(filtered.len(), 1);
    assert_agent_return_entry(&filtered[0]);
}

#[test]
fn development_trace_scenario_exports_markdown_and_jsonl() {
    let harness = TestHarness::new();
    let trace = harness.development_trace();

    trace.append_entry(&sample_user_query_entry()).unwrap();
    trace.append_entry(&sample_agent_dispatch_entry()).unwrap();
    trace.append_entry(&sample_agent_return_entry()).unwrap();
    let filter = DevelopmentTraceFilter {
        cycle_id: Some(sample_development_trace_cycle_id().to_owned()),
        kind: None,
        limit: Some(10),
    };

    let markdown = trace.export_entries(&filter, DevelopmentTraceExportFormat::Markdown).unwrap();
    let jsonl = trace.export_entries(&filter, DevelopmentTraceExportFormat::Jsonl).unwrap();

    assert_development_trace_markdown_export(&markdown);
    assert_development_trace_jsonl_export(&jsonl);
}

#[test]
fn development_trace_scenario_rejects_v2_summary_only_user_prompt() {
    let harness = TestHarness::new();
    let trace = harness.development_trace();

    let error = trace
        .append_entry(&NewDevelopmentTraceEntry {
            event_id: "summary-only-user-query".to_owned(),
            cycle_id: sample_development_trace_cycle_id().to_owned(),
            user_turn_id: None,
            kind: DevelopmentTraceKind::UserQuery,
            role_name: Some("user".to_owned()),
            summary: "summary only".to_owned(),
            body: "summary only".to_owned(),
            metadata_json: r#"{"trace_contract_version":2,"phase_id":"user-query","cycle_step":1,"role":"user","status":"requested","content_json":{"user_request":"summary only","constraints":["strict v2"],"acceptance_criteria":["must reject missing verbatim"]}}"#.to_owned(),
            created_at: "2026-06-04T21:10:04+09:00".to_owned(),
        })
        .expect_err("v2 user query without user_request_verbatim should fail")
        .to_string();

    assert!(error.contains("missing_verbatim_evidence"));
}

#[test]
fn development_trace_scenario_rejects_v2_test_summary_without_all_command_evidence() {
    let harness = TestHarness::new();
    let trace = harness.development_trace();

    let error = trace
        .append_entry(&NewDevelopmentTraceEntry {
            event_id: "test-summary-missing-output-evidence".to_owned(),
            cycle_id: sample_development_trace_cycle_id().to_owned(),
            user_turn_id: None,
            kind: DevelopmentTraceKind::TestSummary,
            role_name: Some("test".to_owned()),
            summary: "test command evidence incomplete".to_owned(),
            body: "running tests".to_owned(),
            metadata_json: test_summary_metadata_without_output_verbatim(),
            created_at: "2026-06-13T12:00:00+09:00".to_owned(),
        })
        .expect_err("v2 test_summary missing output_verbatim should fail")
        .to_string();

    assert!(error.contains("missing_command_evidence"));
    assert!(error.contains("output_verbatim"));
}

#[test]
fn development_trace_scenario_accepts_empty_output_verbatim_text() {
    let harness = TestHarness::new();
    let trace = harness.development_trace();

    trace
        .append_entry(&NewDevelopmentTraceEntry {
            event_id: "test-summary-empty-output-evidence".to_owned(),
            cycle_id: sample_development_trace_cycle_id().to_owned(),
            user_turn_id: None,
            kind: DevelopmentTraceKind::TestSummary,
            role_name: Some("test".to_owned()),
            summary: "test command evidence with empty output".to_owned(),
            body: "command produced no terminal output".to_owned(),
            metadata_json: complete_test_summary_metadata_with_verbatim_texts(
                "cargo test -p xavi-harness",
                "exit status: 0",
                "",
            ),
            created_at: "2026-06-13T12:00:00+09:00".to_owned(),
        })
        .expect("empty output_verbatim.text should be accepted with complete metadata");
}

#[test]
fn development_trace_scenario_rejects_empty_command_and_result_verbatim_text() {
    let harness = TestHarness::new();
    let trace = harness.development_trace();

    let command_error = trace
        .append_entry(&NewDevelopmentTraceEntry {
            event_id: "test-summary-empty-command-evidence".to_owned(),
            cycle_id: sample_development_trace_cycle_id().to_owned(),
            user_turn_id: None,
            kind: DevelopmentTraceKind::TestSummary,
            role_name: Some("test".to_owned()),
            summary: "test command evidence with empty command verbatim".to_owned(),
            body: "running tests".to_owned(),
            metadata_json: complete_test_summary_metadata_with_verbatim_texts(
                "",
                "exit status: 0",
                "running tests\nok",
            ),
            created_at: "2026-06-13T12:00:00+09:00".to_owned(),
        })
        .expect_err("empty command_verbatim.text should fail")
        .to_string();

    let result_error = trace
        .append_entry(&NewDevelopmentTraceEntry {
            event_id: "test-summary-empty-result-evidence".to_owned(),
            cycle_id: sample_development_trace_cycle_id().to_owned(),
            user_turn_id: None,
            kind: DevelopmentTraceKind::TestSummary,
            role_name: Some("test".to_owned()),
            summary: "test command evidence with empty result verbatim".to_owned(),
            body: "running tests".to_owned(),
            metadata_json: complete_test_summary_metadata_with_verbatim_texts(
                "cargo test -p xavi-harness",
                "",
                "running tests\nok",
            ),
            created_at: "2026-06-13T12:00:00+09:00".to_owned(),
        })
        .expect_err("empty result_verbatim.text should fail")
        .to_string();

    assert!(command_error.contains("metadata.text must not be empty"));
    assert!(result_error.contains("metadata.text must not be empty"));
}

#[test]
fn development_trace_scenario_rejects_empty_output_verbatim_without_required_metadata() {
    let harness = TestHarness::new();
    let trace = harness.development_trace();

    let output_verbatim =
        command_evidence_without_timestamp("", "command_output", "harness-test-command#output", 6);
    let error = trace
        .append_entry(&NewDevelopmentTraceEntry {
            event_id: "test-summary-empty-output-missing-timestamp".to_owned(),
            cycle_id: sample_development_trace_cycle_id().to_owned(),
            user_turn_id: None,
            kind: DevelopmentTraceKind::TestSummary,
            role_name: Some("test".to_owned()),
            summary: "test command evidence with incomplete empty output verbatim".to_owned(),
            body: "running tests".to_owned(),
            metadata_json: test_summary_metadata_with_evidence(
                &command_evidence(
                    "cargo test -p xavi-harness",
                    "command_invocation",
                    "harness-test-command#command",
                    4,
                ),
                &command_evidence(
                    "exit status: 0",
                    "command_result",
                    "harness-test-command#result",
                    5,
                ),
                &output_verbatim,
            ),
            created_at: "2026-06-13T12:00:00+09:00".to_owned(),
        })
        .expect_err(
            "empty output_verbatim.text should still require timestamp/source/hash metadata",
        )
        .to_string();

    assert!(error.contains("missing metadata.timestamp"));
}

#[test]
fn development_trace_audit_fails_when_cycle_refs_are_missing() {
    let harness = TestHarness::new();
    let trace = harness.development_trace();

    trace.append_entry(&sample_user_query_entry()).unwrap();
    trace.append_entry(&sample_agent_dispatch_entry()).unwrap();
    trace.append_entry(&sample_agent_return_entry()).unwrap();
    trace.append_entry(&complete_test_summary_entry()).unwrap();
    trace.append_entry(&cycle_report_entry()).unwrap();

    let entries = trace
        .list_entries(&DevelopmentTraceFilter {
            cycle_id: Some(sample_development_trace_cycle_id().to_owned()),
            kind: None,
            limit: Some(10),
        })
        .unwrap();
    let report = audit_development_trace_cycle(sample_development_trace_cycle_id(), &entries);

    assert_eq!(report.status, "failed");
    assert!(report.findings.iter().any(|finding| {
        finding.severity == "failure" && finding.code == "missing_cycle_baseline_ref"
    }));
    assert!(report.findings.iter().any(|finding| {
        finding.severity == "failure" && finding.code == "missing_cycle_head_ref"
    }));
}

fn complete_test_summary_entry() -> NewDevelopmentTraceEntry {
    NewDevelopmentTraceEntry {
        event_id: "test-summary-complete-command-evidence".to_owned(),
        cycle_id: sample_development_trace_cycle_id().to_owned(),
        user_turn_id: None,
        kind: DevelopmentTraceKind::TestSummary,
        role_name: Some("test".to_owned()),
        summary: "test command evidence complete".to_owned(),
        body: "running tests\nok".to_owned(),
        metadata_json: complete_test_summary_metadata(),
        created_at: "2026-06-13T12:00:00+09:00".to_owned(),
    }
}

fn cycle_report_entry() -> NewDevelopmentTraceEntry {
    NewDevelopmentTraceEntry {
        event_id: "cycle-report-without-cycle-refs".to_owned(),
        cycle_id: sample_development_trace_cycle_id().to_owned(),
        user_turn_id: None,
        kind: DevelopmentTraceKind::OrchestraJudgment,
        role_name: Some("orchestra".to_owned()),
        summary: "cycle report completed".to_owned(),
        body: "cycle report completed".to_owned(),
        metadata_json: r#"{"trace_contract_version":1,"phase_id":"cycle-report","cycle_step":5,"role":"orchestra","status":"reported","cycle_status":"complete","content_json":{"observed_facts":["test summary evidence stored"],"decision":"approved","reasoning_summary":"cycle report fixture","next_action":"audit cycle refs"}}"#.to_owned(),
        created_at: "2026-06-13T12:00:01+09:00".to_owned(),
    }
}

fn complete_test_summary_metadata() -> String {
    complete_test_summary_metadata_with_verbatim_texts(
        "cargo test -p xavi-harness",
        "exit status: 0",
        "running tests\nok",
    )
}

fn complete_test_summary_metadata_with_verbatim_texts(
    command_text: &str,
    result_text: &str,
    output_text: &str,
) -> String {
    test_summary_metadata_with_evidence(
        &command_evidence(command_text, "command_invocation", "harness-test-command#command", 4),
        &command_evidence(result_text, "command_result", "harness-test-command#result", 5),
        &command_evidence(output_text, "command_output", "harness-test-command#output", 6),
    )
}

fn test_summary_metadata_with_evidence(
    command_verbatim: &str,
    result_verbatim: &str,
    output_verbatim: &str,
) -> String {
    format!(
        r#"{{"trace_contract_version":2,"phase_id":"test-summary","cycle_step":4,"role":"test","agent_id":null,"commands":["cargo test -p xavi-harness"],"status":"passed","result":"passed","content_json":{{"commands":["cargo test -p xavi-harness"],"result":"passed","evidence":"harness-test-command","command_record":{{"command":"cargo test -p xavi-harness","actor":"test","result":"passed","exit_code":0,"evidence_ref":"harness-test-command","command_verbatim":{command_verbatim},"result_verbatim":{result_verbatim},"output_verbatim":{output_verbatim}}},"command_verbatim":{command_verbatim},"result_verbatim":{result_verbatim},"output_verbatim":{output_verbatim}}}}}"#
    )
}

fn test_summary_metadata_without_output_verbatim() -> String {
    let command_verbatim = command_evidence(
        "cargo test -p xavi-harness",
        "command_invocation",
        "harness-test-command#command",
        4,
    );
    let result_verbatim =
        command_evidence("exit status: 0", "command_result", "harness-test-command#result", 5);

    format!(
        r#"{{"trace_contract_version":2,"phase_id":"test-summary","cycle_step":4,"role":"test","agent_id":null,"commands":["cargo test -p xavi-harness"],"status":"passed","result":"passed","content_json":{{"commands":["cargo test -p xavi-harness"],"result":"passed","evidence":"harness-test-command","command_record":{{"command":"cargo test -p xavi-harness","actor":"test","result":"passed","exit_code":0,"evidence_ref":"harness-test-command","command_verbatim":{command_verbatim},"result_verbatim":{result_verbatim}}},"command_verbatim":{command_verbatim},"result_verbatim":{result_verbatim}}}}}"#
    )
}

fn command_evidence(text: &str, source_type: &str, source_ref: &str, order: i64) -> String {
    let hash = trace_text_sha256_hex(text);
    format!(
        r#"{{"text":"{}","source_type":"{source_type}","source_ref":"{source_ref}","role":"test","agent_id":null,"hash_sha256":"{hash}","timestamp":"2026-06-13T12:00:00+09:00","order":{order}}}"#,
        json_string(text)
    )
}

fn command_evidence_without_timestamp(
    text: &str,
    source_type: &str,
    source_ref: &str,
    order: i64,
) -> String {
    let hash = trace_text_sha256_hex(text);
    format!(
        r#"{{"text":"{}","source_type":"{source_type}","source_ref":"{source_ref}","role":"test","agent_id":null,"hash_sha256":"{hash}","order":{order}}}"#,
        json_string(text)
    )
}

fn json_string(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            control if control.is_control() => {
                use std::fmt::Write as _;
                write!(&mut escaped, "\\u{:04x}", u32::from(control)).unwrap();
            }
            other => escaped.push(other),
        }
    }
    escaped
}
