//! Domain model for append-only natural-language development trace entries.

use std::fmt::Write as _;

/// Stable identifier supplied by the caller for one trace entry.
pub type DevelopmentTraceEventId = String;

/// Stable identifier for a development cycle.
pub type DevelopmentTraceCycleId = String;

/// Stable identifier for a user turn inside a development cycle.
pub type DevelopmentTraceUserTurnId = String;

/// Supported natural-language development trace entry kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopmentTraceKind {
    /// User query or response text.
    UserQuery,
    /// Orchestra judgment or routing decision.
    OrchestraJudgment,
    /// Agent or role dispatch prompt summary.
    AgentDispatch,
    /// Agent or role return summary.
    AgentReturn,
    /// File change summary.
    FileSummary,
    /// Test or verification summary.
    TestSummary,
    /// Project knowledge note.
    ProjectKnowledgeNote,
}

impl DevelopmentTraceKind {
    /// Returns the stable `SQLite` representation.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserQuery => "user_query",
            Self::OrchestraJudgment => "orchestra_judgment",
            Self::AgentDispatch => "agent_dispatch",
            Self::AgentReturn => "agent_return",
            Self::FileSummary => "file_summary",
            Self::TestSummary => "test_summary",
            Self::ProjectKnowledgeNote => "project_knowledge_note",
        }
    }

    /// Parses a caller-facing kind value.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let normalized = normalize_kind_text(value);
        match normalized.as_str() {
            "user_query" | "userquery" | "user_prompt" | "userprompt" => Some(Self::UserQuery),
            "orchestra_judgment" | "orchestrajudgment" | "orchestra_decision"
            | "orchestradecision" => Some(Self::OrchestraJudgment),
            "agent_dispatch" | "agentdispatch" | "dispatch" => Some(Self::AgentDispatch),
            "agent_return" | "agentreturn" | "return" => Some(Self::AgentReturn),
            "file_summary" | "filesummary" | "file_change_summary" | "filechangesummary" => {
                Some(Self::FileSummary)
            }
            "test_summary" | "testsummary" | "test_result_summary" | "testresultsummary" => {
                Some(Self::TestSummary)
            }
            "project_knowledge_note"
            | "projectknowledgenote"
            | "knowledge_note"
            | "knowledgenote" => Some(Self::ProjectKnowledgeNote),
            _ => None,
        }
    }
}

/// Export formats supported by the trace use case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopmentTraceExportFormat {
    /// Markdown report.
    Markdown,
    /// JSON Lines stream.
    Jsonl,
}

impl DevelopmentTraceExportFormat {
    /// Parses a caller-facing export format.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "markdown" | "md" => Some(Self::Markdown),
            "jsonl" | "json-lines" | "json_lines" => Some(Self::Jsonl),
            _ => None,
        }
    }
}

/// Caller-supplied development trace payload before persistence assigns row id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDevelopmentTraceEntry {
    /// Stable event identifier.
    pub event_id: DevelopmentTraceEventId,
    /// Parent development cycle identifier.
    pub cycle_id: DevelopmentTraceCycleId,
    /// Optional parent user-turn identifier.
    pub user_turn_id: Option<DevelopmentTraceUserTurnId>,
    /// Natural-language trace kind.
    pub kind: DevelopmentTraceKind,
    /// Optional role or agent associated with the entry.
    pub role_name: Option<String>,
    /// Short one-line summary.
    pub summary: String,
    /// Full natural-language body.
    pub body: String,
    /// Caller-supplied metadata JSON stored as a string.
    pub metadata_json: String,
    /// Timestamp string supplied by the caller.
    pub created_at: String,
}

/// Persisted append-only development trace entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentTraceEntry {
    /// `SQLite` row id assigned on append.
    pub id: i64,
    /// Stable event identifier.
    pub event_id: DevelopmentTraceEventId,
    /// Parent development cycle identifier.
    pub cycle_id: DevelopmentTraceCycleId,
    /// Optional parent user-turn identifier.
    pub user_turn_id: Option<DevelopmentTraceUserTurnId>,
    /// Natural-language trace kind.
    pub kind: DevelopmentTraceKind,
    /// Optional role or agent associated with the entry.
    pub role_name: Option<String>,
    /// Short one-line summary.
    pub summary: String,
    /// Full natural-language body.
    pub body: String,
    /// Caller-supplied metadata JSON stored as a string.
    pub metadata_json: String,
    /// Timestamp string supplied by the caller.
    pub created_at: String,
}

/// Canonical database columns derived from `metadata_json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentTraceCanonicalColumns {
    /// Contract/schema version copied from metadata.
    pub schema_version: Option<i64>,
    /// Caller-declared sequence number for the cycle.
    pub sequence_no: Option<i64>,
    /// Caller-declared phase identifier.
    pub phase: Option<String>,
    /// Parent trace event identifier, when this entry links to another entry.
    pub parent_event_id: Option<String>,
    /// Canonical JSON content used by audits and UI projections.
    pub content_json: String,
    /// Stable hash of `content_json`.
    pub content_hash: String,
    /// Source kind for recovered events or externally sourced entries.
    pub source_kind: Option<String>,
    /// Source event identifier for recovered events or externally sourced entries.
    pub source_event_id: Option<String>,
}

/// Query criteria for listing or exporting development trace entries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DevelopmentTraceFilter {
    /// Optional cycle identifier filter.
    pub cycle_id: Option<DevelopmentTraceCycleId>,
    /// Optional kind filter.
    pub kind: Option<DevelopmentTraceKind>,
    /// Optional maximum number of rows.
    pub limit: Option<usize>,
}

impl DevelopmentTraceFilter {
    /// Creates a filter that returns every entry.
    #[must_use]
    pub fn all() -> Self {
        Self::default()
    }
}

/// Legacy metadata contract version used before verbatim evidence became required.
pub const TRACE_LEGACY_CONTRACT_VERSION: i64 = 1;

/// Evidence-first metadata contract version trusted by cycle reports.
pub const TRACE_EVIDENCE_CONTRACT_VERSION: i64 = 2;

/// Current strict metadata contract version used for new orchestration traces.
pub const TRACE_CONTRACT_VERSION: i64 = TRACE_EVIDENCE_CONTRACT_VERSION;

/// Roles accepted by the trace contract.
pub const TRACE_CONTRACT_ROLES: [&str; 11] = [
    "orchestra",
    "planning",
    "codegen",
    "review",
    "test",
    "analysis",
    "user-docs",
    "ai-docs",
    "cycle-report",
    "dev-console",
    "user",
];

const CYCLE_STEP_MISSING_FINDING_LIMIT: usize = 200;
const REQUIRED_CYCLE_KINDS: [DevelopmentTraceKind; 5] = [
    DevelopmentTraceKind::UserQuery,
    DevelopmentTraceKind::OrchestraJudgment,
    DevelopmentTraceKind::AgentDispatch,
    DevelopmentTraceKind::AgentReturn,
    DevelopmentTraceKind::TestSummary,
];
const COMMAND_VERBATIM_FIELD_NAMES: [&str; 3] =
    ["command_verbatim", "result_verbatim", "output_verbatim"];
const CYCLE_BASELINE_REF_FIELD: &str = "cycle_baseline_ref";
const CYCLE_HEAD_REF_FIELD: &str = "cycle_head_ref";

#[derive(Debug, Clone, Copy, Default)]
struct RequiredCycleKindsSeen {
    flags: [bool; REQUIRED_CYCLE_KINDS.len()],
}

impl RequiredCycleKindsSeen {
    fn mark(&mut self, kind: DevelopmentTraceKind) {
        if let Some(index) = REQUIRED_CYCLE_KINDS.iter().position(|candidate| *candidate == kind) {
            self.flags[index] = true;
        }
    }

    fn iter(self) -> impl Iterator<Item = (bool, DevelopmentTraceKind)> {
        REQUIRED_CYCLE_KINDS
            .into_iter()
            .enumerate()
            .map(move |(index, kind)| (self.flags[index], kind))
    }
}

/// One trace integrity issue found by a read-only audit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentTraceAuditFinding {
    /// `failure` or `warning`.
    pub severity: &'static str,
    /// Stable machine-readable code.
    pub code: &'static str,
    /// Human-readable explanation.
    pub message: String,
    /// Related event id, when the finding is tied to one entry.
    pub event_id: Option<String>,
    /// Related trace kind, when available.
    pub kind: Option<&'static str>,
    /// Related role, when available.
    pub role: Option<String>,
}

/// Read-only integrity audit result for one cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentTraceAuditReport {
    /// Audited cycle id.
    pub cycle_id: DevelopmentTraceCycleId,
    /// Number of entries checked.
    pub checked_entry_count: usize,
    /// `passed`, `warning`, or `failed`.
    pub status: &'static str,
    /// Number of failure findings.
    pub failure_count: usize,
    /// Number of warning findings.
    pub warning_count: usize,
    /// Detailed findings.
    pub findings: Vec<DevelopmentTraceAuditFinding>,
}

impl DevelopmentTraceAuditReport {
    /// Returns true when this report contains at least one failure.
    #[must_use]
    pub fn has_failures(&self) -> bool {
        self.failure_count > 0
    }
}

/// Validates append-time trace metadata for kinds covered by the strict contract.
///
/// This is intentionally fail-closed for contract-covered entries. It does not
/// infer missing values or repair malformed metadata.
///
/// # Errors
///
/// Returns an error when `metadata_json` is malformed, required contract fields
/// are missing, enum values are unsupported, or `metadata.role` conflicts with
/// the caller-supplied role.
pub fn validate_trace_contract_append(
    kind: DevelopmentTraceKind,
    cli_role: Option<&str>,
    metadata_json: &str,
) -> Result<(), String> {
    validate_trace_entry_contract_append(kind, cli_role, "", metadata_json)
}

/// Validates append-time trace metadata and structured content for strict kinds.
///
/// This is intentionally fail-closed for contract-covered entries. It does not
/// infer missing values or repair malformed metadata.
///
/// # Errors
///
/// Returns an error when `metadata_json` is malformed, required contract fields
/// are missing, enum values are unsupported, `metadata.role` conflicts with the
/// caller-supplied role, or required content sections are missing.
pub fn validate_trace_entry_contract_append(
    kind: DevelopmentTraceKind,
    cli_role: Option<&str>,
    body: &str,
    metadata_json: &str,
) -> Result<(), String> {
    let object = parse_metadata_object(metadata_json)
        .map_err(|error| format!("invalid metadata_json: {error}"))?;
    validate_contract_object(kind, cli_role, body, &object).map(|_| ())
}

/// Validates a new user-facing append path that must be evidence-first.
///
/// Legacy v1 prompt-like records remain readable and auditable, but new append
/// callers must provide the v2 verbatim evidence contract for runtime boundary
/// events.
///
/// # Errors
///
/// Returns an error when the normal contract is invalid, or when a prompt-like
/// entry is not `trace_contract_version=2` with trusted verbatim evidence.
pub fn validate_evidence_first_trace_entry_append(
    kind: DevelopmentTraceKind,
    cli_role: Option<&str>,
    body: &str,
    metadata_json: &str,
) -> Result<(), String> {
    validate_trace_entry_contract_append(kind, cli_role, body, metadata_json)?;
    if !trusted_verbatim_evidence_required(kind) {
        return Ok(());
    }

    let object = parse_metadata_object(metadata_json)
        .map_err(|error| format!("invalid metadata_json: {error}"))?;
    match trace_contract_version(&object)? {
        Some(TRACE_EVIDENCE_CONTRACT_VERSION) => Ok(()),
        Some(version) => Err(contract_error(
            "missing_verbatim_evidence",
            &format!(
                "{} new append requires trace_contract_version=2 with trusted verbatim evidence; got trace_contract_version={version}",
                kind.as_str()
            ),
        )),
        None => Err(contract_error(
            "missing_verbatim_evidence",
            &format!(
                "{} new append requires trace_contract_version=2 with trusted verbatim evidence",
                kind.as_str()
            ),
        )),
    }
}

/// Extracts canonical database columns from metadata without inventing values.
///
/// # Errors
///
/// Returns an error when metadata is malformed or canonical metadata fields have
/// invalid types.
pub fn canonical_development_trace_columns(
    kind: DevelopmentTraceKind,
    body: &str,
    metadata_json: &str,
) -> Result<DevelopmentTraceCanonicalColumns, String> {
    let object = parse_metadata_object(metadata_json)
        .map_err(|error| format!("invalid metadata_json: {error}"))?;
    let _contract = validate_contract_object(kind, None, body, &object)?;
    let schema_version =
        optional_positive_integer(&object, "schema_version")?.or(trace_contract_version(&object)?);
    let sequence_no = optional_positive_integer(&object, "sequence_no")?
        .or(optional_positive_integer(&object, "cycle_step")?);
    let phase = optional_string(&object, "phase")?.or(optional_string(&object, "phase_id")?);
    let parent_event_id = optional_string(&object, "parent_event_id")?;
    let source_kind = optional_metadata_or_content_string(&object, "source_kind")?
        .or(verbatim_evidence_string(kind, &object, "source_type")?);
    let source_event_id = optional_metadata_or_content_string(&object, "source_event_id")?
        .or(verbatim_evidence_string(kind, &object, "source_ref")?);
    let content_json = canonical_content_json(kind, body, &object)?;
    let content_hash = verbatim_evidence_string(kind, &object, "hash_sha256")?
        .unwrap_or_else(|| legacy_content_hash(&content_json));

    Ok(DevelopmentTraceCanonicalColumns {
        schema_version,
        sequence_no,
        phase,
        parent_event_id,
        content_json,
        content_hash,
        source_kind,
        source_event_id,
    })
}

/// Runs a read-only metadata/phase integrity audit over one cycle.
#[must_use]
pub fn audit_development_trace_cycle(
    cycle_id: &str,
    entries: &[DevelopmentTraceEntry],
) -> DevelopmentTraceAuditReport {
    let mut findings = Vec::new();
    let mut contracts = Vec::new();
    let mut cycle_report_seen = false;
    let mut required_kinds_seen = RequiredCycleKindsSeen::default();

    for entry in entries {
        required_kinds_seen.mark(entry.kind);
        match contract_entry_from_trace(entry) {
            Ok(Some(contract)) => {
                cycle_report_seen |= contract.flags.cycle_report;
                contracts.push(contract);
            }
            Ok(None) => {}
            Err(message) => {
                let code = trace_contract_error_code(&message);
                findings.push(failure(
                    code,
                    message,
                    Some(entry.event_id.clone()),
                    Some(entry.kind.as_str()),
                    entry.role_name.clone(),
                ));
            }
        }
    }

    audit_dispatch_return_pairs(&contracts, &mut findings);
    audit_recovered_source_links(entries, &contracts, &mut findings);
    audit_trusted_verbatim_evidence(&contracts, &mut findings);
    audit_test_summary_command_evidence(&contracts, &mut findings);
    audit_cycle_refs(&contracts, &mut findings);
    audit_cycle_steps(&contracts, &mut findings);
    audit_required_cycle_kinds(required_kinds_seen, &mut findings);

    if !cycle_report_seen {
        findings.push(failure(
            "missing_cycle_report",
            "cycle_report contract entry was not found".to_owned(),
            None,
            None,
            None,
        ));
    }

    let failure_count = findings.iter().filter(|finding| finding.severity == "failure").count();
    let warning_count = findings.iter().filter(|finding| finding.severity == "warning").count();
    let status = if failure_count > 0 {
        "failed"
    } else if warning_count > 0 {
        "warning"
    } else {
        "passed"
    };

    DevelopmentTraceAuditReport {
        cycle_id: cycle_id.to_owned(),
        checked_entry_count: entries.len(),
        status,
        failure_count,
        warning_count,
        findings,
    }
}

/// Renders audit output for a human terminal.
#[must_use]
pub fn render_development_trace_audit_text(report: &DevelopmentTraceAuditReport) -> String {
    let mut output = String::new();
    let _ = writeln!(
        output,
        "trace audit: cycle={} status={} checked={} failures={} warnings={}",
        report.cycle_id,
        report.status,
        report.checked_entry_count,
        report.failure_count,
        report.warning_count
    );
    if report.findings.is_empty() {
        output.push_str("findings: none\n");
        return output;
    }
    output.push_str("findings:\n");
    for finding in &report.findings {
        let _ = writeln!(
            output,
            "- {} {} event={} kind={} role={} {}",
            finding.severity,
            finding.code,
            finding.event_id.as_deref().unwrap_or("-"),
            finding.kind.unwrap_or("-"),
            finding.role.as_deref().unwrap_or("-"),
            finding.message
        );
    }
    output
}

/// Renders audit output as one JSON object.
#[must_use]
pub fn render_development_trace_audit_json(report: &DevelopmentTraceAuditReport) -> String {
    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "cycle_id", &json_string(&report.cycle_id), true);
    push_json_field(&mut output, "status", &json_string(report.status), false);
    push_json_field(
        &mut output,
        "checked_entry_count",
        &report.checked_entry_count.to_string(),
        false,
    );
    push_json_field(&mut output, "failure_count", &report.failure_count.to_string(), false);
    push_json_field(&mut output, "warning_count", &report.warning_count.to_string(), false);
    output.push_str(",\"findings\":");
    push_audit_findings_json(&mut output, &report.findings);
    output.push('}');
    output
}

/// Renders audit findings as JSON Lines.
#[must_use]
pub fn render_development_trace_audit_jsonl(report: &DevelopmentTraceAuditReport) -> String {
    let mut output = String::new();
    for finding in &report.findings {
        let mut line = String::new();
        line.push('{');
        push_json_field(&mut line, "cycle_id", &json_string(&report.cycle_id), true);
        push_json_field(&mut line, "status", &json_string(report.status), false);
        push_json_field(&mut line, "severity", &json_string(finding.severity), false);
        push_json_field(&mut line, "code", &json_string(finding.code), false);
        push_json_field(
            &mut line,
            "event_id",
            &json_optional_string(finding.event_id.as_deref()),
            false,
        );
        push_json_field(&mut line, "kind", &json_optional_string(finding.kind), false);
        push_json_field(&mut line, "role", &json_optional_string(finding.role.as_deref()), false);
        push_json_field(&mut line, "message", &json_string(&finding.message), false);
        line.push('}');
        output.push_str(&line);
        output.push('\n');
    }
    if report.findings.is_empty() {
        output.push_str(&render_development_trace_audit_json(report));
        output.push('\n');
    }
    output
}

fn contract_entry_from_trace(
    entry: &DevelopmentTraceEntry,
) -> Result<Option<ContractEntry>, String> {
    let object = parse_metadata_object(&entry.metadata_json)
        .map_err(|error| format!("{} metadata_json parse failed: {error}", entry.event_id))?;
    let Some(version) = trace_contract_version(&object)? else {
        if contract_required_for_kind(entry.kind) {
            return Err(format!(
                "{} is {} without metadata.trace_contract_version",
                entry.event_id,
                entry.kind.as_str()
            ));
        }
        return Ok(None);
    };
    validate_supported_trace_contract_version(version).map_err(|error| {
        format!(
            "{} has unsupported metadata.trace_contract_version {version}: {error}",
            entry.event_id
        )
    })?;

    let contract =
        validate_contract_object(entry.kind, entry.role_name.as_deref(), &entry.body, &object)?;
    if matches!(entry.kind, DevelopmentTraceKind::AgentDispatch | DevelopmentTraceKind::AgentReturn)
        && entry.role_name.as_deref().map_or("", str::trim).is_empty()
    {
        return Err(format!("{} has contract metadata but missing row role_name", entry.event_id));
    }
    Ok(Some(ContractEntry {
        event_id: entry.event_id.clone(),
        kind: entry.kind,
        role: contract.role,
        agent_id: contract.agent_id,
        parent_event_id: contract.parent_event_id,
        cycle_step: contract.cycle_step,
        row_id: entry.id,
        record_kind: contract.record_kind,
        source_kind: contract.source_kind,
        source_event_id: contract.source_event_id,
        contract_version: contract.contract_version,
        flags: contract.flags,
    }))
}

fn validate_contract_object(
    kind: DevelopmentTraceKind,
    cli_role: Option<&str>,
    body: &str,
    object: &MetadataObject,
) -> Result<ValidatedContract, String> {
    let Some(version) = trace_contract_version(object)? else {
        if contract_required_for_kind(kind) {
            return Err(contract_error(
                "missing_content_json",
                &format!(
                    "{} metadata requires trace_contract_version={TRACE_CONTRACT_VERSION}",
                    kind.as_str()
                ),
            ));
        }
        return Ok(ValidatedContract::empty());
    };
    validate_supported_trace_contract_version(version)?;

    let phase_id = required_string(object, "phase_id")?;
    let cycle_step = required_positive_integer(object, "cycle_step")?;
    let role = required_role(object, "role")?;
    validate_cli_role_match(cli_role, &role)?;
    let status = required_string(object, "status")?;
    validate_status(&status)?;
    if let Some(decision) = optional_string(object, "decision")? {
        validate_decision(&decision)?;
    }
    let record_kind = trace_record_kind(object)?;
    let source_kind = optional_metadata_or_content_string(object, "source_kind")?;
    let source_event_id = optional_metadata_or_content_string(object, "source_event_id")?;
    validate_recovery_policy(record_kind, source_kind.as_deref(), source_event_id.as_deref())?;

    let agent_id = match kind {
        DevelopmentTraceKind::AgentDispatch | DevelopmentTraceKind::AgentReturn => {
            Some(required_string(object, "agent_id")?)
        }
        _ => optional_nullable_string(object, "agent_id")?,
    };
    validate_content_contract(
        kind,
        version,
        record_kind,
        body,
        object,
        &role,
        agent_id.as_deref(),
    )?;
    let has_command_evidence = validate_test_summary_command_evidence_contract(
        kind,
        version,
        object,
        &role,
        agent_id.as_deref(),
    )?;
    let (has_cycle_baseline_ref, has_cycle_head_ref) =
        validate_cycle_ref_contract(object, &role, agent_id.as_deref())?;

    let mut contract = ValidatedContract {
        contract_version: version,
        phase_id,
        cycle_step,
        role,
        status,
        agent_id: agent_id.clone(),
        parent_event_id: None,
        record_kind,
        source_kind,
        source_event_id,
        flags: ContractFlags {
            cycle_report: optional_string(object, "cycle_status")?.is_some(),
            evidence: ContractEvidenceFlags {
                verbatim: version == TRACE_EVIDENCE_CONTRACT_VERSION
                    && trusted_verbatim_evidence_required(kind),
                command: has_command_evidence,
            },
            cycle_refs: ContractCycleRefFlags {
                baseline: has_cycle_baseline_ref,
                head: has_cycle_head_ref,
            },
        },
    };

    match kind {
        DevelopmentTraceKind::AgentDispatch => {
            let _expected_next_kind = required_string(object, "expected_next_kind")?;
        }
        DevelopmentTraceKind::AgentReturn => {
            contract.parent_event_id = Some(required_string(object, "parent_event_id")?);
            validate_result(&required_string(object, "result")?)?;
        }
        DevelopmentTraceKind::TestSummary => {
            let commands = required_string_array(object, "commands")?;
            if commands.is_empty() {
                return Err("metadata.commands must contain at least one command".to_owned());
            }
            validate_result(&required_string(object, "result")?)?;
        }
        DevelopmentTraceKind::UserQuery | DevelopmentTraceKind::OrchestraJudgment => {}
        other if contract_required_for_kind(other) => unreachable!("covered contract kind"),
        _ => {}
    }

    if contract.phase_id.trim().is_empty() || contract.status.trim().is_empty() {
        return Err("contract metadata contains an empty required value".to_owned());
    }
    Ok(contract)
}

fn trace_contract_version(object: &MetadataObject) -> Result<Option<i64>, String> {
    let Some(value) = object.get("trace_contract_version") else {
        return Ok(None);
    };
    let MetadataValue::Number(number) = value else {
        return Err("metadata.trace_contract_version must be a JSON number".to_owned());
    };
    number
        .parse::<i64>()
        .map(Some)
        .map_err(|_| "metadata.trace_contract_version must be an integer".to_owned())
}

fn validate_supported_trace_contract_version(version: i64) -> Result<(), String> {
    match version {
        TRACE_LEGACY_CONTRACT_VERSION | TRACE_EVIDENCE_CONTRACT_VERSION => Ok(()),
        _ => Err(format!("unsupported trace_contract_version {version}")),
    }
}

fn contract_required_for_kind(kind: DevelopmentTraceKind) -> bool {
    matches!(
        kind,
        DevelopmentTraceKind::UserQuery
            | DevelopmentTraceKind::OrchestraJudgment
            | DevelopmentTraceKind::AgentDispatch
            | DevelopmentTraceKind::AgentReturn
            | DevelopmentTraceKind::TestSummary
    )
}

fn trace_record_kind(object: &MetadataObject) -> Result<TraceRecordKind, String> {
    let value = optional_string(object, "record_kind")?
        .or(optional_string(object, "trace_record_kind")?)
        .or(optional_string(object, "recovery_policy")?);
    let Some(value) = value else {
        return Ok(TraceRecordKind::ActualEvent);
    };
    TraceRecordKind::parse(&value)
        .ok_or_else(|| format!("metadata.record_kind has unsupported value {value:?}"))
}

fn validate_recovery_policy(
    record_kind: TraceRecordKind,
    source_kind: Option<&str>,
    source_event_id: Option<&str>,
) -> Result<(), String> {
    if record_kind != TraceRecordKind::RecoveredEvent {
        return Ok(());
    }
    if source_kind.is_none() || source_event_id.is_none() {
        return Err(contract_error(
            "source_missing",
            "recovered_event requires metadata.source_kind and metadata.source_event_id",
        ));
    }
    Ok(())
}

fn validate_content_contract(
    kind: DevelopmentTraceKind,
    contract_version: i64,
    record_kind: TraceRecordKind,
    body: &str,
    object: &MetadataObject,
    role: &str,
    agent_id: Option<&str>,
) -> Result<(), String> {
    match kind {
        DevelopmentTraceKind::UserQuery => {
            validate_user_query_content_contract(contract_version, object)?;
        }
        DevelopmentTraceKind::OrchestraJudgment => {
            validate_orchestra_judgment_content(record_kind, object)?;
        }
        DevelopmentTraceKind::AgentDispatch => validate_required_content_sections(
            "agent_dispatch",
            &[
                "injected_context",
                "instructions",
                "constraints",
                "expected_outputs",
                "context_report_requirement",
            ],
            object,
        )?,
        DevelopmentTraceKind::AgentReturn => validate_required_content_sections(
            "agent_return",
            &["returned_summary", "changed_files_or_scope", "result", "context_report"],
            object,
        )?,
        DevelopmentTraceKind::TestSummary => validate_required_content_sections(
            "test_summary",
            &["commands", "result", "evidence"],
            object,
        )?,
        DevelopmentTraceKind::FileSummary | DevelopmentTraceKind::ProjectKnowledgeNote => {}
    }

    if contract_version == TRACE_EVIDENCE_CONTRACT_VERSION
        && trusted_verbatim_evidence_required(kind)
    {
        validate_verbatim_evidence_contract(kind, object, role, agent_id)?;
    }

    if record_kind == TraceRecordKind::RecoveredEvent
        && !recovered_event_has_evidence(kind, body, object)
    {
        return Err(contract_error(
            "missing_content_section",
            "recovered_event requires an evidence content section",
        ));
    }

    Ok(())
}

fn validate_user_query_content_contract(
    contract_version: i64,
    object: &MetadataObject,
) -> Result<(), String> {
    if contract_version == TRACE_EVIDENCE_CONTRACT_VERSION {
        let content = required_content_json_object("user_query", object)?;
        required_verbatim_evidence_object(DevelopmentTraceKind::UserQuery, content)?;
        return Ok(());
    }

    validate_required_content_sections(
        "user_query",
        &["user_request", "constraints", "acceptance_criteria"],
        object,
    )
}

fn validate_orchestra_judgment_content(
    record_kind: TraceRecordKind,
    object: &MetadataObject,
) -> Result<(), String> {
    let normal_sections = ["observed_facts", "decision", "reasoning_summary", "next_action"];
    let content = required_content_json_object("orchestra_judgment", object)?;
    if all_content_json_sections_present(content, &normal_sections) {
        return Ok(());
    }

    match record_kind {
        TraceRecordKind::RecoveredEvent => validate_content_json_sections(
            "orchestra_judgment recovered_event",
            &["evidence", "source_kind", "source_event_id"],
            content,
        ),
        TraceRecordKind::AuditGap => validate_content_json_sections(
            "orchestra_judgment audit_gap",
            &["gap_description", "missing_event_kind", "next_action"],
            content,
        ),
        TraceRecordKind::ActualEvent => {
            validate_content_json_sections("orchestra_judgment", &normal_sections, content)
        }
    }
}

fn trusted_verbatim_evidence_required(kind: DevelopmentTraceKind) -> bool {
    matches!(
        kind,
        DevelopmentTraceKind::UserQuery
            | DevelopmentTraceKind::AgentDispatch
            | DevelopmentTraceKind::AgentReturn
    )
}

fn validate_verbatim_evidence_contract(
    kind: DevelopmentTraceKind,
    object: &MetadataObject,
    role: &str,
    agent_id: Option<&str>,
) -> Result<(), String> {
    let content = required_content_json_object(kind.as_str(), object)?;
    let (field_name, evidence) = required_verbatim_evidence_object(kind, content)?;
    validate_single_verbatim_evidence(kind, field_name, evidence, role, agent_id)
}

fn validate_test_summary_command_evidence_contract(
    kind: DevelopmentTraceKind,
    contract_version: i64,
    object: &MetadataObject,
    role: &str,
    agent_id: Option<&str>,
) -> Result<bool, String> {
    if kind != DevelopmentTraceKind::TestSummary {
        return Ok(false);
    }
    if contract_version != TRACE_EVIDENCE_CONTRACT_VERSION {
        return Ok(false);
    }

    let content = required_content_json_object(kind.as_str(), object)?;
    let command_record = required_object_field(content, "command_record", "metadata.content_json")?;
    for field_name in COMMAND_VERBATIM_FIELD_NAMES {
        let evidence = required_object_field(
            command_record,
            field_name,
            "metadata.content_json.command_record",
        )?;
        validate_single_verbatim_evidence(kind, field_name, evidence, role, agent_id)?;
    }
    Ok(true)
}

fn validate_single_verbatim_evidence(
    kind: DevelopmentTraceKind,
    field_name: &str,
    evidence: &MetadataObject,
    role: &str,
    agent_id: Option<&str>,
) -> Result<(), String> {
    let text = required_verbatim_text(kind, field_name, evidence)?;
    let source_type = required_string(evidence, "source_type")?;
    let _source_ref = required_string(evidence, "source_ref")?;
    let evidence_role = required_role(evidence, "role")?;
    let _timestamp = required_string(evidence, "timestamp")?;
    let _order = required_non_negative_integer(evidence, "order")?;
    if evidence_role != role {
        return Err(contract_error(
            "invalid_verbatim_evidence",
            &format!(
                "metadata.content_json.{field_name}.role {evidence_role:?} does not match metadata.role {role:?}"
            ),
        ));
    }
    let evidence_agent_id = required_nullable_string(evidence, "agent_id")?;
    if let Some(expected_agent_id) = agent_id {
        match evidence_agent_id.as_deref() {
            Some(actual_agent_id) if actual_agent_id == expected_agent_id => {}
            Some(actual_agent_id) => {
                return Err(contract_error(
                    "invalid_verbatim_evidence",
                    &format!(
                        "metadata.content_json.{field_name}.agent_id {actual_agent_id:?} does not match metadata.agent_id {expected_agent_id:?}"
                    ),
                ));
            }
            None => {
                return Err(contract_error(
                    "invalid_verbatim_evidence",
                    &format!(
                        "metadata.content_json.{field_name}.agent_id must match metadata.agent_id {expected_agent_id:?}"
                    ),
                ));
            }
        }
    }
    validate_source_type_for_verbatim(kind, &source_type, field_name)?;
    let hash = required_string(evidence, "hash_sha256")?;
    let expected_hash = trace_text_sha256_hex(&text);
    if hash != expected_hash {
        return Err(contract_error(
            "invalid_verbatim_hash",
            &format!("metadata.content_json.{field_name}.hash_sha256 does not match sha256(text)"),
        ));
    }
    Ok(())
}

fn required_verbatim_text(
    kind: DevelopmentTraceKind,
    field_name: &str,
    evidence: &MetadataObject,
) -> Result<String, String> {
    let Some(value) = evidence.get("text") else {
        return Err("missing metadata.text".to_owned());
    };
    let MetadataValue::String(value) = value else {
        return Err("metadata.text must be a JSON string".to_owned());
    };
    if value.trim().is_empty() && !empty_verbatim_text_allowed(kind, field_name) {
        return Err("metadata.text must not be empty".to_owned());
    }
    Ok(value.clone())
}

fn empty_verbatim_text_allowed(kind: DevelopmentTraceKind, field_name: &str) -> bool {
    kind == DevelopmentTraceKind::TestSummary && field_name == "output_verbatim"
}

fn validate_cycle_ref_contract(
    object: &MetadataObject,
    role: &str,
    agent_id: Option<&str>,
) -> Result<(bool, bool), String> {
    let Some(MetadataValue::Object(content)) = object.get("content_json") else {
        return Ok((false, false));
    };
    let has_baseline =
        validate_optional_cycle_ref(content, CYCLE_BASELINE_REF_FIELD, "baseline", role, agent_id)?;
    let has_head =
        validate_optional_cycle_ref(content, CYCLE_HEAD_REF_FIELD, "head", role, agent_id)?;
    Ok((has_baseline, has_head))
}

fn validate_optional_cycle_ref(
    content: &MetadataObject,
    field_name: &str,
    expected_ref_kind: &str,
    role: &str,
    agent_id: Option<&str>,
) -> Result<bool, String> {
    let Some(value) = content.get(field_name) else {
        return Ok(false);
    };
    let MetadataValue::Object(reference) = value else {
        return Err(contract_error(
            "invalid_cycle_ref",
            &format!("metadata.content_json.{field_name} must be a JSON object"),
        ));
    };
    validate_cycle_ref_object(field_name, expected_ref_kind, reference, role, agent_id)?;
    Ok(true)
}

fn validate_cycle_ref_object(
    field_name: &str,
    expected_ref_kind: &str,
    reference: &MetadataObject,
    role: &str,
    agent_id: Option<&str>,
) -> Result<(), String> {
    let text = required_string(reference, "text")?;
    let ref_kind = required_string(reference, "ref_kind")?;
    if ref_kind.trim().to_ascii_lowercase().replace('-', "_") != expected_ref_kind {
        return Err(contract_error(
            "invalid_cycle_ref",
            &format!("metadata.content_json.{field_name}.ref_kind must be {expected_ref_kind:?}"),
        ));
    }
    let _git_ref = required_string(reference, "git_ref")?;
    let _diff_ref = required_string(reference, "diff_ref")?;
    let _source_ref = required_string(reference, "source_ref")?;
    let evidence_role = required_role(reference, "role")?;
    if evidence_role != role {
        return Err(contract_error(
            "invalid_cycle_ref",
            &format!(
                "metadata.content_json.{field_name}.role {evidence_role:?} does not match metadata.role {role:?}"
            ),
        ));
    }
    let evidence_agent_id = required_nullable_string(reference, "agent_id")?;
    if let Some(expected_agent_id) = agent_id {
        match evidence_agent_id.as_deref() {
            Some(actual_agent_id) if actual_agent_id == expected_agent_id => {}
            Some(actual_agent_id) => {
                return Err(contract_error(
                    "invalid_cycle_ref",
                    &format!(
                        "metadata.content_json.{field_name}.agent_id {actual_agent_id:?} does not match metadata.agent_id {expected_agent_id:?}"
                    ),
                ));
            }
            None => {
                return Err(contract_error(
                    "invalid_cycle_ref",
                    &format!(
                        "metadata.content_json.{field_name}.agent_id must match metadata.agent_id {expected_agent_id:?}"
                    ),
                ));
            }
        }
    }
    let _timestamp = required_string(reference, "timestamp")?;
    let _order = required_non_negative_integer(reference, "order")?;
    let hash = required_string(reference, "hash_sha256")?;
    let expected_hash = trace_text_sha256_hex(&text);
    if hash != expected_hash {
        return Err(contract_error(
            "invalid_cycle_ref_hash",
            &format!("metadata.content_json.{field_name}.hash_sha256 does not match sha256(text)"),
        ));
    }
    Ok(())
}

fn required_object_field<'a>(
    object: &'a MetadataObject,
    key: &str,
    path: &str,
) -> Result<&'a MetadataObject, String> {
    let Some(value) = object.get(key) else {
        return Err(contract_error(
            "missing_command_evidence",
            &format!("{path}.{key} is required"),
        ));
    };
    let MetadataValue::Object(object) = value else {
        return Err(contract_error(
            "invalid_command_evidence",
            &format!("{path}.{key} must be a JSON object"),
        ));
    };
    if !metadata_object_has_content(object) {
        return Err(contract_error(
            "missing_command_evidence",
            &format!("{path}.{key} must not be empty"),
        ));
    }
    Ok(object)
}

fn required_verbatim_evidence_object(
    kind: DevelopmentTraceKind,
    content: &MetadataObject,
) -> Result<(&'static str, &MetadataObject), String> {
    for field_name in verbatim_evidence_field_names(kind) {
        if let Some(value) = content.get(field_name) {
            let MetadataValue::Object(evidence) = value else {
                return Err(contract_error(
                    "invalid_verbatim_evidence",
                    &format!("metadata.content_json.{field_name} must be a JSON object"),
                ));
            };
            if !metadata_object_has_content(evidence) {
                return Err(contract_error(
                    "missing_verbatim_evidence",
                    &format!("metadata.content_json.{field_name} must not be empty"),
                ));
            }
            return Ok((*field_name, evidence));
        }
    }
    Err(contract_error(
        "missing_verbatim_evidence",
        &format!(
            "{} requires one of {} in metadata.content_json",
            kind.as_str(),
            verbatim_evidence_field_names(kind).join(", ")
        ),
    ))
}

fn verbatim_evidence_field_names(kind: DevelopmentTraceKind) -> &'static [&'static str] {
    match kind {
        DevelopmentTraceKind::UserQuery => &["user_request_verbatim", "prompt_verbatim"],
        DevelopmentTraceKind::AgentDispatch => &["prompt_verbatim"],
        DevelopmentTraceKind::AgentReturn => &["response_verbatim", "result_verbatim"],
        DevelopmentTraceKind::TestSummary => &COMMAND_VERBATIM_FIELD_NAMES,
        DevelopmentTraceKind::OrchestraJudgment
        | DevelopmentTraceKind::FileSummary
        | DevelopmentTraceKind::ProjectKnowledgeNote => &[],
    }
}

fn validate_source_type_for_verbatim(
    kind: DevelopmentTraceKind,
    source_type: &str,
    field_name: &str,
) -> Result<(), String> {
    let normalized = source_type.trim().to_ascii_lowercase().replace('-', "_");
    let allowed = match kind {
        DevelopmentTraceKind::UserQuery => {
            &["user_prompt", "user_request", "dev_console_input"][..]
        }
        DevelopmentTraceKind::AgentDispatch => &["orchestra_dispatch", "agent_dispatch"][..],
        DevelopmentTraceKind::AgentReturn => &["agent_result", "agent_response"][..],
        DevelopmentTraceKind::TestSummary => match field_name {
            "command_verbatim" => &["command_invocation", "command_record", "terminal_command"][..],
            "result_verbatim" => &["command_result", "process_status", "exit_status"][..],
            "output_verbatim" => &["command_output", "process_output", "terminal_output"][..],
            _ => &[][..],
        },
        DevelopmentTraceKind::OrchestraJudgment
        | DevelopmentTraceKind::FileSummary
        | DevelopmentTraceKind::ProjectKnowledgeNote => &[][..],
    };
    if allowed.iter().any(|candidate| *candidate == normalized) {
        Ok(())
    } else {
        Err(contract_error(
            "invalid_verbatim_evidence",
            &format!(
                "metadata.content_json.{field_name}.source_type has unsupported value {source_type:?}"
            ),
        ))
    }
}

fn validate_content_json_sections(
    label: &str,
    sections: &[&str],
    content: &MetadataObject,
) -> Result<(), String> {
    for section in sections {
        if !metadata_object_section_has_content(content, section) {
            return Err(contract_error(
                "missing_content_section",
                &format!("{label} is missing required content section {section:?}"),
            ));
        }
    }
    Ok(())
}

fn validate_required_content_sections(
    label: &str,
    sections: &[&str],
    object: &MetadataObject,
) -> Result<(), String> {
    let content = required_content_json_object(label, object)?;
    validate_content_json_sections(label, sections, content)
}

fn all_content_json_sections_present(content: &MetadataObject, sections: &[&str]) -> bool {
    sections.iter().all(|section| metadata_object_section_has_content(content, section))
}

fn recovered_event_has_evidence(
    kind: DevelopmentTraceKind,
    body: &str,
    object: &MetadataObject,
) -> bool {
    if contract_required_for_kind(kind) {
        return object.get("content_json").is_some_and(|value| match value {
            MetadataValue::Object(content) => {
                metadata_object_section_has_content(content, "evidence")
            }
            _ => false,
        });
    }
    legacy_content_has_section(object, body, "evidence")
}

fn required_content_json_object<'a>(
    label: &str,
    object: &'a MetadataObject,
) -> Result<&'a MetadataObject, String> {
    match object.get("content_json") {
        Some(MetadataValue::Object(content)) if metadata_object_has_content(content) => Ok(content),
        Some(MetadataValue::Object(_)) => Err(contract_error(
            "missing_content_json",
            &format!("{label} requires non-empty metadata.content_json"),
        )),
        Some(_) => Err(contract_error(
            "missing_content_json",
            &format!("{label} requires metadata.content_json to be a JSON object"),
        )),
        None => Err(contract_error(
            "missing_content_json",
            &format!("{label} requires metadata.content_json"),
        )),
    }
}

fn metadata_object_section_has_content(object: &MetadataObject, section: &str) -> bool {
    object.get(section).is_some_and(metadata_value_has_content)
}

fn legacy_content_has_section(object: &MetadataObject, body: &str, section: &str) -> bool {
    object.get("content_json").is_some_and(|value| match value {
        MetadataValue::Object(content) => metadata_object_section_has_content(content, section),
        _ => false,
    }) || object.get(section).is_some_and(metadata_value_has_content)
        || body_has_section(body, section)
}

fn body_has_section(body: &str, section: &str) -> bool {
    let section_label = section.replace('_', " ");
    body.lines().any(|line| {
        let normalized = line.trim_start().trim_start_matches('#').trim().to_ascii_lowercase();
        body_line_has_section(&normalized, section)
            || body_line_has_section(&normalized, &section_label)
    })
}

fn body_line_has_section(line: &str, section: &str) -> bool {
    let Some(rest) = line.strip_prefix(section) else {
        return false;
    };
    rest.starts_with(':') || rest.starts_with(" -") || rest.starts_with(" --")
}

fn metadata_value_has_content(value: &MetadataValue) -> bool {
    match value {
        MetadataValue::String(value) | MetadataValue::Number(value) => !value.trim().is_empty(),
        MetadataValue::Bool => true,
        MetadataValue::Null => false,
        MetadataValue::Array(values) => values.iter().any(metadata_value_has_content),
        MetadataValue::Object(object) => metadata_object_has_content(object),
    }
}

fn metadata_object_has_content(object: &MetadataObject) -> bool {
    object.fields.iter().any(|(_, value)| metadata_value_has_content(value))
}

fn required_role(object: &MetadataObject, key: &str) -> Result<String, String> {
    let role = normalize_role(&required_string(object, key)?);
    if TRACE_CONTRACT_ROLES.iter().any(|known| *known == role) {
        Ok(role)
    } else {
        Err(format!("metadata.{key} has unknown role {role:?}"))
    }
}

fn validate_cli_role_match(cli_role: Option<&str>, metadata_role: &str) -> Result<(), String> {
    let Some(cli_role) = cli_role.map(normalize_role).filter(|role| !role.is_empty()) else {
        return Ok(());
    };
    if cli_role == metadata_role {
        Ok(())
    } else {
        Err(format!("metadata.role {metadata_role:?} does not match --role {cli_role:?}"))
    }
}

fn normalize_role(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('_', "-")
}

fn validate_status(value: &str) -> Result<(), String> {
    validate_enum(
        "status",
        value,
        &[
            "requested",
            "approved",
            "rejected",
            "dispatched",
            "running",
            "waiting",
            "returned",
            "complete",
            "completed",
            "passed",
            "failed",
            "timeout",
            "shutdown",
            "blocked",
            "invalid",
            "reported",
        ],
    )
}

fn validate_result(value: &str) -> Result<(), String> {
    validate_enum(
        "result",
        value,
        &[
            "success",
            "passed",
            "failed",
            "blocked",
            "partial",
            "cancelled",
            "timeout",
            "no_findings",
            "not_run",
            "interrupted",
        ],
    )
}

fn validate_decision(value: &str) -> Result<(), String> {
    validate_enum(
        "decision",
        value,
        &["approved", "rejected", "revision_requested", "blocked", "cancelled"],
    )
}

fn validate_enum(field: &str, value: &str, allowed: &[&str]) -> Result<(), String> {
    let normalized = value.trim().to_ascii_lowercase().replace('-', "_");
    if allowed.iter().any(|candidate| *candidate == normalized) {
        Ok(())
    } else {
        Err(format!("metadata.{field} has unsupported value {value:?}"))
    }
}

fn required_string(object: &MetadataObject, key: &str) -> Result<String, String> {
    let Some(value) = object.get(key) else {
        return Err(format!("missing metadata.{key}"));
    };
    let MetadataValue::String(value) = value else {
        return Err(format!("metadata.{key} must be a JSON string"));
    };
    if value.trim().is_empty() {
        return Err(format!("metadata.{key} must not be empty"));
    }
    Ok(value.clone())
}

fn required_nullable_string(object: &MetadataObject, key: &str) -> Result<Option<String>, String> {
    let Some(value) = object.get(key) else {
        return Err(format!("missing metadata.{key}"));
    };
    match value {
        MetadataValue::String(value) if !value.trim().is_empty() => Ok(Some(value.clone())),
        MetadataValue::String(_) => Err(format!("metadata.{key} must not be empty")),
        MetadataValue::Null => Ok(None),
        _ => Err(format!("metadata.{key} must be a JSON string or null")),
    }
}

fn optional_string(object: &MetadataObject, key: &str) -> Result<Option<String>, String> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    let MetadataValue::String(value) = value else {
        return Err(format!("metadata.{key} must be a JSON string"));
    };
    if value.trim().is_empty() {
        return Err(format!("metadata.{key} must not be empty"));
    }
    Ok(Some(value.clone()))
}

fn optional_nullable_string(object: &MetadataObject, key: &str) -> Result<Option<String>, String> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    match value {
        MetadataValue::String(value) if !value.trim().is_empty() => Ok(Some(value.clone())),
        MetadataValue::String(_) => Err(format!("metadata.{key} must not be empty")),
        MetadataValue::Null => Ok(None),
        _ => Err(format!("metadata.{key} must be a JSON string or null")),
    }
}

fn optional_metadata_or_content_string(
    object: &MetadataObject,
    key: &str,
) -> Result<Option<String>, String> {
    if let Some(value) = optional_string(object, key)? {
        return Ok(Some(value));
    }
    optional_content_json_string(object, key)
}

fn optional_content_json_string(
    object: &MetadataObject,
    key: &str,
) -> Result<Option<String>, String> {
    let Some(MetadataValue::Object(content)) = object.get("content_json") else {
        return Ok(None);
    };
    let Some(value) = content.get(key) else {
        return Ok(None);
    };
    let MetadataValue::String(value) = value else {
        return Err(format!("metadata.content_json.{key} must be a JSON string"));
    };
    if value.trim().is_empty() {
        return Err(format!("metadata.content_json.{key} must not be empty"));
    }
    Ok(Some(value.clone()))
}

fn verbatim_evidence_string(
    kind: DevelopmentTraceKind,
    object: &MetadataObject,
    key: &str,
) -> Result<Option<String>, String> {
    let Some(MetadataValue::Object(content)) = object.get("content_json") else {
        return Ok(None);
    };
    for field_name in verbatim_evidence_field_names(kind) {
        let Some(value) = content.get(field_name) else {
            continue;
        };
        let MetadataValue::Object(evidence) = value else {
            return Ok(None);
        };
        let Some(value) = evidence.get(key) else {
            return Ok(None);
        };
        let MetadataValue::String(value) = value else {
            return Err(format!("metadata.content_json.{field_name}.{key} must be a JSON string"));
        };
        if value.trim().is_empty() {
            return Err(format!("metadata.content_json.{field_name}.{key} must not be empty"));
        }
        return Ok(Some(value.clone()));
    }
    Ok(None)
}

fn required_positive_integer(object: &MetadataObject, key: &str) -> Result<i64, String> {
    let Some(value) = object.get(key) else {
        return Err(format!("missing metadata.{key}"));
    };
    let MetadataValue::Number(value) = value else {
        return Err(format!("metadata.{key} must be a JSON number"));
    };
    let parsed = value.parse::<i64>().map_err(|_| format!("metadata.{key} must be an integer"))?;
    if parsed <= 0 {
        return Err(format!("metadata.{key} must be positive"));
    }
    Ok(parsed)
}

fn required_non_negative_integer(object: &MetadataObject, key: &str) -> Result<i64, String> {
    let Some(value) = object.get(key) else {
        return Err(format!("missing metadata.{key}"));
    };
    let MetadataValue::Number(value) = value else {
        return Err(format!("metadata.{key} must be a JSON number"));
    };
    let parsed = value.parse::<i64>().map_err(|_| format!("metadata.{key} must be an integer"))?;
    if parsed < 0 {
        return Err(format!("metadata.{key} must be non-negative"));
    }
    Ok(parsed)
}

fn optional_positive_integer(object: &MetadataObject, key: &str) -> Result<Option<i64>, String> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    let MetadataValue::Number(value) = value else {
        return Err(format!("metadata.{key} must be a JSON number"));
    };
    let parsed = value.parse::<i64>().map_err(|_| format!("metadata.{key} must be an integer"))?;
    if parsed <= 0 {
        return Err(format!("metadata.{key} must be positive"));
    }
    Ok(Some(parsed))
}

fn required_string_array(object: &MetadataObject, key: &str) -> Result<Vec<String>, String> {
    let Some(value) = object.get(key) else {
        return Err(format!("missing metadata.{key}"));
    };
    let MetadataValue::Array(values) = value else {
        return Err(format!("metadata.{key} must be a JSON array"));
    };
    let mut output = Vec::new();
    for value in values {
        let MetadataValue::String(value) = value else {
            return Err(format!("metadata.{key} must contain only strings"));
        };
        if value.trim().is_empty() {
            return Err(format!("metadata.{key} must not contain empty strings"));
        }
        output.push(value.clone());
    }
    Ok(output)
}

fn canonical_content_json(
    kind: DevelopmentTraceKind,
    body: &str,
    object: &MetadataObject,
) -> Result<String, String> {
    if contract_required_for_kind(kind) {
        let content = required_content_json_object(kind.as_str(), object)?;
        let mut output = String::new();
        push_metadata_object_json(&mut output, content);
        return Ok(output);
    }

    if let Some(value) = object.get("content_json") {
        if !metadata_value_has_content(value) {
            return Err(contract_error(
                "missing_content_json",
                "metadata.content_json must not be empty",
            ));
        }
        let mut output = String::new();
        push_metadata_value_json(&mut output, value);
        return Ok(output);
    }

    let mut section_fields = Vec::new();
    for section in canonical_content_section_keys(kind) {
        if let Some(value) = object.get(section) {
            if metadata_value_has_content(value) {
                section_fields.push((*section, value));
            }
        }
    }
    if !section_fields.is_empty() {
        let mut output = String::new();
        output.push('{');
        for (index, (key, value)) in section_fields.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str(&json_string(key));
            output.push(':');
            push_metadata_value_json(&mut output, value);
        }
        output.push('}');
        return Ok(output);
    }

    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "body", &json_string(body), true);
    output.push('}');
    Ok(output)
}

fn canonical_content_section_keys(kind: DevelopmentTraceKind) -> &'static [&'static str] {
    match kind {
        DevelopmentTraceKind::UserQuery => {
            &["user_request", "constraints", "acceptance_criteria", "evidence"]
        }
        DevelopmentTraceKind::OrchestraJudgment => &[
            "observed_facts",
            "decision",
            "reasoning_summary",
            "next_action",
            "evidence",
            "source_kind",
            "source_event_id",
            "gap_description",
            "missing_event_kind",
        ],
        DevelopmentTraceKind::AgentDispatch => &[
            "injected_context",
            "instructions",
            "constraints",
            "expected_outputs",
            "context_report_requirement",
            "evidence",
        ],
        DevelopmentTraceKind::AgentReturn => {
            &["returned_summary", "changed_files_or_scope", "result", "context_report", "evidence"]
        }
        DevelopmentTraceKind::TestSummary => &["commands", "result", "evidence"],
        DevelopmentTraceKind::FileSummary | DevelopmentTraceKind::ProjectKnowledgeNote => &[],
    }
}

fn audit_dispatch_return_pairs(
    contracts: &[ContractEntry],
    findings: &mut Vec<DevelopmentTraceAuditFinding>,
) {
    let dispatches = contracts
        .iter()
        .filter(|contract| contract.kind == DevelopmentTraceKind::AgentDispatch)
        .collect::<Vec<_>>();
    let returns = contracts
        .iter()
        .filter(|contract| contract.kind == DevelopmentTraceKind::AgentReturn)
        .collect::<Vec<_>>();

    for agent_return in &returns {
        let Some(parent_event_id) = agent_return.parent_event_id.as_deref() else {
            findings.push(failure(
                "missing_parent_event_id",
                format!("{} has no parent_event_id", agent_return.event_id),
                Some(agent_return.event_id.clone()),
                Some(agent_return.kind.as_str()),
                Some(agent_return.role.clone()),
            ));
            continue;
        };
        let Some(dispatch) =
            dispatches.iter().find(|dispatch| dispatch.event_id == parent_event_id)
        else {
            findings.push(failure(
                "missing_parent_dispatch",
                format!(
                    "{} parent_event_id {parent_event_id:?} does not match an agent_dispatch",
                    agent_return.event_id
                ),
                Some(agent_return.event_id.clone()),
                Some(agent_return.kind.as_str()),
                Some(agent_return.role.clone()),
            ));
            continue;
        };
        if dispatch.agent_id != agent_return.agent_id {
            findings.push(failure(
                "agent_id_mismatch",
                format!(
                    "{} agent_id does not match parent dispatch {}",
                    agent_return.event_id, dispatch.event_id
                ),
                Some(agent_return.event_id.clone()),
                Some(agent_return.kind.as_str()),
                Some(agent_return.role.clone()),
            ));
        }
    }

    for dispatch in dispatches {
        if !returns.iter().any(|agent_return| {
            agent_return.parent_event_id.as_deref() == Some(dispatch.event_id.as_str())
        }) {
            findings.push(failure(
                "missing_agent_return",
                format!("{} has no matching agent_return", dispatch.event_id),
                Some(dispatch.event_id.clone()),
                Some(dispatch.kind.as_str()),
                Some(dispatch.role.clone()),
            ));
        }
    }
}

fn audit_recovered_source_links(
    entries: &[DevelopmentTraceEntry],
    contracts: &[ContractEntry],
    findings: &mut Vec<DevelopmentTraceAuditFinding>,
) {
    for contract in
        contracts.iter().filter(|contract| contract.record_kind == TraceRecordKind::RecoveredEvent)
    {
        let Some(source_event_id) = contract.source_event_id.as_deref() else {
            findings.push(failure(
                "source_missing",
                format!("{} recovered_event has no source_event_id", contract.event_id),
                Some(contract.event_id.clone()),
                Some(contract.kind.as_str()),
                Some(contract.role.clone()),
            ));
            continue;
        };
        let Some(source) = entries.iter().find(|entry| entry.event_id == source_event_id) else {
            findings.push(failure(
                "source_missing",
                format!(
                    "{} recovered_event source_event_id {source_event_id:?} does not match a stored trace entry",
                    contract.event_id
                ),
                Some(contract.event_id.clone()),
                Some(contract.kind.as_str()),
                Some(contract.role.clone()),
            ));
            continue;
        };
        let Some(source_kind) = contract.source_kind.as_deref() else {
            findings.push(failure(
                "source_missing",
                format!("{} recovered_event has no source_kind", contract.event_id),
                Some(contract.event_id.clone()),
                Some(contract.kind.as_str()),
                Some(contract.role.clone()),
            ));
            continue;
        };
        let Some(expected_kind) = DevelopmentTraceKind::parse(source_kind) else {
            findings.push(failure(
                "source_mismatch",
                format!(
                    "{} recovered_event source_kind {source_kind:?} is not a supported trace kind",
                    contract.event_id
                ),
                Some(contract.event_id.clone()),
                Some(contract.kind.as_str()),
                Some(contract.role.clone()),
            ));
            continue;
        };
        if source.kind != expected_kind {
            findings.push(failure(
                "source_mismatch",
                format!(
                    "{} recovered_event source_kind {source_kind:?} does not match source event kind {}",
                    contract.event_id,
                    source.kind.as_str()
                ),
                Some(contract.event_id.clone()),
                Some(contract.kind.as_str()),
                Some(contract.role.clone()),
            ));
        }
    }
}

fn audit_trusted_verbatim_evidence(
    contracts: &[ContractEntry],
    findings: &mut Vec<DevelopmentTraceAuditFinding>,
) {
    for contract in contracts {
        if !trusted_verbatim_evidence_required(contract.kind) {
            continue;
        }
        if contract.contract_version == TRACE_EVIDENCE_CONTRACT_VERSION
            && contract.flags.evidence.verbatim
        {
            continue;
        }
        findings.push(warning(
            "missing_verbatim_evidence",
            format!(
                "{} is {} trace_contract_version={} without trusted v2 verbatim evidence",
                contract.event_id,
                contract.kind.as_str(),
                contract.contract_version
            ),
            Some(contract.event_id.clone()),
            Some(contract.kind.as_str()),
            Some(contract.role.clone()),
        ));
    }
}

fn audit_test_summary_command_evidence(
    contracts: &[ContractEntry],
    findings: &mut Vec<DevelopmentTraceAuditFinding>,
) {
    for contract in
        contracts.iter().filter(|contract| contract.kind == DevelopmentTraceKind::TestSummary)
    {
        if contract.contract_version == TRACE_EVIDENCE_CONTRACT_VERSION
            && contract.flags.evidence.command
        {
            continue;
        }
        findings.push(failure(
            "missing_command_evidence",
            format!(
                "{} is test_summary trace_contract_version={} without command_verbatim, result_verbatim, and output_verbatim command evidence",
                contract.event_id, contract.contract_version
            ),
            Some(contract.event_id.clone()),
            Some(contract.kind.as_str()),
            Some(contract.role.clone()),
        ));
    }
}

fn audit_cycle_refs(contracts: &[ContractEntry], findings: &mut Vec<DevelopmentTraceAuditFinding>) {
    let baseline_seen = contracts.iter().any(|contract| contract.flags.cycle_refs.baseline);
    let head_seen = contracts.iter().any(|contract| contract.flags.cycle_refs.head);
    if !baseline_seen {
        findings.push(failure(
            "missing_cycle_baseline_ref",
            "cycle is missing a stored cycle_baseline_ref from trace cycle-start".to_owned(),
            None,
            Some(DevelopmentTraceKind::OrchestraJudgment.as_str()),
            None,
        ));
    }
    if !head_seen {
        findings.push(failure(
            "missing_cycle_head_ref",
            "cycle is missing a stored cycle_head_ref from trace cycle-end".to_owned(),
            None,
            Some(DevelopmentTraceKind::OrchestraJudgment.as_str()),
            None,
        ));
    }
}

fn audit_required_cycle_kinds(
    seen: RequiredCycleKindsSeen,
    findings: &mut Vec<DevelopmentTraceAuditFinding>,
) {
    for (seen, kind) in seen.iter() {
        if !seen {
            findings.push(failure(
                "missing_required_event",
                format!("cycle is missing required {} trace entry", kind.as_str()),
                None,
                Some(kind.as_str()),
                None,
            ));
        }
    }
}

fn audit_cycle_steps(
    contracts: &[ContractEntry],
    findings: &mut Vec<DevelopmentTraceAuditFinding>,
) {
    if contracts.is_empty() {
        return;
    }

    let mut ordered = contracts.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|contract| contract.row_id);
    let mut previous = 0_i64;
    for contract in &ordered {
        if contract.cycle_step < previous {
            findings.push(failure(
                "cycle_step_order",
                format!(
                    "{} has cycle_step {} after later step {}",
                    contract.event_id, contract.cycle_step, previous
                ),
                Some(contract.event_id.clone()),
                Some(contract.kind.as_str()),
                Some(contract.role.clone()),
            ));
        }
        previous = contract.cycle_step;
    }

    let mut steps = contracts.iter().map(|contract| contract.cycle_step).collect::<Vec<_>>();
    steps.sort_unstable();
    for window in steps.windows(2) {
        if window[0] == window[1] {
            findings.push(failure(
                "cycle_step_duplicate",
                format!("cycle_step {} is used by more than one contract entry", window[0]),
                None,
                None,
                None,
            ));
        }
    }
    let Some(max_step) = steps.last().copied() else {
        return;
    };
    steps.dedup();
    let mut previous = 0_i64;
    let mut missing_total = 0_u128;
    let mut emitted_missing = 0_usize;
    for step in steps {
        if step > previous.saturating_add(1) {
            let gap_start = previous + 1;
            let gap_count = u128::try_from(step - previous - 1).unwrap_or(u128::MAX);
            missing_total = missing_total.saturating_add(gap_count);
            let available = CYCLE_STEP_MISSING_FINDING_LIMIT.saturating_sub(emitted_missing);
            let emit_count = available.min(usize::try_from(gap_count).unwrap_or(usize::MAX));
            for offset in 0..emit_count {
                let expected = gap_start + i64::try_from(offset).unwrap_or(0);
                findings.push(failure(
                    "cycle_step_missing",
                    format!("cycle_step {expected} is missing"),
                    None,
                    None,
                    None,
                ));
            }
            emitted_missing += emit_count;
        }
        previous = step;
    }

    if missing_total > CYCLE_STEP_MISSING_FINDING_LIMIT as u128 {
        findings.push(failure(
            "cycle_step_missing_truncated",
            format!(
                "cycle_step missing audit truncated after {CYCLE_STEP_MISSING_FINDING_LIMIT} findings; {missing_total} missing steps up to max cycle_step {max_step}"
            ),
            None,
            None,
            None,
        ));
    }
}

fn failure(
    code: &'static str,
    message: String,
    event_id: Option<String>,
    kind: Option<&'static str>,
    role: Option<String>,
) -> DevelopmentTraceAuditFinding {
    DevelopmentTraceAuditFinding { severity: "failure", code, message, event_id, kind, role }
}

fn warning(
    code: &'static str,
    message: String,
    event_id: Option<String>,
    kind: Option<&'static str>,
    role: Option<String>,
) -> DevelopmentTraceAuditFinding {
    DevelopmentTraceAuditFinding { severity: "warning", code, message, event_id, kind, role }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TraceRecordKind {
    ActualEvent,
    RecoveredEvent,
    AuditGap,
}

impl TraceRecordKind {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "actual_event" | "event" | "original_event" => Some(Self::ActualEvent),
            "recovered_event" | "recovered" => Some(Self::RecoveredEvent),
            "audit_gap" | "gap" => Some(Self::AuditGap),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedContract {
    contract_version: i64,
    phase_id: String,
    cycle_step: i64,
    role: String,
    status: String,
    agent_id: Option<String>,
    parent_event_id: Option<String>,
    record_kind: TraceRecordKind,
    source_kind: Option<String>,
    source_event_id: Option<String>,
    flags: ContractFlags,
}

impl ValidatedContract {
    fn empty() -> Self {
        Self {
            contract_version: 0,
            phase_id: String::new(),
            cycle_step: 0,
            role: String::new(),
            status: String::new(),
            agent_id: None,
            parent_event_id: None,
            record_kind: TraceRecordKind::ActualEvent,
            source_kind: None,
            source_event_id: None,
            flags: ContractFlags::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ContractFlags {
    cycle_report: bool,
    evidence: ContractEvidenceFlags,
    cycle_refs: ContractCycleRefFlags,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ContractEvidenceFlags {
    verbatim: bool,
    command: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ContractCycleRefFlags {
    baseline: bool,
    head: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ContractEntry {
    event_id: String,
    kind: DevelopmentTraceKind,
    role: String,
    agent_id: Option<String>,
    parent_event_id: Option<String>,
    cycle_step: i64,
    row_id: i64,
    record_kind: TraceRecordKind,
    source_kind: Option<String>,
    source_event_id: Option<String>,
    contract_version: i64,
    flags: ContractFlags,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MetadataObject {
    fields: Vec<(String, MetadataValue)>,
}

impl MetadataObject {
    fn get(&self, key: &str) -> Option<&MetadataValue> {
        self.fields.iter().rev().find_map(|(candidate, value)| (candidate == key).then_some(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MetadataValue {
    String(String),
    Number(String),
    Bool,
    Null,
    Array(Vec<MetadataValue>),
    Object(MetadataObject),
}

fn parse_metadata_object(input: &str) -> Result<MetadataObject, String> {
    let mut parser = MetadataParser { input: input.as_bytes(), offset: 0 };
    let object = parser.parse_object()?;
    parser.skip_whitespace();
    if parser.offset == parser.input.len() {
        Ok(object)
    } else {
        Err("trailing bytes after JSON object".to_owned())
    }
}

struct MetadataParser<'a> {
    input: &'a [u8],
    offset: usize,
}

impl MetadataParser<'_> {
    fn parse_object(&mut self) -> Result<MetadataObject, String> {
        self.skip_whitespace();
        self.expect_byte(b'{')?;
        let mut fields = Vec::new();
        self.skip_whitespace();
        if self.consume_byte(b'}') {
            return Ok(MetadataObject { fields });
        }
        loop {
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect_byte(b':')?;
            let value = self.parse_value()?;
            fields.push((key, value));
            self.skip_whitespace();
            if self.consume_byte(b'}') {
                break;
            }
            self.expect_byte(b',')?;
        }
        Ok(MetadataObject { fields })
    }

    fn parse_array(&mut self) -> Result<Vec<MetadataValue>, String> {
        self.skip_whitespace();
        self.expect_byte(b'[')?;
        let mut values = Vec::new();
        self.skip_whitespace();
        if self.consume_byte(b']') {
            return Ok(values);
        }
        loop {
            values.push(self.parse_value()?);
            self.skip_whitespace();
            if self.consume_byte(b']') {
                break;
            }
            self.expect_byte(b',')?;
        }
        Ok(values)
    }

    fn parse_value(&mut self) -> Result<MetadataValue, String> {
        self.skip_whitespace();
        match self.peek_byte() {
            Some(b'"') => self.parse_string().map(MetadataValue::String),
            Some(b'{') => self.parse_object().map(MetadataValue::Object),
            Some(b'[') => self.parse_array().map(MetadataValue::Array),
            Some(b't') => {
                self.expect_keyword(b"true")?;
                Ok(MetadataValue::Bool)
            }
            Some(b'f') => {
                self.expect_keyword(b"false")?;
                Ok(MetadataValue::Bool)
            }
            Some(b'n') => {
                self.expect_keyword(b"null")?;
                Ok(MetadataValue::Null)
            }
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(MetadataValue::Number),
            Some(other) => Err(format!("unexpected JSON byte 0x{other:02x}")),
            None => Err("unexpected end of JSON".to_owned()),
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        self.expect_byte(b'"')?;
        let mut output = String::new();
        let mut chunk_start = self.offset;
        while let Some(byte) = self.peek_byte() {
            match byte {
                b'"' => {
                    if chunk_start < self.offset {
                        output.push_str(self.utf8_chunk(chunk_start, self.offset)?);
                    }
                    self.offset += 1;
                    return Ok(output);
                }
                b'\\' => {
                    if chunk_start < self.offset {
                        output.push_str(self.utf8_chunk(chunk_start, self.offset)?);
                    }
                    self.offset += 1;
                    output.push(self.parse_escape()?);
                    chunk_start = self.offset;
                }
                0x00..=0x1f => return Err("control character in JSON string".to_owned()),
                _ => self.offset += 1,
            }
        }
        Err("unterminated JSON string".to_owned())
    }

    fn parse_escape(&mut self) -> Result<char, String> {
        let Some(byte) = self.next_byte() else {
            return Err("unterminated JSON escape".to_owned());
        };
        match byte {
            b'"' => Ok('"'),
            b'\\' => Ok('\\'),
            b'/' => Ok('/'),
            b'b' => Ok('\u{0008}'),
            b'f' => Ok('\u{000c}'),
            b'n' => Ok('\n'),
            b'r' => Ok('\r'),
            b't' => Ok('\t'),
            b'u' => self.parse_unicode_escape(),
            other => Err(format!("unsupported JSON escape 0x{other:02x}")),
        }
    }

    fn parse_unicode_escape(&mut self) -> Result<char, String> {
        let mut codepoint = 0_u32;
        for _ in 0..4 {
            let Some(byte) = self.next_byte() else {
                return Err("unterminated unicode escape".to_owned());
            };
            codepoint =
                codepoint * 16 + u32::from(hex_value(byte).ok_or("invalid unicode escape")?);
        }
        char::from_u32(codepoint).ok_or_else(|| "invalid unicode codepoint".to_owned())
    }

    fn parse_number(&mut self) -> Result<String, String> {
        let start = self.offset;
        if self.consume_byte(b'-') && self.peek_byte().is_none() {
            return Err("invalid JSON number".to_owned());
        }
        match self.peek_byte() {
            Some(b'0') => {
                self.offset += 1;
            }
            Some(b'1'..=b'9') => {
                self.offset += 1;
                while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                    self.offset += 1;
                }
            }
            _ => return Err("invalid JSON number".to_owned()),
        }
        if self.consume_byte(b'.') {
            let digit_start = self.offset;
            while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                self.offset += 1;
            }
            if self.offset == digit_start {
                return Err("invalid JSON number fraction".to_owned());
            }
        }
        if matches!(self.peek_byte(), Some(b'e' | b'E')) {
            self.offset += 1;
            if matches!(self.peek_byte(), Some(b'+' | b'-')) {
                self.offset += 1;
            }
            let digit_start = self.offset;
            while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                self.offset += 1;
            }
            if self.offset == digit_start {
                return Err("invalid JSON number exponent".to_owned());
            }
        }
        self.utf8_chunk(start, self.offset).map(str::to_owned)
    }

    fn expect_keyword(&mut self, keyword: &[u8]) -> Result<(), String> {
        if self.input.get(self.offset..self.offset + keyword.len()) == Some(keyword) {
            self.offset += keyword.len();
            Ok(())
        } else {
            Err("invalid JSON keyword".to_owned())
        }
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), String> {
        self.skip_whitespace();
        if self.consume_byte(expected) {
            Ok(())
        } else {
            Err(format!("expected JSON byte 0x{expected:02x}"))
        }
    }

    fn consume_byte(&mut self, expected: u8) -> bool {
        if self.peek_byte() == Some(expected) {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        let byte = self.peek_byte()?;
        self.offset += 1;
        Some(byte)
    }

    fn peek_byte(&self) -> Option<u8> {
        self.input.get(self.offset).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek_byte(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.offset += 1;
        }
    }

    fn utf8_chunk(&self, start: usize, end: usize) -> Result<&str, String> {
        std::str::from_utf8(&self.input[start..end])
            .map_err(|_| "metadata_json contains invalid utf-8".to_owned())
    }
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn push_audit_findings_json(output: &mut String, findings: &[DevelopmentTraceAuditFinding]) {
    output.push('[');
    for (index, finding) in findings.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push('{');
        push_json_field(output, "severity", &json_string(finding.severity), true);
        push_json_field(output, "code", &json_string(finding.code), false);
        push_json_field(
            output,
            "event_id",
            &json_optional_string(finding.event_id.as_deref()),
            false,
        );
        push_json_field(output, "kind", &json_optional_string(finding.kind), false);
        push_json_field(output, "role", &json_optional_string(finding.role.as_deref()), false);
        push_json_field(output, "message", &json_string(&finding.message), false);
        output.push('}');
    }
    output.push(']');
}

fn trace_contract_error_code(message: &str) -> &'static str {
    for code in [
        "missing_content_section",
        "missing_content_json",
        "missing_verbatim_evidence",
        "invalid_verbatim_evidence",
        "invalid_verbatim_hash",
        "missing_command_evidence",
        "invalid_command_evidence",
        "invalid_cycle_ref",
        "invalid_cycle_ref_hash",
        "unlinked_event",
        "source_missing",
        "source_mismatch",
    ] {
        if message.starts_with(code) && message.as_bytes().get(code.len()) == Some(&b':') {
            return code;
        }
    }
    "invalid_trace_contract"
}

fn contract_error(code: &'static str, message: &str) -> String {
    format!("{code}: {message}")
}

fn push_metadata_value_json(output: &mut String, value: &MetadataValue) {
    match value {
        MetadataValue::String(value) => output.push_str(&json_string(value)),
        MetadataValue::Number(value) => output.push_str(value),
        MetadataValue::Bool => output.push_str("true"),
        MetadataValue::Null => output.push_str("null"),
        MetadataValue::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                push_metadata_value_json(output, value);
            }
            output.push(']');
        }
        MetadataValue::Object(object) => push_metadata_object_json(output, object),
    }
}

fn push_metadata_object_json(output: &mut String, object: &MetadataObject) {
    output.push('{');
    for (index, (key, value)) in object.fields.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&json_string(key));
        output.push(':');
        push_metadata_value_json(output, value);
    }
    output.push('}');
}

/// Computes the lowercase SHA-256 hex digest for stored verbatim evidence text.
#[must_use]
pub fn trace_text_sha256_hex(text: &str) -> String {
    sha256_hex(text.as_bytes())
}

fn legacy_content_hash(content_json: &str) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in content_json.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64:{hash:016x}")
}

#[allow(clippy::too_many_lines)]
fn sha256_hex(input: &[u8]) -> String {
    const INITIAL_STATE: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    const ROUND_CONSTANTS: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];

    let bit_len =
        u64::try_from(input.len()).expect("input length should fit in u64").wrapping_mul(8);
    let mut message = input.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    let mut state = INITIAL_STATE;
    for chunk in message.chunks_exact(64) {
        let mut schedule = [0_u32; 64];
        for (index, word) in chunk.chunks_exact(4).take(16).enumerate() {
            schedule[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..64 {
            let small_sigma_zero = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let small_sigma_one = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(small_sigma_zero)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(small_sigma_one);
        }

        let mut working_a = state[0];
        let mut working_b = state[1];
        let mut working_c = state[2];
        let mut working_d = state[3];
        let mut working_e = state[4];
        let mut working_f = state[5];
        let mut working_g = state[6];
        let mut working_h = state[7];

        for (index, constant) in ROUND_CONSTANTS.iter().enumerate() {
            let sum_one =
                working_e.rotate_right(6) ^ working_e.rotate_right(11) ^ working_e.rotate_right(25);
            let choose = (working_e & working_f) ^ (!working_e & working_g);
            let temp_one = working_h
                .wrapping_add(sum_one)
                .wrapping_add(choose)
                .wrapping_add(*constant)
                .wrapping_add(schedule[index]);
            let sum_zero =
                working_a.rotate_right(2) ^ working_a.rotate_right(13) ^ working_a.rotate_right(22);
            let majority =
                (working_a & working_b) ^ (working_a & working_c) ^ (working_b & working_c);
            let temp_two = sum_zero.wrapping_add(majority);

            working_h = working_g;
            working_g = working_f;
            working_f = working_e;
            working_e = working_d.wrapping_add(temp_one);
            working_d = working_c;
            working_c = working_b;
            working_b = working_a;
            working_a = temp_one.wrapping_add(temp_two);
        }

        state[0] = state[0].wrapping_add(working_a);
        state[1] = state[1].wrapping_add(working_b);
        state[2] = state[2].wrapping_add(working_c);
        state[3] = state[3].wrapping_add(working_d);
        state[4] = state[4].wrapping_add(working_e);
        state[5] = state[5].wrapping_add(working_f);
        state[6] = state[6].wrapping_add(working_g);
        state[7] = state[7].wrapping_add(working_h);
    }

    let mut output = String::with_capacity(64);
    for word in state {
        let _ = write!(output, "{word:08x}");
    }
    output
}

fn push_json_field(output: &mut String, key: &str, value: &str, first: bool) {
    if !first {
        output.push(',');
    }
    output.push_str(&json_string(key));
    output.push(':');
    output.push_str(value);
}

fn json_optional_string(value: Option<&str>) -> String {
    value.map_or_else(|| "null".to_owned(), json_string)
}

fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0c}' => escaped.push_str("\\f"),
            control if control.is_control() => {
                let _ = write!(escaped, "\\u{:04x}", u32::from(control));
            }
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

fn normalize_kind_text(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|character| match character {
            '-' | ' ' => '_',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_text_sha256_matches_known_vector() {
        assert_eq!(
            trace_text_sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn append_contract_rejects_missing_metadata_for_agent_dispatch() {
        let error = validate_trace_contract_append(
            DevelopmentTraceKind::AgentDispatch,
            Some("planning"),
            "{}",
        )
        .expect_err("dispatch without contract should fail");

        assert!(error.contains("trace_contract_version"));
    }

    #[test]
    fn append_contract_rejects_role_mismatch() {
        let error = validate_trace_contract_append(
            DevelopmentTraceKind::AgentDispatch,
            Some("codegen"),
            &dispatch_metadata("planning", 1),
        )
        .expect_err("role mismatch should fail");

        assert!(error.contains("does not match --role"));
    }

    #[test]
    fn append_contract_accepts_cycle_report_role() {
        validate_trace_contract_append(
            DevelopmentTraceKind::AgentDispatch,
            Some("cycle_report"),
            &dispatch_metadata("cycle-report", 1),
        )
        .expect("cycle-report dispatch should be a known trace role");

        validate_trace_contract_append(
            DevelopmentTraceKind::AgentReturn,
            Some("cycle-report"),
            &return_metadata("cycle-report", "dispatch-cycle-report-1", 2, "returned", "success"),
        )
        .expect("cycle-report return should be a known trace role");
    }

    #[test]
    fn append_contract_accepts_timeout_status_and_result() {
        validate_trace_contract_append(
            DevelopmentTraceKind::AgentReturn,
            Some("test"),
            &return_metadata("test", "dispatch-test-1", 2, "timeout", "timeout"),
        )
        .expect("timeout status/result should be explicit contract values");
    }

    #[test]
    fn append_contract_rejects_wait_timeout_status_alias() {
        let error = validate_trace_contract_append(
            DevelopmentTraceKind::AgentReturn,
            Some("test"),
            &return_metadata("test", "dispatch-test-1", 2, "wait_timeout", "timeout"),
        )
        .expect_err("wait_timeout is not an accepted contract value");

        assert!(error.contains("unsupported value"));
        assert!(error.contains("wait_timeout"));
    }

    #[test]
    fn complete_contract_cycle_audit_passes() {
        let entries = vec![
            trace_entry(
                1,
                "user-query-1",
                DevelopmentTraceKind::UserQuery,
                Some("user"),
                &user_query_metadata(1),
            ),
            trace_entry(
                2,
                "dispatch-codegen-1",
                DevelopmentTraceKind::AgentDispatch,
                Some("codegen"),
                &dispatch_metadata("codegen", 2),
            ),
            trace_entry(
                3,
                "return-codegen-1",
                DevelopmentTraceKind::AgentReturn,
                Some("codegen"),
                &return_metadata("codegen", "dispatch-codegen-1", 3, "returned", "success"),
            ),
            trace_entry(
                4,
                "cycle-start-1",
                DevelopmentTraceKind::OrchestraJudgment,
                Some("orchestra"),
                &cycle_ref_metadata(CYCLE_BASELINE_REF_FIELD, "baseline", 4),
            ),
            trace_entry(
                5,
                "test-summary-1",
                DevelopmentTraceKind::TestSummary,
                Some("test"),
                &test_summary_metadata(5),
            ),
            trace_entry(
                6,
                "cycle-report-1",
                DevelopmentTraceKind::OrchestraJudgment,
                Some("orchestra"),
                &orchestra_report_metadata(6),
            ),
            trace_entry(
                7,
                "cycle-end-1",
                DevelopmentTraceKind::OrchestraJudgment,
                Some("orchestra"),
                &cycle_ref_metadata(CYCLE_HEAD_REF_FIELD, "head", 7),
            ),
        ];

        let report = audit_development_trace_cycle("cycle-test", &entries);

        assert_eq!(report.status, "passed");
        assert_eq!(report.failure_count, 0);
        assert_eq!(report.warning_count, 0);
    }

    #[test]
    fn audit_fails_when_dispatch_has_no_matching_return() {
        let entries = vec![trace_entry(
            1,
            "dispatch-codegen-1",
            DevelopmentTraceKind::AgentDispatch,
            Some("codegen"),
            &dispatch_metadata("codegen", 1),
        )];

        let report = audit_development_trace_cycle("cycle-test", &entries);

        assert_eq!(report.status, "failed");
        assert!(report.findings.iter().any(|finding| finding.code == "missing_agent_return"));
    }

    #[test]
    fn audit_fails_when_cycle_report_is_missing() {
        let entries = vec![
            trace_entry(
                1,
                "user-query-1",
                DevelopmentTraceKind::UserQuery,
                Some("user"),
                &user_query_metadata(1),
            ),
            trace_entry(
                2,
                "dispatch-codegen-1",
                DevelopmentTraceKind::AgentDispatch,
                Some("codegen"),
                &dispatch_metadata("codegen", 2),
            ),
            trace_entry(
                3,
                "return-codegen-1",
                DevelopmentTraceKind::AgentReturn,
                Some("codegen"),
                &return_metadata("codegen", "dispatch-codegen-1", 3, "returned", "success"),
            ),
            trace_entry(
                4,
                "test-summary-1",
                DevelopmentTraceKind::TestSummary,
                Some("test"),
                &test_summary_metadata(4),
            ),
        ];

        let report = audit_development_trace_cycle("cycle-test", &entries);

        assert_eq!(report.status, "failed");
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == "missing_cycle_report"
                    && finding.severity == "failure")
        );
    }

    #[test]
    fn audit_caps_large_cycle_step_missing_findings() {
        let final_step =
            i64::try_from(CYCLE_STEP_MISSING_FINDING_LIMIT + 11).expect("test step should fit");
        let entries = vec![
            trace_entry(
                1,
                "user-query-1",
                DevelopmentTraceKind::UserQuery,
                Some("user"),
                &user_query_metadata(1),
            ),
            trace_entry(
                2,
                "cycle-report-large-gap",
                DevelopmentTraceKind::OrchestraJudgment,
                Some("orchestra"),
                &orchestra_report_metadata(final_step),
            ),
        ];

        let report = audit_development_trace_cycle("cycle-test", &entries);
        let missing_count =
            report.findings.iter().filter(|finding| finding.code == "cycle_step_missing").count();

        assert_eq!(missing_count, CYCLE_STEP_MISSING_FINDING_LIMIT);
        assert!(report.findings.iter().any(|finding| {
            finding.code == "cycle_step_missing_truncated"
                && finding.message.contains("missing audit truncated")
        }));
    }

    #[test]
    fn audit_fails_legacy_agent_return_without_contract() {
        let entries = vec![trace_entry(
            1,
            "return-codegen-legacy",
            DevelopmentTraceKind::AgentReturn,
            Some("codegen"),
            "{}",
        )];

        let report = audit_development_trace_cycle("cycle-test", &entries);

        assert_eq!(report.status, "failed");
        assert!(report.findings.iter().any(|finding| finding.code == "invalid_trace_contract"
            && finding.message.contains("trace_contract_version")));
    }

    #[test]
    fn recovered_event_requires_source_and_evidence() {
        let error = validate_trace_contract_append(
            DevelopmentTraceKind::OrchestraJudgment,
            Some("orchestra"),
            r#"{"trace_contract_version":1,"phase_id":"recovery-1","cycle_step":1,"role":"orchestra","status":"reported","record_kind":"recovered_event","content_json":{"evidence":"existing trace row confirms this"}}"#,
        )
        .expect_err("recovered_event without source should fail");

        assert!(error.contains("source_missing"));
    }

    #[test]
    fn audit_gap_allows_missing_evidence_without_recovery() {
        validate_trace_contract_append(
            DevelopmentTraceKind::OrchestraJudgment,
            Some("orchestra"),
            r#"{"trace_contract_version":1,"phase_id":"audit-gap-1","cycle_step":1,"role":"orchestra","status":"reported","record_kind":"audit_gap","content_json":{"gap_description":"return trace was not present","missing_event_kind":"agent_return","next_action":"show audit failure"}}"#,
        )
        .expect("audit_gap should not require recovery evidence");
    }

    #[test]
    fn strict_contract_kinds_reject_missing_required_content_sections() {
        for (kind, role, metadata) in [
            (
                DevelopmentTraceKind::UserQuery,
                Some("user"),
                r#"{"trace_contract_version":1,"phase_id":"user-query","cycle_step":1,"role":"user","status":"requested","content_json":{"user_request":"go"}}"#,
            ),
            (
                DevelopmentTraceKind::OrchestraJudgment,
                Some("orchestra"),
                r#"{"trace_contract_version":1,"phase_id":"orchestra-1","cycle_step":1,"role":"orchestra","status":"reported","content_json":{"decision":"approved"}}"#,
            ),
            (
                DevelopmentTraceKind::AgentDispatch,
                Some("codegen"),
                r#"{"trace_contract_version":1,"phase_id":"codegen-1","cycle_step":1,"role":"codegen","agent_id":"agent-codegen-1","status":"dispatched","expected_next_kind":"agent_return","content_json":{"instructions":"implement"}}"#,
            ),
            (
                DevelopmentTraceKind::AgentReturn,
                Some("codegen"),
                r#"{"trace_contract_version":1,"phase_id":"codegen-1","cycle_step":1,"role":"codegen","agent_id":"agent-codegen-1","parent_event_id":"dispatch-codegen-1","status":"returned","result":"success","content_json":{"returned_summary":"done"}}"#,
            ),
            (
                DevelopmentTraceKind::TestSummary,
                Some("test"),
                r#"{"trace_contract_version":1,"phase_id":"test-1","cycle_step":1,"role":"test","commands":["cargo test -p xavi-domain"],"status":"passed","result":"passed","content_json":{"result":"passed"}}"#,
            ),
        ] {
            let error = validate_trace_contract_append(kind, role, metadata)
                .expect_err("incomplete content contract should fail closed");

            assert!(
                error.contains("missing_content_section"),
                "unexpected error for {}: {error}",
                kind.as_str()
            );
        }
    }

    #[test]
    fn strict_contract_rejects_body_only_sections_without_content_json() {
        let error = validate_trace_entry_contract_append(
            DevelopmentTraceKind::UserQuery,
            Some("user"),
            "user_request: go\nconstraints: strict metadata\nacceptance_criteria: entry stored",
            r#"{"trace_contract_version":1,"phase_id":"user-query","cycle_step":1,"role":"user","status":"requested"}"#,
        )
        .expect_err("strict trace content must be metadata.content_json, not body-only labels");

        assert!(error.contains("missing_content_json"));
    }

    #[test]
    fn recovered_event_accepts_source_fields_from_content_json() {
        validate_trace_contract_append(
            DevelopmentTraceKind::OrchestraJudgment,
            Some("orchestra"),
            r#"{"trace_contract_version":1,"phase_id":"recovery-1","cycle_step":1,"role":"orchestra","status":"reported","record_kind":"recovered_event","content_json":{"evidence":"stored source row confirms this","source_kind":"user_query","source_event_id":"user-query-1"}}"#,
        )
        .expect("recovered_event can declare source link inside content_json");
    }

    #[test]
    fn audit_reports_source_missing_and_source_mismatch_for_recovered_events() {
        let missing_source_entries = vec![trace_entry(
            1,
            "recovered-missing-source",
            DevelopmentTraceKind::OrchestraJudgment,
            Some("orchestra"),
            &recovered_event_metadata(1, "user_query", "missing-user-query"),
        )];

        let missing_source_report =
            audit_development_trace_cycle("cycle-test", &missing_source_entries);

        assert!(
            missing_source_report.findings.iter().any(|finding| finding.code == "source_missing")
        );

        let mismatch_entries = vec![
            trace_entry(
                1,
                "user-query-1",
                DevelopmentTraceKind::UserQuery,
                Some("user"),
                &user_query_metadata(1),
            ),
            trace_entry(
                2,
                "recovered-mismatch",
                DevelopmentTraceKind::OrchestraJudgment,
                Some("orchestra"),
                &recovered_event_metadata(2, "agent_return", "user-query-1"),
            ),
        ];

        let mismatch_report = audit_development_trace_cycle("cycle-test", &mismatch_entries);

        assert!(mismatch_report.findings.iter().any(|finding| finding.code == "source_mismatch"));
    }

    #[test]
    fn canonical_columns_extract_content_and_source_contract_fields() {
        let canonical = canonical_development_trace_columns(
            DevelopmentTraceKind::OrchestraJudgment,
            "body",
            &recovered_event_metadata(7, "user_query", "user-query-1"),
        )
        .expect("canonical columns should derive from complete recovered_event contract");

        assert_eq!(canonical.schema_version, Some(1));
        assert_eq!(canonical.sequence_no, Some(7));
        assert_eq!(canonical.phase.as_deref(), Some("recovery-1"));
        assert_eq!(canonical.source_kind.as_deref(), Some("user_query"));
        assert_eq!(canonical.source_event_id.as_deref(), Some("user-query-1"));
        assert!(canonical.content_json.contains("\"evidence\""));
        assert!(canonical.content_hash.starts_with("fnv1a64:"));
    }

    #[test]
    fn canonical_columns_use_verbatim_text_hash_for_evidence_first_user_query() {
        let text = "원문 사용자 요청 전체";
        let metadata = user_query_metadata_with_text(1, text);
        let canonical =
            canonical_development_trace_columns(DevelopmentTraceKind::UserQuery, text, &metadata)
                .expect("evidence-first user query should produce canonical columns");

        assert_eq!(canonical.schema_version, Some(2));
        assert_eq!(canonical.source_kind.as_deref(), Some("user_prompt"));
        assert_eq!(canonical.source_event_id.as_deref(), Some("source://user-query/1"));
        assert_eq!(canonical.content_hash, trace_text_sha256_hex(text));
        assert!(canonical.content_json.contains("\"user_request_verbatim\""));
    }

    #[test]
    fn evidence_first_append_rejects_legacy_summary_only_user_query() {
        let error = validate_evidence_first_trace_entry_append(
            DevelopmentTraceKind::UserQuery,
            Some("user"),
            "summary only",
            &legacy_user_query_metadata(1),
        )
        .expect_err("new append path should reject v1 prompt-like events");

        assert!(error.contains("missing_verbatim_evidence"));
        assert!(error.contains("trace_contract_version=2"));
    }

    #[test]
    fn evidence_first_user_query_accepts_only_user_request_verbatim() {
        let metadata = user_query_metadata_with_verbatim_field(
            1,
            "user_request_verbatim",
            "source://user-query/1",
        );

        validate_evidence_first_trace_entry_append(
            DevelopmentTraceKind::UserQuery,
            Some("user"),
            "append raw user request",
            &metadata,
        )
        .expect("v2 user_query should accept user_request_verbatim without legacy sections");
    }

    #[test]
    fn evidence_first_user_query_accepts_only_prompt_verbatim() {
        let metadata =
            user_query_metadata_with_verbatim_field(1, "prompt_verbatim", "source://user-query/1");

        validate_evidence_first_trace_entry_append(
            DevelopmentTraceKind::UserQuery,
            Some("user"),
            "append raw user request",
            &metadata,
        )
        .expect("v2 user_query should accept prompt_verbatim without legacy sections");
    }

    #[test]
    fn evidence_first_user_query_rejects_v2_without_verbatim() {
        let error = validate_evidence_first_trace_entry_append(
            DevelopmentTraceKind::UserQuery,
            Some("user"),
            "summary only",
            r#"{"trace_contract_version":2,"phase_id":"user-query","cycle_step":1,"role":"user","status":"requested","content_json":{"user_request":"summary only","constraints":["legacy"],"acceptance_criteria":["stored"]}}"#,
        )
        .expect_err("v2 user_query without verbatim evidence should fail");

        assert!(error.contains("missing_verbatim_evidence"));
    }

    #[test]
    fn legacy_user_query_contract_still_accepts_required_sections() {
        validate_trace_contract_append(
            DevelopmentTraceKind::UserQuery,
            Some("user"),
            &legacy_user_query_metadata(1),
        )
        .expect("v1 user_query with legacy sections should remain compatible");
    }

    #[test]
    fn evidence_first_dispatch_rejects_bad_prompt_verbatim_hash() {
        let metadata = dispatch_metadata_with_hash("codegen", 1, "not-the-real-hash");
        let error = validate_trace_contract_append(
            DevelopmentTraceKind::AgentDispatch,
            Some("codegen"),
            &metadata,
        )
        .expect_err("bad prompt_verbatim hash should fail");

        assert!(error.contains("invalid_verbatim_hash"));
    }

    #[test]
    fn evidence_first_user_query_rejects_missing_verbatim_timestamp() {
        let metadata = user_query_metadata(1).replace(r#","timestamp":"unix:1","order":1"#, "");
        let error = validate_trace_contract_append(
            DevelopmentTraceKind::UserQuery,
            Some("user"),
            &metadata,
        )
        .expect_err("v2 user_request_verbatim without timestamp/order should fail");

        assert!(error.contains("missing metadata.timestamp"));
    }

    #[test]
    fn legacy_summary_only_user_query_is_audit_visible_missing_evidence() {
        let entries = vec![trace_entry(
            1,
            "user-query-legacy",
            DevelopmentTraceKind::UserQuery,
            Some("user"),
            r#"{"trace_contract_version":1,"phase_id":"user-query","cycle_step":1,"role":"user","status":"requested","content_json":{"user_request":"summary only","constraints":["legacy"],"acceptance_criteria":["stored"]}}"#,
        )];

        let report = audit_development_trace_cycle("cycle-test", &entries);

        assert_eq!(report.status, "failed");
        assert!(report.findings.iter().any(|finding| {
            finding.code == "missing_verbatim_evidence" && finding.severity == "warning"
        }));
    }

    #[test]
    fn legacy_summary_only_prompt_like_cycle_audit_remains_readable_with_warnings() {
        let entries = vec![
            trace_entry(
                1,
                "user-query-legacy",
                DevelopmentTraceKind::UserQuery,
                Some("user"),
                &legacy_user_query_metadata(1),
            ),
            trace_entry(
                2,
                "dispatch-codegen-legacy",
                DevelopmentTraceKind::AgentDispatch,
                Some("codegen"),
                &legacy_dispatch_metadata("codegen", 2),
            ),
            trace_entry(
                3,
                "return-codegen-legacy",
                DevelopmentTraceKind::AgentReturn,
                Some("codegen"),
                &legacy_return_metadata("codegen", "dispatch-codegen-legacy", 3),
            ),
            trace_entry(
                4,
                "cycle-start-legacy",
                DevelopmentTraceKind::OrchestraJudgment,
                Some("orchestra"),
                &cycle_ref_metadata(CYCLE_BASELINE_REF_FIELD, "baseline", 4),
            ),
            trace_entry(
                5,
                "test-summary-legacy",
                DevelopmentTraceKind::TestSummary,
                Some("test"),
                &test_summary_metadata(5),
            ),
            trace_entry(
                6,
                "cycle-report-legacy",
                DevelopmentTraceKind::OrchestraJudgment,
                Some("orchestra"),
                &orchestra_report_metadata(6),
            ),
            trace_entry(
                7,
                "cycle-end-legacy",
                DevelopmentTraceKind::OrchestraJudgment,
                Some("orchestra"),
                &cycle_ref_metadata(CYCLE_HEAD_REF_FIELD, "head", 7),
            ),
        ];

        let report = audit_development_trace_cycle("cycle-test", &entries);

        assert_eq!(report.status, "warning");
        assert_eq!(report.failure_count, 0);
        assert_eq!(
            report
                .findings
                .iter()
                .filter(|finding| finding.code == "missing_verbatim_evidence"
                    && finding.severity == "warning")
                .count(),
            3
        );
    }

    #[test]
    fn canonical_columns_fail_closed_for_strict_entry_without_content() {
        let error = canonical_development_trace_columns(
            DevelopmentTraceKind::UserQuery,
            "body without required section labels",
            r#"{"trace_contract_version":1,"phase_id":"user-query","cycle_step":1,"role":"user","status":"requested"}"#,
        )
        .expect_err("canonical extraction should not invent strict content");

        assert!(error.contains("missing_content_json"));
    }

    fn trace_entry(
        id: i64,
        event_id: &str,
        kind: DevelopmentTraceKind,
        role_name: Option<&str>,
        metadata_json: &str,
    ) -> DevelopmentTraceEntry {
        DevelopmentTraceEntry {
            id,
            event_id: event_id.to_owned(),
            cycle_id: "cycle-test".to_owned(),
            user_turn_id: None,
            kind,
            role_name: role_name.map(str::to_owned),
            summary: event_id.to_owned(),
            body: event_id.to_owned(),
            metadata_json: metadata_json.to_owned(),
            created_at: "unix:1".to_owned(),
        }
    }

    fn user_query_metadata(step: i64) -> String {
        user_query_metadata_with_text(step, "implement trace DB contract")
    }

    fn user_query_metadata_with_text(step: i64, text: &str) -> String {
        let hash = trace_text_sha256_hex(text);
        format!(
            r#"{{"trace_contract_version":2,"phase_id":"user-query","cycle_step":{step},"role":"user","status":"requested","content_json":{{"user_request":{},"constraints":["append only"],"acceptance_criteria":["strict audit failures"],"user_request_verbatim":{{"text":{},"source_type":"user_prompt","source_ref":"source://user-query/{step}","role":"user","agent_id":null,"hash_sha256":"{hash}","timestamp":"unix:{step}","order":{step}}}}}}}"#,
            json_string(text),
            json_string(text)
        )
    }

    fn user_query_metadata_with_verbatim_field(
        step: i64,
        field_name: &str,
        source_ref: &str,
    ) -> String {
        let text = "append raw user request";
        let hash = trace_text_sha256_hex(text);
        format!(
            r#"{{"trace_contract_version":2,"phase_id":"user-query","cycle_step":{step},"role":"user","status":"requested","content_json":{{"{field_name}":{{"text":{},"source_type":"user_prompt","source_ref":"{source_ref}","role":"user","agent_id":null,"hash_sha256":"{hash}","timestamp":"unix:{step}","order":{step}}}}}}}"#,
            json_string(text)
        )
    }

    fn legacy_user_query_metadata(step: i64) -> String {
        format!(
            r#"{{"trace_contract_version":1,"phase_id":"user-query","cycle_step":{step},"role":"user","status":"requested","content_json":{{"user_request":"summary only","constraints":["legacy"],"acceptance_criteria":["stored"]}}}}"#
        )
    }

    fn dispatch_metadata(role: &str, step: i64) -> String {
        let text = "implement scoped change";
        let hash = trace_text_sha256_hex(text);
        dispatch_metadata_with_hash(role, step, &hash)
    }

    fn dispatch_metadata_with_hash(role: &str, step: i64, hash: &str) -> String {
        let text = "implement scoped change";
        format!(
            r#"{{"trace_contract_version":2,"phase_id":"{role}-1","cycle_step":{step},"role":"{role}","agent_id":"agent-{role}-1","status":"dispatched","expected_next_kind":"agent_return","content_json":{{"injected_context":"cycle context","instructions":{},"constraints":["no fallback"],"expected_outputs":["changed files"],"context_report_requirement":"required","prompt_verbatim":{{"text":{},"source_type":"orchestra_dispatch","source_ref":"source://dispatch/{role}/{step}","role":"{role}","agent_id":"agent-{role}-1","hash_sha256":"{hash}","timestamp":"unix:{step}","order":{step}}}}}}}"#,
            json_string(text),
            json_string(text)
        )
    }

    fn legacy_dispatch_metadata(role: &str, step: i64) -> String {
        format!(
            r#"{{"trace_contract_version":1,"phase_id":"{role}-1","cycle_step":{step},"role":"{role}","agent_id":"agent-{role}-1","status":"dispatched","expected_next_kind":"agent_return","content_json":{{"injected_context":"cycle context","instructions":"implement scoped change","constraints":["legacy"],"expected_outputs":["summary"],"context_report_requirement":"required"}}}}"#
        )
    }

    fn return_metadata(
        role: &str,
        parent_event_id: &str,
        step: i64,
        status: &str,
        result: &str,
    ) -> String {
        let text = "returned";
        let hash = trace_text_sha256_hex(text);
        format!(
            r#"{{"trace_contract_version":2,"phase_id":"{role}-1","cycle_step":{step},"role":"{role}","agent_id":"agent-{role}-1","parent_event_id":"{parent_event_id}","status":"{status}","result":"{result}","content_json":{{"returned_summary":{},"changed_files_or_scope":["trace files"],"result":"{result}","context_report":"low","response_verbatim":{{"text":{},"source_type":"agent_result","source_ref":"source://return/{role}/{step}","role":"{role}","agent_id":"agent-{role}-1","hash_sha256":"{hash}","timestamp":"unix:{step}","order":{step}}}}}}}"#,
            json_string(text),
            json_string(text)
        )
    }

    fn legacy_return_metadata(role: &str, parent_event_id: &str, step: i64) -> String {
        format!(
            r#"{{"trace_contract_version":1,"phase_id":"{role}-1","cycle_step":{step},"role":"{role}","agent_id":"agent-{role}-1","parent_event_id":"{parent_event_id}","status":"returned","result":"success","content_json":{{"returned_summary":"done","changed_files_or_scope":["trace files"],"result":"success","context_report":"low"}}}}"#
        )
    }

    fn test_summary_metadata(step: i64) -> String {
        let command = "cargo test -p xavi-bootstrap";
        let result_text = "exit status: 0";
        let output = "test output passed";
        let command_verbatim = test_verbatim_evidence(
            command,
            "command_invocation",
            &format!("source://test/{step}#command"),
            "test",
            None,
            step,
        );
        let result_verbatim = test_verbatim_evidence(
            result_text,
            "command_result",
            &format!("source://test/{step}#result"),
            "test",
            None,
            step + 1,
        );
        let output_verbatim = test_verbatim_evidence(
            output,
            "command_output",
            &format!("source://test/{step}#output"),
            "test",
            None,
            step + 2,
        );
        format!(
            r#"{{"trace_contract_version":2,"phase_id":"test-1","cycle_step":{step},"role":"test","agent_id":null,"commands":[{}],"status":"passed","result":"passed","content_json":{{"commands":[{}],"result":"passed","evidence":"source://test/{step}","command_record":{{"command":{},"actor":"test","result":"passed","exit_code":0,"evidence_ref":"source://test/{step}","command_verbatim":{command_verbatim},"result_verbatim":{result_verbatim},"output_verbatim":{output_verbatim}}},"command_verbatim":{command_verbatim},"result_verbatim":{result_verbatim},"output_verbatim":{output_verbatim}}}}}"#,
            json_string(command),
            json_string(command),
            json_string(command)
        )
    }

    fn test_verbatim_evidence(
        text: &str,
        source_type: &str,
        source_ref: &str,
        role: &str,
        agent_id: Option<&str>,
        order: i64,
    ) -> String {
        let hash = trace_text_sha256_hex(text);
        format!(
            r#"{{"text":{},"source_type":"{source_type}","source_ref":"{source_ref}","role":"{role}","agent_id":{},"hash_sha256":"{hash}","timestamp":"unix:{order}","order":{order}}}"#,
            json_string(text),
            agent_id.map_or_else(|| "null".to_owned(), json_string)
        )
    }

    fn orchestra_report_metadata(step: i64) -> String {
        format!(
            r#"{{"trace_contract_version":1,"phase_id":"cycle-report","cycle_step":{step},"role":"orchestra","status":"reported","cycle_status":"complete","content_json":{{"observed_facts":["all roles returned"],"decision":"approved","reasoning_summary":"contract cycle complete","next_action":"final report"}}}}"#
        )
    }

    fn cycle_ref_metadata(field_name: &str, ref_kind: &str, step: i64) -> String {
        let git_ref = format!("{ref_kind}-ref");
        let diff_ref = format!("diff://{ref_kind}");
        let source_ref = format!("source://cycle-test/{ref_kind}");
        let text = format!("{ref_kind}_ref={git_ref}\ndiff_ref={diff_ref}");
        let hash = trace_text_sha256_hex(&text);
        format!(
            r#"{{"trace_contract_version":2,"phase_id":"cycle-{ref_kind}","cycle_step":{step},"role":"orchestra","agent_id":null,"status":"reported","source_kind":"cycle_ref","source_event_id":"{source_ref}","content_json":{{"observed_facts":["cycle {ref_kind} ref stored"],"decision":"approved","reasoning_summary":"cycle ref fixture","next_action":"continue","{field_name}":{{"text":{},"ref_kind":"{ref_kind}","git_ref":"{git_ref}","diff_ref":"{diff_ref}","source_ref":"{source_ref}","role":"orchestra","agent_id":null,"hash_sha256":"{hash}","timestamp":"unix:{step}","order":{step}}}}}}}"#,
            json_string(&text)
        )
    }

    fn recovered_event_metadata(step: i64, source_kind: &str, source_event_id: &str) -> String {
        format!(
            r#"{{"trace_contract_version":1,"phase_id":"recovery-1","cycle_step":{step},"role":"orchestra","status":"reported","record_kind":"recovered_event","content_json":{{"evidence":"stored source row confirms this","source_kind":"{source_kind}","source_event_id":"{source_event_id}"}}}}"#
        )
    }
}
