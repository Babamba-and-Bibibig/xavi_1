//! Bootstrap binary for the Xavi workspace.

use std::env;
use std::error::Error;
use std::path::Path;

use xavi_application::services::development_cycle_alias_service::DevelopmentCycleAliasService;
use xavi_application::services::development_trace_service::DevelopmentTraceService;
use xavi_application::services::health_check_service::HealthCheckService;
use xavi_domain::development_cycle::{
    DevelopmentCycleAlias, DevelopmentCycleAliasRequest, DevelopmentCycleAliasReservation,
    render_development_cycle_alias_index_json,
};
use xavi_domain::development_trace::{
    DevelopmentTraceExportFormat, DevelopmentTraceFilter, DevelopmentTraceKind,
    NewDevelopmentTraceEntry, audit_development_trace_cycle, canonical_development_trace_columns,
    render_development_trace_audit_json, render_development_trace_audit_jsonl,
    render_development_trace_audit_text, trace_text_sha256_hex,
    validate_evidence_first_trace_entry_append,
};
use xavi_infrastructure::development_trace::sqlite_development_trace_store::SqliteDevelopmentTraceStore;
use xavi_infrastructure::health::in_memory_status_reader::InMemoryHealthStatusReader;

const DEFAULT_TRACE_DB_PATH: &str = ".xavi/development_trace.sqlite3";
const DEFAULT_REPORTS_ROOT_PATH: &str = ".xavi/reports/development_cycles";
const TRACE_ROLES: [&str; 11] = [
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

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "trace") {
        run_trace_cli(&args[1..])?;
        return Ok(());
    }

    let service = HealthCheckService::new(InMemoryHealthStatusReader::healthy());
    let report = service.execute();

    println!("xavi-bootstrap initialized: status={:?}, message={}", report.status, report.message);
    Ok(())
}

fn run_trace_cli(args: &[String]) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(command) = args.first() else {
        print_trace_help();
        return Ok(());
    };

    let options = CliOptions::parse(&args[1..]);

    match command.as_str() {
        "append" => {
            validate_append_trace_options(&options)?;
            append_trace_entry(&trace_service(&options)?, &options)
        }
        "cycle-start" => {
            append_cycle_ref_entry(&trace_service(&options)?, &options, CycleRefKind::Baseline)
        }
        "cycle-end" => {
            append_cycle_ref_entry(&trace_service(&options)?, &options, CycleRefKind::Head)
        }
        "command-record" => append_command_record_entry(&trace_service(&options)?, &options),
        "list" => list_trace_entries(&trace_service(&options)?, &options),
        "show" => show_trace_entry(&trace_service(&options)?, &options),
        "export" => export_trace_entries(&trace_service(&options)?, &options),
        "audit" => audit_trace_entries(&trace_service(&options)?, &options),
        "reserve-alias" => reserve_cycle_alias(&cycle_alias_service(&options)?, &options),
        "resolve-alias" => resolve_cycle_alias(&cycle_alias_service(&options)?, &options),
        _ => {
            print_trace_help();
            Ok(())
        }
    }
}

fn trace_service(
    options: &CliOptions,
) -> Result<DevelopmentTraceService, Box<dyn Error + Send + Sync>> {
    let db_path = options.value("db").unwrap_or(DEFAULT_TRACE_DB_PATH);
    Ok(DevelopmentTraceService::new(SqliteDevelopmentTraceStore::open(db_path)?))
}

fn cycle_alias_service(
    options: &CliOptions,
) -> Result<DevelopmentCycleAliasService, Box<dyn Error + Send + Sync>> {
    let db_path = options.value("db").unwrap_or(DEFAULT_TRACE_DB_PATH);
    Ok(DevelopmentCycleAliasService::new(SqliteDevelopmentTraceStore::open(db_path)?))
}

fn append_trace_entry(
    service: &DevelopmentTraceService,
    options: &CliOptions,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let kind = parse_required_kind(options)?;
    let created_at = options.value("at").map_or_else(epoch_timestamp, ToOwned::to_owned);
    let event_id =
        options.value("event-id").map_or_else(|| generated_event_id(kind), ToOwned::to_owned);
    let cycle_id = trace_cycle_id(options).unwrap_or("default").to_owned();
    let summary = value_any(options, &["summary", "body"]).unwrap_or("").to_owned();
    if summary.is_empty() {
        return Err("missing --summary or --body".into());
    }
    let body = options.value("body").unwrap_or(&summary).to_owned();
    let contract = append_trace_entry_contract(kind, options, &event_id, &created_at, &body)?;

    let entry = NewDevelopmentTraceEntry {
        event_id,
        cycle_id,
        user_turn_id: options.value("user-turn").map(ToOwned::to_owned),
        kind,
        role_name: contract.role_name,
        body,
        summary,
        metadata_json: contract.metadata_json,
        created_at,
    };

    append_prepared_trace_entry(service, &entry)
}

struct AppendTraceEntryContract {
    role_name: Option<String>,
    metadata_json: String,
}

fn append_trace_entry_contract(
    kind: DevelopmentTraceKind,
    options: &CliOptions,
    event_id: &str,
    created_at: &str,
    body: &str,
) -> Result<AppendTraceEntryContract, Box<dyn Error + Send + Sync>> {
    let role_name = validated_trace_role(kind, options)?;
    if let Some(metadata_json) = value_any(options, &["metadata-json", "metadata"]) {
        return Ok(AppendTraceEntryContract { role_name, metadata_json: metadata_json.to_owned() });
    }
    if !append_boundary_verbatim_requested(options) {
        return Ok(AppendTraceEntryContract { role_name, metadata_json: "{}".to_owned() });
    }

    let role = match role_name.as_deref() {
        Some(role) => role.to_owned(),
        None => normalized_cli_role_or_default(options, default_boundary_role(kind)?)?,
    };
    let metadata_json =
        boundary_event_metadata_json(kind, options, event_id, created_at, body, &role)?;
    Ok(AppendTraceEntryContract { role_name: Some(role), metadata_json })
}

fn append_prepared_trace_entry(
    service: &DevelopmentTraceService,
    entry: &NewDevelopmentTraceEntry,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let canonical =
        canonical_development_trace_columns(entry.kind, &entry.body, &entry.metadata_json)
            .map_err(|error| format!("trace canonical column extraction failed: {error}"))?;
    let stored = service.append_entry(entry)?;
    println!(
        "stored development trace entry: id={}, event_id={}, kind={}, content_hash={}, source_kind={}, source_event_id={}",
        stored.id,
        stored.event_id,
        stored.kind.as_str(),
        canonical.content_hash,
        canonical.source_kind.as_deref().unwrap_or("-"),
        canonical.source_event_id.as_deref().unwrap_or("-")
    );
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CycleRefKind {
    Baseline,
    Head,
}

impl CycleRefKind {
    fn phase_id(self) -> &'static str {
        match self {
            Self::Baseline => "cycle-start",
            Self::Head => "cycle-end",
        }
    }

    fn field_name(self) -> &'static str {
        match self {
            Self::Baseline => "cycle_baseline_ref",
            Self::Head => "cycle_head_ref",
        }
    }

    fn ref_kind(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::Head => "head",
        }
    }

    fn default_summary(self) -> &'static str {
        match self {
            Self::Baseline => "cycle baseline ref recorded",
            Self::Head => "cycle head ref recorded",
        }
    }

    fn ref_option_keys(self) -> &'static [&'static str] {
        match self {
            Self::Baseline => &["baseline-ref", "git-ref", "ref"],
            Self::Head => &["head-ref", "git-ref", "ref"],
        }
    }

    fn diff_ref_option_keys(self) -> &'static [&'static str] {
        match self {
            Self::Baseline => &["baseline-diff-ref", "diff-ref"],
            Self::Head => &["head-diff-ref", "diff-ref"],
        }
    }
}

fn append_cycle_ref_entry(
    service: &DevelopmentTraceService,
    options: &CliOptions,
    ref_kind: CycleRefKind,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let cycle_id = trace_cycle_id(options).ok_or("missing --cycle or --cycle-id")?.to_owned();
    let git_ref = value_any(options, ref_kind.ref_option_keys())
        .ok_or_else(|| format!("missing --{} or --git-ref", ref_kind.ref_option_keys()[0]))?;
    let diff_ref = value_any(options, ref_kind.diff_ref_option_keys())
        .ok_or_else(|| format!("missing --{} or --diff-ref", ref_kind.diff_ref_option_keys()[0]))?;
    let created_at = options.value("at").map_or_else(epoch_timestamp, ToOwned::to_owned);
    let event_id = options.value("event-id").map_or_else(
        || generated_event_id(DevelopmentTraceKind::OrchestraJudgment),
        ToOwned::to_owned,
    );
    let order = trace_order(options, &event_id)?;
    let role = normalized_cli_role_or_default(options, "orchestra")?;
    let agent_id = options.value("agent-id").map(str::trim).filter(|value| !value.is_empty());
    let source_ref = options
        .value("source-ref")
        .map_or_else(|| format!("development_trace://events/{event_id}"), ToOwned::to_owned);
    let metadata_json = cycle_ref_metadata_json(CycleRefMetadataInput {
        ref_kind,
        git_ref,
        diff_ref,
        source_ref: &source_ref,
        role: &role,
        agent_id,
        timestamp: &created_at,
        order,
    });
    let summary = options.value("summary").unwrap_or(ref_kind.default_summary()).to_owned();
    let body = options.value("body").unwrap_or(&summary).to_owned();
    let entry = NewDevelopmentTraceEntry {
        event_id,
        cycle_id,
        user_turn_id: options.value("user-turn").map(ToOwned::to_owned),
        kind: DevelopmentTraceKind::OrchestraJudgment,
        role_name: Some(role),
        summary,
        body,
        metadata_json,
        created_at,
    };

    append_prepared_trace_entry(service, &entry)
}

fn append_command_record_entry(
    service: &DevelopmentTraceService,
    options: &CliOptions,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let cycle_id = trace_cycle_id(options).ok_or("missing --cycle or --cycle-id")?.to_owned();
    let command = options.value("command").ok_or("missing --command")?;
    let result = options.value("result").ok_or("missing --result")?;
    let output = options.value("output").ok_or("missing --output")?;
    let result_text = options.value("result-text").unwrap_or(result);
    let created_at = options.value("at").map_or_else(epoch_timestamp, ToOwned::to_owned);
    let event_id = options
        .value("event-id")
        .map_or_else(|| generated_event_id(DevelopmentTraceKind::TestSummary), ToOwned::to_owned);
    let order = required_trace_order(options, "trace command-record")?;
    let role = normalized_cli_role_or_default(options, "test")?;
    let agent_id = options.value("agent-id").map(str::trim).filter(|value| !value.is_empty());
    let source_ref = options
        .value("source-ref")
        .map_or_else(|| format!("development_trace://events/{event_id}"), ToOwned::to_owned);
    let metadata_json = command_record_metadata_json(CommandRecordMetadataInput {
        command,
        result,
        result_text,
        output,
        exit_code: options.value("exit-code"),
        source_ref: &source_ref,
        role: &role,
        agent_id,
        timestamp: &created_at,
        order,
    })?;
    let summary = options
        .value("summary")
        .map_or_else(|| format!("command record: {command}"), ToOwned::to_owned);
    let body = options.value("body").unwrap_or(output).to_owned();
    let entry = NewDevelopmentTraceEntry {
        event_id,
        cycle_id,
        user_turn_id: options.value("user-turn").map(ToOwned::to_owned),
        kind: DevelopmentTraceKind::TestSummary,
        role_name: Some(role),
        summary,
        body,
        metadata_json,
        created_at,
    };

    append_prepared_trace_entry(service, &entry)
}

fn reserve_cycle_alias(
    service: &DevelopmentCycleAliasService,
    options: &CliOptions,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let cycle_id =
        trace_cycle_id(options).ok_or("missing --cycle or --cycle-id for alias reservation")?;
    let request = cycle_alias_request(options)?;
    let created_at = options.value("at").map_or_else(epoch_timestamp, ToOwned::to_owned);
    let reservation = DevelopmentCycleAliasReservation::new(
        cycle_id,
        request,
        options.value("title").map(ToOwned::to_owned),
        created_at,
    )
    .map_err(|error| format!("invalid cycle alias reservation: {error}"))?;
    let alias = service.reserve_alias(&reservation)?;
    write_cycle_alias_index(service, options)?;
    println!("{}", render_cycle_alias_json(&alias));
    Ok(())
}

fn resolve_cycle_alias(
    service: &DevelopmentCycleAliasService,
    options: &CliOptions,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let alias =
        options.value("alias").or_else(|| options.positional(0)).ok_or("missing --alias")?;
    let Some(resolved) = service.resolve_alias(alias)? else {
        return Err(format!("cycle alias not found: {alias}").into());
    };
    println!("{}", render_cycle_alias_json(&resolved));
    Ok(())
}

fn cycle_alias_request(
    options: &CliOptions,
) -> Result<DevelopmentCycleAliasRequest, Box<dyn Error + Send + Sync>> {
    match (options.value("alias"), options.value("category")) {
        (Some(alias), None) => Ok(DevelopmentCycleAliasRequest::FullAlias(alias.to_owned())),
        (None, Some(category)) => Ok(DevelopmentCycleAliasRequest::Category(category.to_owned())),
        (Some(_), Some(_)) => Err("use either --alias or --category, not both".into()),
        (None, None) => Err("missing --alias or --category".into()),
    }
}

fn write_cycle_alias_index(
    service: &DevelopmentCycleAliasService,
    options: &CliOptions,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let reports_dir = options.value("reports-dir").unwrap_or(DEFAULT_REPORTS_ROOT_PATH);
    std::fs::create_dir_all(reports_dir)?;
    let aliases = service.list_aliases()?;
    std::fs::write(
        Path::new(reports_dir).join("aliases.json"),
        render_development_cycle_alias_index_json(&aliases),
    )?;
    Ok(())
}

fn validate_append_trace_options(options: &CliOptions) -> Result<(), Box<dyn Error + Send + Sync>> {
    let kind = parse_required_kind(options)?;
    let summary = value_any(options, &["summary", "body"]).unwrap_or("");
    if summary.is_empty() {
        return Err("missing --summary or --body".into());
    }
    let event_id =
        options.value("event-id").map_or_else(|| generated_event_id(kind), ToOwned::to_owned);
    let created_at = options.value("at").map_or_else(epoch_timestamp, ToOwned::to_owned);
    let body = options.value("body").unwrap_or(summary);
    let contract = append_trace_entry_contract(kind, options, &event_id, &created_at, body)?;
    validate_evidence_first_trace_entry_append(
        kind,
        contract.role_name.as_deref(),
        body,
        &contract.metadata_json,
    )
    .map_err(|error| format!("trace metadata contract validation failed: {error}"))?;
    Ok(())
}

fn list_trace_entries(
    service: &DevelopmentTraceService,
    options: &CliOptions,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let filter = trace_filter(options)?;
    let entries = service.list_entries(&filter)?;
    for entry in entries {
        println!(
            "{} | {} | {} | {} | {} | {} | {}",
            entry.id,
            entry.created_at,
            entry.cycle_id,
            entry.kind.as_str(),
            entry.role_name.as_deref().unwrap_or("-"),
            entry.event_id,
            entry.summary.lines().next().unwrap_or("")
        );
    }
    Ok(())
}

fn show_trace_entry(
    service: &DevelopmentTraceService,
    options: &CliOptions,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let event_id = options
        .value("event-id")
        .or_else(|| options.positional(0))
        .ok_or("missing --event-id")?
        .to_owned();
    let Some(entry) = service.show_entry(&event_id)? else {
        println!("development trace entry not found: {event_id}");
        return Ok(());
    };

    println!("id: {}", entry.id);
    println!("event_id: {}", entry.event_id);
    println!("cycle_id: {}", entry.cycle_id);
    println!("user_turn_id: {}", entry.user_turn_id.as_deref().unwrap_or("-"));
    println!("kind: {}", entry.kind.as_str());
    println!("role_name: {}", entry.role_name.as_deref().unwrap_or("-"));
    println!("created_at: {}", entry.created_at);
    println!("\n[summary]\n{}", entry.summary);
    println!("\n[body]\n{}", entry.body);
    println!("\n[metadata_json]\n{}", entry.metadata_json);
    Ok(())
}

fn export_trace_entries(
    service: &DevelopmentTraceService,
    options: &CliOptions,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let filter = trace_filter(options)?;
    let format_text = options.value("format").unwrap_or("markdown");
    let format = DevelopmentTraceExportFormat::parse(format_text)
        .ok_or_else(|| format!("unknown trace export format: {format_text}"))?;
    print!("{}", service.export_entries(&filter, format)?);
    Ok(())
}

fn audit_trace_entries(
    service: &DevelopmentTraceService,
    options: &CliOptions,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let cycle_id =
        trace_cycle_id(options).ok_or("missing --cycle or --cycle-id for trace audit")?;
    let entries = service.list_entries(&DevelopmentTraceFilter {
        cycle_id: Some(cycle_id.to_owned()),
        kind: None,
        limit: None,
    })?;
    let report = audit_development_trace_cycle(cycle_id, &entries);
    match options.value("format").unwrap_or("text") {
        "text" => print!("{}", render_development_trace_audit_text(&report)),
        "json" => println!("{}", render_development_trace_audit_json(&report)),
        "jsonl" => print!("{}", render_development_trace_audit_jsonl(&report)),
        format => return Err(format!("unknown trace audit format: {format}").into()),
    }
    if report.has_failures() {
        return Err(format!(
            "trace audit failed for cycle {cycle_id}: {} failure(s)",
            report.failure_count
        )
        .into());
    }
    Ok(())
}

fn trace_filter(
    options: &CliOptions,
) -> Result<DevelopmentTraceFilter, Box<dyn Error + Send + Sync>> {
    let kind = match value_any(options, &["kind", "type"]) {
        Some(value) => Some(
            DevelopmentTraceKind::parse(value)
                .ok_or_else(|| format!("unknown development trace kind: {value}"))?,
        ),
        None => None,
    };
    let limit = match options.value("limit") {
        Some(value) => Some(
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid --limit value {value}: {error}"))?,
        ),
        None => None,
    };

    Ok(DevelopmentTraceFilter {
        cycle_id: trace_cycle_id(options).map(ToOwned::to_owned),
        kind,
        limit,
    })
}

fn parse_required_kind(
    options: &CliOptions,
) -> Result<DevelopmentTraceKind, Box<dyn Error + Send + Sync>> {
    let kind_text = value_any(options, &["kind", "type"]).ok_or("missing --kind")?;
    DevelopmentTraceKind::parse(kind_text)
        .ok_or_else(|| format!("unknown development trace kind: {kind_text}").into())
}

fn validated_trace_role(
    kind: DevelopmentTraceKind,
    options: &CliOptions,
) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
    let role = options.value("role").map(str::trim).filter(|value| !value.is_empty());
    match role {
        Some(value) => known_trace_role(value)
            .map(|role| Some(role.to_owned()))
            .ok_or_else(|| unknown_role_error(value).into()),
        None if role_required_for_kind(kind) => Err(format!(
            "--role <role> is required for trace append --kind {}. allowed roles: {}",
            kind.as_str(),
            TRACE_ROLES.join(", ")
        )
        .into()),
        None => Ok(None),
    }
}

fn role_required_for_kind(kind: DevelopmentTraceKind) -> bool {
    matches!(kind, DevelopmentTraceKind::AgentDispatch | DevelopmentTraceKind::AgentReturn)
}

fn known_trace_role(value: &str) -> Option<&'static str> {
    let normalized = value.trim().to_ascii_lowercase().replace('_', "-");
    TRACE_ROLES.iter().copied().find(|role| *role == normalized)
}

fn unknown_role_error(value: &str) -> String {
    format!("unknown --role value {value:?}. allowed roles: {}", TRACE_ROLES.join(", "))
}

fn value_any<'a>(options: &'a CliOptions, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| options.value(key))
}

fn trace_cycle_id(options: &CliOptions) -> Option<&str> {
    value_any(options, &["cycle", "cycle-id"])
}

fn epoch_timestamp() -> String {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => format!("unix:{}", duration.as_secs()),
        Err(_) => "unix:0".to_owned(),
    }
}

fn generated_event_id(kind: DevelopmentTraceKind) -> String {
    let nanos = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(_) => 0,
    };
    format!("{}-{nanos}", kind.as_str())
}

fn trace_order(options: &CliOptions, event_id: &str) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let Some(value) = value_any(options, &["cycle-step", "order"]) else {
        return Ok(trace_sequence_no(event_id));
    };
    parse_trace_order_value(value)
}

fn required_trace_order(
    options: &CliOptions,
    command_name: &str,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let Some(value) = value_any(options, &["cycle-step", "order"]) else {
        return Err(format!(
            "{command_name} requires --cycle-step <n> or --order <n> for contiguous trace audit ordering"
        )
        .into());
    };
    parse_trace_order_value(value)
}

fn parse_trace_order_value(value: &str) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let parsed =
        value.parse::<i64>().map_err(|error| format!("invalid order value {value}: {error}"))?;
    if parsed <= 0 {
        return Err(format!("order must be positive: {value}").into());
    }
    Ok(parsed)
}

fn trace_sequence_no(event_id: &str) -> i64 {
    event_id
        .rsplit('-')
        .next()
        .and_then(|value| value.parse::<u128>().ok())
        .and_then(|value| i64::try_from((value % (i64::MAX as u128 - 1)) + 1).ok())
        .unwrap_or(1)
}

fn normalized_cli_role_or_default(
    options: &CliOptions,
    default: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let value = options.value("role").unwrap_or(default);
    known_trace_role(value).map(str::to_owned).ok_or_else(|| unknown_role_error(value).into())
}

#[derive(Clone, Copy)]
struct CycleRefMetadataInput<'a> {
    ref_kind: CycleRefKind,
    git_ref: &'a str,
    diff_ref: &'a str,
    source_ref: &'a str,
    role: &'a str,
    agent_id: Option<&'a str>,
    timestamp: &'a str,
    order: i64,
}

fn cycle_ref_metadata_json(input: CycleRefMetadataInput<'_>) -> String {
    let reference = cycle_ref_json(input);
    let mut content = String::new();
    content.push('{');
    push_json_field(
        &mut content,
        "observed_facts",
        &format!("[{}]", json_string(input.ref_kind.default_summary())),
        true,
    );
    push_json_field(&mut content, "decision", &json_string("approved"), false);
    push_json_field(
        &mut content,
        "reasoning_summary",
        &json_string("cycle git/diff reference stored in trace DB"),
        false,
    );
    push_json_field(
        &mut content,
        "next_action",
        &json_string(match input.ref_kind {
            CycleRefKind::Baseline => "begin role work from stored baseline",
            CycleRefKind::Head => "close cycle from stored head",
        }),
        false,
    );
    push_json_field(&mut content, input.ref_kind.field_name(), &reference, false);
    content.push('}');

    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "trace_contract_version", "2", true);
    push_json_field(&mut output, "phase_id", &json_string(input.ref_kind.phase_id()), false);
    push_json_field(&mut output, "cycle_step", &input.order.to_string(), false);
    push_json_field(&mut output, "role", &json_string(input.role), false);
    push_json_field(&mut output, "agent_id", &json_nullable_string(input.agent_id), false);
    push_json_field(
        &mut output,
        "status",
        &json_string(match input.ref_kind {
            CycleRefKind::Baseline => "approved",
            CycleRefKind::Head => "reported",
        }),
        false,
    );
    push_json_field(&mut output, "source_kind", &json_string("cycle_ref"), false);
    push_json_field(&mut output, "source_event_id", &json_string(input.source_ref), false);
    push_json_field(&mut output, "content_json", &content, false);
    output.push('}');
    output
}

fn cycle_ref_json(input: CycleRefMetadataInput<'_>) -> String {
    let ref_kind = input.ref_kind.ref_kind();
    let text = format!("{ref_kind}_ref={}\ndiff_ref={}", input.git_ref, input.diff_ref);
    let hash = trace_text_sha256_hex(&text);
    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "text", &json_string(&text), true);
    push_json_field(&mut output, "ref_kind", &json_string(ref_kind), false);
    push_json_field(&mut output, "git_ref", &json_string(input.git_ref), false);
    push_json_field(&mut output, "diff_ref", &json_string(input.diff_ref), false);
    push_json_field(&mut output, "source_ref", &json_string(input.source_ref), false);
    push_json_field(&mut output, "role", &json_string(input.role), false);
    push_json_field(&mut output, "agent_id", &json_nullable_string(input.agent_id), false);
    push_json_field(&mut output, "hash_sha256", &json_string(&hash), false);
    push_json_field(&mut output, "timestamp", &json_string(input.timestamp), false);
    push_json_field(&mut output, "order", &input.order.to_string(), false);
    output.push('}');
    output
}

#[derive(Clone, Copy)]
struct CommandRecordMetadataInput<'a> {
    command: &'a str,
    result: &'a str,
    result_text: &'a str,
    output: &'a str,
    exit_code: Option<&'a str>,
    source_ref: &'a str,
    role: &'a str,
    agent_id: Option<&'a str>,
    timestamp: &'a str,
    order: i64,
}

fn command_record_metadata_json(
    input: CommandRecordMetadataInput<'_>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let status = command_status(input.result);
    let exit_code_json = match input.exit_code {
        Some(value) => value
            .parse::<i64>()
            .map(|value| value.to_string())
            .map_err(|error| format!("invalid --exit-code value {value}: {error}"))?,
        None => "null".to_owned(),
    };
    let command_verbatim = verbatim_evidence_json(
        input.command,
        "command_invocation",
        &format!("{}#command", input.source_ref),
        input.role,
        input.agent_id,
        input.timestamp,
        input.order,
    );
    let result_verbatim = verbatim_evidence_json(
        input.result_text,
        "command_result",
        &format!("{}#result", input.source_ref),
        input.role,
        input.agent_id,
        input.timestamp,
        input.order.saturating_add(1),
    );
    let output_verbatim = verbatim_evidence_json(
        input.output,
        "command_output",
        &format!("{}#output", input.source_ref),
        input.role,
        input.agent_id,
        input.timestamp,
        input.order.saturating_add(2),
    );
    let mut command_record = String::new();
    command_record.push('{');
    push_json_field(&mut command_record, "command", &json_string(input.command), true);
    push_json_field(&mut command_record, "actor", &json_string(input.role), false);
    push_json_field(&mut command_record, "result", &json_string(input.result), false);
    push_json_field(&mut command_record, "exit_code", &exit_code_json, false);
    push_json_field(&mut command_record, "evidence_ref", &json_string(input.source_ref), false);
    push_json_field(&mut command_record, "command_verbatim", &command_verbatim, false);
    push_json_field(&mut command_record, "result_verbatim", &result_verbatim, false);
    push_json_field(&mut command_record, "output_verbatim", &output_verbatim, false);
    command_record.push('}');

    let mut content = String::new();
    content.push('{');
    push_json_field(&mut content, "commands", &format!("[{}]", json_string(input.command)), true);
    push_json_field(&mut content, "result", &json_string(input.result), false);
    push_json_field(&mut content, "evidence", &json_string(input.source_ref), false);
    push_json_field(&mut content, "command_record", &command_record, false);
    push_json_field(&mut content, "command_verbatim", &command_verbatim, false);
    push_json_field(&mut content, "result_verbatim", &result_verbatim, false);
    push_json_field(&mut content, "output_verbatim", &output_verbatim, false);
    content.push('}');

    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "trace_contract_version", "2", true);
    push_json_field(&mut output, "phase_id", &json_string("command-record"), false);
    push_json_field(&mut output, "cycle_step", &input.order.to_string(), false);
    push_json_field(&mut output, "role", &json_string(input.role), false);
    push_json_field(&mut output, "agent_id", &json_nullable_string(input.agent_id), false);
    push_json_field(&mut output, "commands", &format!("[{}]", json_string(input.command)), false);
    push_json_field(&mut output, "status", &json_string(status), false);
    push_json_field(&mut output, "result", &json_string(input.result), false);
    push_json_field(&mut output, "source_kind", &json_string("command_record"), false);
    push_json_field(&mut output, "source_event_id", &json_string(input.source_ref), false);
    push_json_field(&mut output, "content_json", &content, false);
    output.push('}');
    Ok(output)
}

fn command_status(result: &str) -> &'static str {
    match result.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "passed" | "success" | "no_findings" => "passed",
        "timeout" => "timeout",
        "blocked" | "not_run" | "interrupted" | "cancelled" => "blocked",
        _ => "failed",
    }
}

fn append_boundary_verbatim_requested(options: &CliOptions) -> bool {
    value_any(
        options,
        &[
            "verbatim-text",
            "user-request-verbatim",
            "prompt-verbatim",
            "response-verbatim",
            "result-verbatim",
        ],
    )
    .is_some()
}

fn default_boundary_role(
    kind: DevelopmentTraceKind,
) -> Result<&'static str, Box<dyn Error + Send + Sync>> {
    match kind {
        DevelopmentTraceKind::UserQuery => Ok("user"),
        DevelopmentTraceKind::AgentDispatch | DevelopmentTraceKind::AgentReturn => {
            Err(format!("--role is required for generated v2 {}", kind.as_str()).into())
        }
        _ => Err(format!(
            "--verbatim-text metadata generation is only supported for user_query, agent_dispatch, and agent_return; got {}",
            kind.as_str()
        )
        .into()),
    }
}

fn boundary_event_metadata_json(
    kind: DevelopmentTraceKind,
    options: &CliOptions,
    event_id: &str,
    timestamp: &str,
    body: &str,
    role: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    match kind {
        DevelopmentTraceKind::UserQuery => {
            user_query_boundary_metadata_json(options, event_id, timestamp, body, role)
        }
        DevelopmentTraceKind::AgentDispatch => {
            agent_dispatch_boundary_metadata_json(options, event_id, timestamp, body, role)
        }
        DevelopmentTraceKind::AgentReturn => {
            agent_return_boundary_metadata_json(options, event_id, timestamp, body, role)
        }
        _ => Err(format!(
            "--verbatim-text metadata generation is only supported for user_query, agent_dispatch, and agent_return; got {}",
            kind.as_str()
        )
        .into()),
    }
}

fn user_query_boundary_metadata_json(
    options: &CliOptions,
    event_id: &str,
    timestamp: &str,
    body: &str,
    role: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let order = trace_order(options, event_id)?;
    let verbatim = boundary_verbatim_evidence_json(BoundaryVerbatimInput {
        kind: DevelopmentTraceKind::UserQuery,
        options,
        event_id,
        timestamp,
        body,
        role,
        agent_id: None,
        order,
    })?;
    let field_name = boundary_verbatim_field_name(DevelopmentTraceKind::UserQuery, options)?;
    let phase_id = options.value("phase-id").unwrap_or("user-query");
    let status = options.value("status").unwrap_or("requested");
    let source_type = boundary_source_type(DevelopmentTraceKind::UserQuery, options)?;
    let source_ref = boundary_source_ref(options, event_id);

    let mut content = String::new();
    content.push('{');
    push_json_field(&mut content, field_name, &verbatim, true);
    content.push('}');

    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "trace_contract_version", "2", true);
    push_json_field(&mut output, "phase_id", &json_string(phase_id), false);
    push_json_field(&mut output, "cycle_step", &order.to_string(), false);
    push_json_field(&mut output, "role", &json_string(role), false);
    push_json_field(&mut output, "agent_id", "null", false);
    push_json_field(&mut output, "status", &json_string(status), false);
    push_json_field(&mut output, "source_kind", &json_string(source_type), false);
    push_json_field(&mut output, "source_event_id", &json_string(&source_ref), false);
    push_json_field(&mut output, "content_json", &content, false);
    output.push('}');
    Ok(output)
}

fn agent_dispatch_boundary_metadata_json(
    options: &CliOptions,
    event_id: &str,
    timestamp: &str,
    body: &str,
    role: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let order = trace_order(options, event_id)?;
    let agent_id = required_agent_id(options, DevelopmentTraceKind::AgentDispatch)?;
    let verbatim = boundary_verbatim_evidence_json(BoundaryVerbatimInput {
        kind: DevelopmentTraceKind::AgentDispatch,
        options,
        event_id,
        timestamp,
        body,
        role,
        agent_id: Some(&agent_id),
        order,
    })?;
    let phase_id_default = format!("{role}-dispatch");
    let phase_id = options.value("phase-id").unwrap_or(&phase_id_default);
    let status = options.value("status").unwrap_or("dispatched");
    let source_type = boundary_source_type(DevelopmentTraceKind::AgentDispatch, options)?;
    let source_ref = boundary_source_ref(options, event_id);

    let mut content = String::new();
    content.push('{');
    push_json_field(
        &mut content,
        "injected_context",
        &json_string(options.value("injected-context").unwrap_or("trace append boundary event")),
        true,
    );
    push_json_field(&mut content, "instructions", &json_string(body), false);
    push_json_field(
        &mut content,
        "constraints",
        &json_array_single_string(options.value("constraint").unwrap_or("stay in delegated scope")),
        false,
    );
    push_json_field(
        &mut content,
        "expected_outputs",
        &json_array_single_string(options.value("expected-output").unwrap_or("role return")),
        false,
    );
    push_json_field(
        &mut content,
        "context_report_requirement",
        &json_string(options.value("context-report-requirement").unwrap_or("required")),
        false,
    );
    push_json_field(&mut content, "prompt_verbatim", &verbatim, false);
    content.push('}');

    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "trace_contract_version", "2", true);
    push_json_field(&mut output, "phase_id", &json_string(phase_id), false);
    push_json_field(&mut output, "cycle_step", &order.to_string(), false);
    push_json_field(&mut output, "role", &json_string(role), false);
    push_json_field(&mut output, "agent_id", &json_string(&agent_id), false);
    push_json_field(&mut output, "status", &json_string(status), false);
    push_json_field(
        &mut output,
        "expected_next_kind",
        &json_string(options.value("expected-next-kind").unwrap_or("agent_return")),
        false,
    );
    push_json_field(&mut output, "source_kind", &json_string(source_type), false);
    push_json_field(&mut output, "source_event_id", &json_string(&source_ref), false);
    push_json_field(&mut output, "content_json", &content, false);
    output.push('}');
    Ok(output)
}

fn agent_return_boundary_metadata_json(
    options: &CliOptions,
    event_id: &str,
    timestamp: &str,
    body: &str,
    role: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let order = trace_order(options, event_id)?;
    let agent_id = required_agent_id(options, DevelopmentTraceKind::AgentReturn)?;
    let parent_event_id = options
        .value("parent-event-id")
        .or_else(|| options.value("parent"))
        .ok_or("missing --parent-event-id for generated v2 agent_return metadata")?;
    let result = options.value("result").unwrap_or("success");
    let field_name = boundary_verbatim_field_name(DevelopmentTraceKind::AgentReturn, options)?;
    let verbatim = boundary_verbatim_evidence_json(BoundaryVerbatimInput {
        kind: DevelopmentTraceKind::AgentReturn,
        options,
        event_id,
        timestamp,
        body,
        role,
        agent_id: Some(&agent_id),
        order,
    })?;
    let phase_id_default = format!("{role}-return");
    let phase_id = options.value("phase-id").unwrap_or(&phase_id_default);
    let status = options.value("status").unwrap_or("returned");
    let source_type = boundary_source_type(DevelopmentTraceKind::AgentReturn, options)?;
    let source_ref = boundary_source_ref(options, event_id);

    let mut content = String::new();
    content.push('{');
    push_json_field(
        &mut content,
        "returned_summary",
        &json_string(options.value("summary").unwrap_or("agent returned")),
        true,
    );
    push_json_field(
        &mut content,
        "changed_files_or_scope",
        &json_array_single_string(
            options.value("changed-files-or-scope").unwrap_or("not declared"),
        ),
        false,
    );
    push_json_field(&mut content, "result", &json_string(result), false);
    push_json_field(
        &mut content,
        "context_report",
        &json_string(options.value("context-report").unwrap_or("not declared")),
        false,
    );
    push_json_field(&mut content, field_name, &verbatim, false);
    content.push('}');

    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "trace_contract_version", "2", true);
    push_json_field(&mut output, "phase_id", &json_string(phase_id), false);
    push_json_field(&mut output, "cycle_step", &order.to_string(), false);
    push_json_field(&mut output, "role", &json_string(role), false);
    push_json_field(&mut output, "agent_id", &json_string(&agent_id), false);
    push_json_field(&mut output, "parent_event_id", &json_string(parent_event_id), false);
    push_json_field(&mut output, "status", &json_string(status), false);
    push_json_field(&mut output, "result", &json_string(result), false);
    push_json_field(&mut output, "source_kind", &json_string(source_type), false);
    push_json_field(&mut output, "source_event_id", &json_string(&source_ref), false);
    push_json_field(&mut output, "content_json", &content, false);
    output.push('}');
    Ok(output)
}

#[derive(Clone, Copy)]
struct BoundaryVerbatimInput<'a> {
    kind: DevelopmentTraceKind,
    options: &'a CliOptions,
    event_id: &'a str,
    timestamp: &'a str,
    body: &'a str,
    role: &'a str,
    agent_id: Option<&'a str>,
    order: i64,
}

fn boundary_verbatim_evidence_json(
    input: BoundaryVerbatimInput<'_>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let text = boundary_verbatim_text(input.options).unwrap_or(input.body);
    let source_type = boundary_source_type(input.kind, input.options)?;
    let source_ref = boundary_source_ref(input.options, input.event_id);
    Ok(verbatim_evidence_json(
        text,
        source_type,
        &source_ref,
        input.role,
        input.agent_id,
        input.timestamp,
        input.order,
    ))
}

fn boundary_verbatim_text(options: &CliOptions) -> Option<&str> {
    value_any(
        options,
        &[
            "verbatim-text",
            "user-request-verbatim",
            "prompt-verbatim",
            "response-verbatim",
            "result-verbatim",
        ],
    )
}

fn boundary_verbatim_field_name(
    kind: DevelopmentTraceKind,
    options: &CliOptions,
) -> Result<&'static str, Box<dyn Error + Send + Sync>> {
    if let Some(field_name) = value_any(options, &["verbatim-field", "evidence-field"]) {
        return match (kind, field_name.trim()) {
            (DevelopmentTraceKind::UserQuery, "user_request_verbatim" | "prompt_verbatim") => {
                Ok(if field_name.trim() == "prompt_verbatim" {
                    "prompt_verbatim"
                } else {
                    "user_request_verbatim"
                })
            }
            (DevelopmentTraceKind::AgentDispatch, "prompt_verbatim") => Ok("prompt_verbatim"),
            (DevelopmentTraceKind::AgentReturn, "response_verbatim" | "result_verbatim") => {
                Ok(if field_name.trim() == "result_verbatim" {
                    "result_verbatim"
                } else {
                    "response_verbatim"
                })
            }
            _ => {
                Err(format!("unsupported --verbatim-field {field_name:?} for {}", kind.as_str())
                    .into())
            }
        };
    }
    if options.value("prompt-verbatim").is_some() {
        return match kind {
            DevelopmentTraceKind::UserQuery | DevelopmentTraceKind::AgentDispatch => {
                Ok("prompt_verbatim")
            }
            _ => Err(format!("--prompt-verbatim is not supported for {}", kind.as_str()).into()),
        };
    }
    if options.value("result-verbatim").is_some() {
        return match kind {
            DevelopmentTraceKind::AgentReturn => Ok("result_verbatim"),
            _ => Err(format!("--result-verbatim is not supported for {}", kind.as_str()).into()),
        };
    }
    if options.value("response-verbatim").is_some() {
        return match kind {
            DevelopmentTraceKind::AgentReturn => Ok("response_verbatim"),
            _ => Err(format!("--response-verbatim is not supported for {}", kind.as_str()).into()),
        };
    }
    if options.value("user-request-verbatim").is_some() {
        return match kind {
            DevelopmentTraceKind::UserQuery => Ok("user_request_verbatim"),
            _ => {
                Err(format!("--user-request-verbatim is not supported for {}", kind.as_str())
                    .into())
            }
        };
    }
    match kind {
        DevelopmentTraceKind::UserQuery => Ok("user_request_verbatim"),
        DevelopmentTraceKind::AgentDispatch => Ok("prompt_verbatim"),
        DevelopmentTraceKind::AgentReturn => Ok("response_verbatim"),
        _ => Err(format!(
            "--verbatim-text metadata generation is not supported for {}",
            kind.as_str()
        )
        .into()),
    }
}

fn boundary_source_type(
    kind: DevelopmentTraceKind,
    options: &CliOptions,
) -> Result<&str, Box<dyn Error + Send + Sync>> {
    if let Some(source_type) = options.value("source-type") {
        return Ok(source_type);
    }
    match kind {
        DevelopmentTraceKind::UserQuery => Ok("user_prompt"),
        DevelopmentTraceKind::AgentDispatch => Ok("orchestra_dispatch"),
        DevelopmentTraceKind::AgentReturn => Ok("agent_result"),
        _ => Err(format!("--source-type default is not defined for {}", kind.as_str()).into()),
    }
}

fn boundary_source_ref(options: &CliOptions, event_id: &str) -> String {
    options
        .value("source-ref")
        .map_or_else(|| format!("development_trace://events/{event_id}"), ToOwned::to_owned)
}

fn required_agent_id(
    options: &CliOptions,
    kind: DevelopmentTraceKind,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    options
        .value("agent-id")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            format!("missing --agent-id for generated v2 {} metadata", kind.as_str()).into()
        })
}

fn json_array_single_string(value: &str) -> String {
    format!("[{}]", json_string(value))
}

fn verbatim_evidence_json(
    text: &str,
    source_type: &str,
    source_ref: &str,
    role: &str,
    agent_id: Option<&str>,
    timestamp: &str,
    order: i64,
) -> String {
    let hash = trace_text_sha256_hex(text);
    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "text", &json_string(text), true);
    push_json_field(&mut output, "source_type", &json_string(source_type), false);
    push_json_field(&mut output, "source_ref", &json_string(source_ref), false);
    push_json_field(&mut output, "role", &json_string(role), false);
    push_json_field(&mut output, "agent_id", &json_nullable_string(agent_id), false);
    push_json_field(&mut output, "hash_sha256", &json_string(&hash), false);
    push_json_field(&mut output, "timestamp", &json_string(timestamp), false);
    push_json_field(&mut output, "order", &order.to_string(), false);
    output.push('}');
    output
}

fn json_nullable_string(value: Option<&str>) -> String {
    value.map_or_else(|| "null".to_owned(), json_string)
}

fn render_cycle_alias_json(alias: &DevelopmentCycleAlias) -> String {
    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "cycle_id", &json_string(&alias.cycle_id), true);
    push_json_field(&mut output, "cycle_alias", &json_string(&alias.cycle_alias), false);
    push_json_field(&mut output, "cycle_category", &json_string(&alias.cycle_category), false);
    push_json_field(
        &mut output,
        "cycle_category_key",
        &json_string(&alias.cycle_category_key),
        false,
    );
    push_json_field(&mut output, "cycle_sequence", &alias.cycle_sequence.to_string(), false);
    push_json_field(
        &mut output,
        "cycle_title",
        &json_optional_string(alias.cycle_title.as_deref()),
        false,
    );
    push_json_field(&mut output, "created_at", &json_string(&alias.created_at), false);
    output.push('}');
    output
}

fn push_json_field(output: &mut String, key: &str, value: &str, first: bool) {
    if !first {
        output.push(',');
    }
    let _ = std::fmt::Write::write_fmt(output, format_args!("{}:{value}", json_string(key)));
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
                let _ = std::fmt::Write::write_fmt(
                    &mut escaped,
                    format_args!("\\u{:04x}", u32::from(control)),
                );
            }
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

fn print_trace_help() {
    println!(
        "\
trace commands:
  trace append --kind <kind> --summary <text> [--body <text>] [--cycle <id>|--cycle-id <id>] [--role <role>] [fields]
  trace append --kind <user_query|agent_dispatch|agent_return> --summary <text> --verbatim-text <text> --source-ref <ref> [--agent-id <id>] [--order <n>|--cycle-step <n>]
  trace cycle-start --cycle <id>|--cycle-id <id> --baseline-ref <ref> --baseline-diff-ref <diff-ref> [--event-id <id>] [--cycle-step <n>]
  trace cycle-end --cycle <id>|--cycle-id <id> --head-ref <ref> --head-diff-ref <diff-ref> [--event-id <id>] [--cycle-step <n>]
  trace command-record --cycle <id>|--cycle-id <id> --cycle-step <n>|--order <n> --command <text> --result <passed|failed|timeout> --output <text> [--exit-code <n>]
  trace list [--cycle <id>|--cycle-id <id>] [--kind <kind>] [--limit <n>]
  trace show --event-id <id>
  trace export --format <markdown|jsonl> [--cycle <id>|--cycle-id <id>] [--kind <kind>] [--limit <n>]
  trace audit --cycle <id>|--cycle-id <id> [--format <text|json|jsonl>]
  trace reserve-alias --cycle <canonical-id>|--cycle-id <canonical-id> (--alias <category-NNN>|--category <category>) [--title <text>] [--reports-dir <path>]
  trace resolve-alias --alias <category-NNN>

kinds:
  user_query
  orchestra_judgment
  agent_dispatch
  agent_return
  file_summary
  test_summary
  project_knowledge_note

role:
  --role is required for agent_dispatch and agent_return.
  allowed roles: orchestra, planning, codegen, review, test, analysis, user-docs, ai-docs, cycle-report, dev-console, user

metadata/content contract:
  user_query, orchestra_judgment, agent_dispatch, agent_return, and test_summary require metadata_json with a supported trace_contract_version.
  trace_contract_version=1 user_query keeps legacy content_json.user_request, constraints, and acceptance_criteria compatibility.
  trace_contract_version=2 is evidence-first: user_query requires content_json.user_request_verbatim or prompt_verbatim, agent_dispatch requires prompt_verbatim, and agent_return requires response_verbatim or result_verbatim.
  trace append can generate v2 boundary metadata for user_query, agent_dispatch, and agent_return from --verbatim-text, --source-ref, --role, --agent-id, --order/--cycle-step, and --parent-event-id for agent_return.
  trace command-record stores a v2 test_summary command_record with command_verbatim, result_verbatim, and output_verbatim, and requires an explicit --cycle-step or --order.
  trace cycle-start/cycle-end store v2 cycle_baseline_ref and cycle_head_ref objects for audit.
  each verbatim object must include text, source_type, source_ref, role, agent_id, hash_sha256, timestamp, and order. hash_sha256 is the lowercase sha256 hex digest of text.
  trace append prints the canonical content_hash/source fields that will be stored beside metadata_json.
  metadata.role must match --role when --role is supplied.
  recovered_event requires source_kind, source_event_id, and evidence. Without evidence, record an audit_gap instead of a recovered_event.
  trace audit is read-only and exits non-zero when required trace contract records are missing.
  trace reserve-alias stores the canonical cycle_id unchanged, reserves a human alias fail-closed, and writes reports aliases.json.

common option:
  --db <path> defaults to .xavi/development_trace.sqlite3"
    );
}

struct CliOptions {
    pairs: Vec<(String, String)>,
    positionals: Vec<String>,
}

impl CliOptions {
    fn parse(args: &[String]) -> Self {
        let mut pairs = Vec::new();
        let mut positionals = Vec::new();
        let mut index = 0;
        while index < args.len() {
            let key = &args[index];
            if let Some(stripped) = key.strip_prefix("--") {
                let value = args.get(index + 1).cloned().unwrap_or_default();
                pairs.push((stripped.to_owned(), value));
                index += 2;
            } else {
                positionals.push(key.clone());
                index += 1;
            }
        }
        Self { pairs, positionals }
    }

    fn value(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find_map(|(candidate, value)| (candidate == key).then_some(value.as_str()))
    }

    fn positional(&self, index: usize) -> Option<&str> {
        self.positionals.get(index).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_dispatch_and_return_require_known_role() {
        let dispatch = options(&[
            "--kind",
            "agent_dispatch",
            "--summary",
            "dispatch planning",
            "--metadata-json",
            dispatch_metadata("planning").as_str(),
        ]);
        let dispatch_kind = parse_required_kind(&dispatch).expect("kind should parse");
        let dispatch_error = validated_trace_role(dispatch_kind, &dispatch)
            .expect_err("agent dispatch without role should fail")
            .to_string();
        assert!(dispatch_error.contains("--role <role> is required"));

        let agent_return = options(&[
            "--kind",
            "agent_return",
            "--summary",
            "return codegen",
            "--metadata-json",
            return_metadata("codegen", "dispatch-codegen-1").as_str(),
        ]);
        let return_kind = parse_required_kind(&agent_return).expect("kind should parse");
        let return_error = validated_trace_role(return_kind, &agent_return)
            .expect_err("agent return without role should fail")
            .to_string();
        assert!(return_error.contains("--role <role> is required"));
    }

    #[test]
    fn trace_role_validation_accepts_known_roles_and_normalizes_aliases() {
        let user_query =
            options(&["--kind", "user_query", "--summary", "user said go", "--role", "user"]);
        let user_query_kind = parse_required_kind(&user_query).expect("kind should parse");
        assert_eq!(
            validated_trace_role(user_query_kind, &user_query).unwrap(),
            Some("user".to_owned())
        );

        let agent_return = options(&[
            "--kind",
            "agent_return",
            "--summary",
            "docs returned",
            "--role",
            "user_docs",
            "--metadata-json",
            return_metadata("user-docs", "dispatch-user-docs-1").as_str(),
        ]);
        let return_kind = parse_required_kind(&agent_return).expect("kind should parse");
        assert_eq!(
            validated_trace_role(return_kind, &agent_return).unwrap(),
            Some("user-docs".to_owned())
        );

        let cycle_report_return = options(&[
            "--kind",
            "agent_return",
            "--summary",
            "cycle report returned",
            "--role",
            "cycle_report",
            "--metadata-json",
            return_metadata("cycle-report", "dispatch-cycle-report-1").as_str(),
        ]);
        let cycle_report_kind =
            parse_required_kind(&cycle_report_return).expect("kind should parse");
        assert_eq!(
            validated_trace_role(cycle_report_kind, &cycle_report_return).unwrap(),
            Some("cycle-report".to_owned())
        );
    }

    #[test]
    fn trace_role_validation_rejects_unknown_role_values() {
        let options = options(&[
            "--kind",
            "agent_return",
            "--summary",
            "returned",
            "--role",
            "builder",
            "--metadata-json",
            return_metadata("builder", "dispatch-builder-1").as_str(),
        ]);
        let kind = parse_required_kind(&options).expect("kind should parse");
        let error =
            validated_trace_role(kind, &options).expect_err("unknown role should fail").to_string();

        assert!(error.contains("unknown --role value"));
        assert!(error.contains("codegen"));
        assert!(error.contains("cycle-report"));
        assert!(error.contains("user"));
    }

    #[test]
    fn non_agent_trace_can_omit_role() {
        let options = options(&["--kind", "orchestra_judgment", "--summary", "accepted"]);
        let kind = parse_required_kind(&options).expect("kind should parse");

        assert_eq!(validated_trace_role(kind, &options).unwrap(), None);
    }

    #[test]
    fn append_validation_rejects_agent_dispatch_without_contract_metadata() {
        let options = options(&[
            "--kind",
            "agent_dispatch",
            "--summary",
            "dispatch planning",
            "--role",
            "planning",
        ]);
        let error = validate_append_trace_options(&options)
            .expect_err("missing contract metadata should fail")
            .to_string();

        assert!(error.contains("trace metadata contract validation failed"));
        assert!(error.contains("trace_contract_version"));
    }

    #[test]
    fn append_validation_rejects_metadata_role_mismatch() {
        let options = options(&[
            "--kind",
            "agent_dispatch",
            "--summary",
            "dispatch planning",
            "--role",
            "codegen",
            "--metadata-json",
            dispatch_metadata("planning").as_str(),
        ]);
        let error = validate_append_trace_options(&options)
            .expect_err("metadata role mismatch should fail")
            .to_string();

        assert!(error.contains("does not match --role"));
    }

    #[test]
    fn append_validation_rejects_incomplete_user_query_content_contract() {
        let options = options(&[
            "--kind",
            "user_query",
            "--summary",
            "user said go",
            "--metadata-json",
            r#"{"trace_contract_version":1,"phase_id":"user-query","cycle_step":1,"role":"user","status":"requested","content_json":{"user_request":"go"}}"#,
        ]);
        let error = validate_append_trace_options(&options)
            .expect_err("incomplete user_query content should fail")
            .to_string();

        assert!(error.contains("trace metadata contract validation failed"));
        assert!(error.contains("missing_content_section"));
    }

    #[test]
    fn trace_append_validation_accepts_evidence_first_user_query_with_timestamp_order() {
        let options = options(&[
            "--kind",
            "user_query",
            "--summary",
            "append trace entry",
            "--metadata-json",
            user_query_metadata().as_str(),
        ]);

        validate_append_trace_options(&options)
            .expect("v2 user query with complete verbatim evidence should pass");
    }

    #[test]
    fn trace_append_validation_accepts_evidence_first_user_query_with_prompt_verbatim() {
        let options = options(&[
            "--kind",
            "user_query",
            "--summary",
            "append trace entry",
            "--metadata-json",
            user_query_metadata_with_verbatim_field("prompt_verbatim").as_str(),
        ]);

        validate_append_trace_options(&options)
            .expect("v2 user query with prompt_verbatim evidence should pass");
    }

    #[test]
    fn trace_append_validation_rejects_evidence_first_user_query_without_verbatim() {
        let options = options(&[
            "--kind",
            "user_query",
            "--summary",
            "summary only",
            "--metadata-json",
            r#"{"trace_contract_version":2,"phase_id":"user-query","cycle_step":1,"role":"user","status":"requested","content_json":{"user_request":"summary only","constraints":["legacy"],"acceptance_criteria":["stored"]}}"#,
        ]);

        let error = validate_append_trace_options(&options)
            .expect_err("v2 user_query without verbatim evidence should fail")
            .to_string();

        assert!(error.contains("trace metadata contract validation failed"));
        assert!(error.contains("missing_verbatim_evidence"));
    }

    #[test]
    fn trace_append_validation_rejects_legacy_summary_only_user_query() {
        let options = options(&[
            "--kind",
            "user_query",
            "--summary",
            "summary only",
            "--metadata-json",
            r#"{"trace_contract_version":1,"phase_id":"user-query","cycle_step":1,"role":"user","status":"requested","content_json":{"user_request":"summary only","constraints":["legacy"],"acceptance_criteria":["stored"]}}"#,
        ]);

        let error = validate_append_trace_options(&options)
            .expect_err("new CLI append path should reject v1 prompt-like events")
            .to_string();

        assert!(error.contains("trace metadata contract validation failed"));
        assert!(error.contains("trace_contract_version=2"));
    }

    #[test]
    fn append_validation_accepts_complete_test_summary_contract_without_cli_role() {
        let metadata = test_summary_metadata();
        let options = options(&[
            "--kind",
            "test_summary",
            "--summary",
            "tests passed",
            "--metadata-json",
            metadata.as_str(),
        ]);

        validate_append_trace_options(&options)
            .expect("complete test summary contract should pass");
    }

    #[test]
    fn trace_filter_accepts_cycle_id_alias_for_list_and_export() {
        let options = options(&["--cycle-id", "cycle-alias", "--limit", "7"]);
        let filter = trace_filter(&options).expect("filter should parse");

        assert_eq!(filter.cycle_id.as_deref(), Some("cycle-alias"));
        assert_eq!(filter.limit, Some(7));
    }

    #[test]
    fn append_trace_entry_uses_cycle_id_alias() {
        let service = test_trace_service();
        let metadata = user_query_metadata();
        let options = options(&[
            "--kind",
            "user_query",
            "--summary",
            "user correction",
            "--cycle-id",
            "cycle-alias",
            "--event-id",
            "user-correction-1",
            "--metadata-json",
            metadata.as_str(),
        ]);

        append_trace_entry(&service, &options).expect("append should use cycle-id alias");
        let entries = service
            .list_entries(&DevelopmentTraceFilter {
                cycle_id: Some("cycle-alias".to_owned()),
                kind: None,
                limit: None,
            })
            .expect("entries should list");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].cycle_id, "cycle-alias");
        assert_eq!(entries[0].event_id, "user-correction-1");
    }

    #[test]
    fn trace_append_generates_v2_user_query_metadata_from_verbatim_text() {
        let service = test_trace_service();
        let options = options(&[
            "--kind",
            "user_query",
            "--summary",
            "user boundary",
            "--cycle-id",
            "cycle-generated-v2",
            "--event-id",
            "user-boundary-1",
            "--verbatim-text",
            "original user request",
            "--source-ref",
            "runtime://user/1",
            "--order",
            "1",
            "--at",
            "unix:1",
        ]);

        validate_append_trace_options(&options).expect("generated v2 user_query should validate");
        append_trace_entry(&service, &options).expect("generated v2 user_query should append");
        let stored = service
            .show_entry("user-boundary-1")
            .expect("show should read")
            .expect("entry should exist");

        assert_eq!(stored.role_name.as_deref(), Some("user"));
        assert!(stored.metadata_json.contains("\"trace_contract_version\":2"));
        assert!(stored.metadata_json.contains("\"user_request_verbatim\""));
        assert!(stored.metadata_json.contains("\"source_ref\":\"runtime://user/1\""));
        assert!(stored.metadata_json.contains(&trace_text_sha256_hex("original user request")));
    }

    #[test]
    fn trace_append_generates_v2_dispatch_and_return_metadata_from_verbatim_text() {
        let service = test_trace_service();
        let dispatch_options = options(&[
            "--kind",
            "agent_dispatch",
            "--summary",
            "dispatch codegen",
            "--cycle-id",
            "cycle-generated-v2",
            "--event-id",
            "dispatch-codegen-generated-1",
            "--role",
            "codegen",
            "--agent-id",
            "agent-codegen-generated-1",
            "--verbatim-text",
            "implement the requested change",
            "--source-ref",
            "runtime://dispatch/codegen/1",
            "--order",
            "2",
            "--at",
            "unix:2",
        ]);
        let return_options = options(&[
            "--kind",
            "agent_return",
            "--summary",
            "return codegen",
            "--cycle-id",
            "cycle-generated-v2",
            "--event-id",
            "return-codegen-generated-1",
            "--role",
            "codegen",
            "--agent-id",
            "agent-codegen-generated-1",
            "--parent-event-id",
            "dispatch-codegen-generated-1",
            "--verbatim-text",
            "codegen completed the change",
            "--source-ref",
            "runtime://return/codegen/1",
            "--result",
            "success",
            "--order",
            "3",
            "--at",
            "unix:3",
        ]);

        validate_append_trace_options(&dispatch_options)
            .expect("generated v2 dispatch should validate");
        validate_append_trace_options(&return_options)
            .expect("generated v2 return should validate");
        append_trace_entry(&service, &dispatch_options).expect("dispatch should append");
        append_trace_entry(&service, &return_options).expect("return should append");
        let dispatch = service
            .show_entry("dispatch-codegen-generated-1")
            .expect("show dispatch should read")
            .expect("dispatch should exist");
        let agent_return = service
            .show_entry("return-codegen-generated-1")
            .expect("show return should read")
            .expect("return should exist");

        assert!(dispatch.metadata_json.contains("\"prompt_verbatim\""));
        assert!(dispatch.metadata_json.contains("\"agent_id\":\"agent-codegen-generated-1\""));
        assert!(agent_return.metadata_json.contains("\"response_verbatim\""));
        assert!(
            agent_return
                .metadata_json
                .contains("\"parent_event_id\":\"dispatch-codegen-generated-1\"")
        );
        assert!(
            agent_return
                .metadata_json
                .contains(&trace_text_sha256_hex("codegen completed the change"))
        );
    }

    #[test]
    fn audit_trace_entries_accepts_cycle_id_alias() {
        let service = test_trace_service();
        append_test_entry(
            &service,
            "cycle-alias",
            "user-query-1",
            DevelopmentTraceKind::UserQuery,
            Some("user"),
            user_query_metadata().as_str(),
        );
        append_test_entry(
            &service,
            "cycle-alias",
            "dispatch-codegen-1",
            DevelopmentTraceKind::AgentDispatch,
            Some("codegen"),
            dispatch_metadata("codegen").as_str(),
        );
        append_test_entry(
            &service,
            "cycle-alias",
            "return-codegen-1",
            DevelopmentTraceKind::AgentReturn,
            Some("codegen"),
            return_metadata("codegen", "dispatch-codegen-1").as_str(),
        );
        append_test_entry(
            &service,
            "cycle-alias",
            "cycle-start-1",
            DevelopmentTraceKind::OrchestraJudgment,
            Some("orchestra"),
            cycle_ref_metadata_json(CycleRefMetadataInput {
                ref_kind: CycleRefKind::Baseline,
                git_ref: "base-ref",
                diff_ref: "diff://base",
                source_ref: "source://cycle-alias/baseline",
                role: "orchestra",
                agent_id: None,
                timestamp: "unix:4",
                order: 4,
            })
            .as_str(),
        );
        let test_summary_metadata = test_summary_metadata();
        append_test_entry(
            &service,
            "cycle-alias",
            "test-summary-1",
            DevelopmentTraceKind::TestSummary,
            Some("test"),
            test_summary_metadata.as_str(),
        );
        append_test_entry(
            &service,
            "cycle-alias",
            "cycle-report-1",
            DevelopmentTraceKind::OrchestraJudgment,
            Some("orchestra"),
            r#"{"trace_contract_version":1,"phase_id":"cycle-report","cycle_step":6,"role":"orchestra","status":"reported","cycle_status":"complete","content_json":{"observed_facts":["audit fixtures appended"],"decision":"approved","reasoning_summary":"cycle has required entries","next_action":"return audit result"}}"#,
        );
        append_test_entry(
            &service,
            "cycle-alias",
            "cycle-end-1",
            DevelopmentTraceKind::OrchestraJudgment,
            Some("orchestra"),
            cycle_ref_metadata_json(CycleRefMetadataInput {
                ref_kind: CycleRefKind::Head,
                git_ref: "head-ref",
                diff_ref: "diff://head",
                source_ref: "source://cycle-alias/head",
                role: "orchestra",
                agent_id: None,
                timestamp: "unix:7",
                order: 7,
            })
            .as_str(),
        );
        let options = options(&["--cycle-id", "cycle-alias", "--format", "json"]);

        audit_trace_entries(&service, &options).expect("audit should accept cycle-id alias");
    }

    #[test]
    fn reserve_cycle_alias_uses_exact_full_alias_and_writes_aliases_index() {
        let db_path = temp_path("alias-full", "sqlite3");
        let reports_dir = temp_dir("alias-full-reports");
        let options = options(&[
            "--db",
            db_path.to_str().unwrap(),
            "--reports-dir",
            reports_dir.to_str().unwrap(),
            "--cycle-id",
            "cycle-alias-cli",
            "--alias",
            "Feature_한글-001",
            "--title",
            "별칭 CLI 테스트",
            "--at",
            "unix:7",
        ]);
        let service = cycle_alias_service(&options).expect("alias service should open");

        reserve_cycle_alias(&service, &options).expect("alias should reserve");
        let alias_json =
            std::fs::read_to_string(reports_dir.join("aliases.json")).expect("index should exist");

        assert!(alias_json.contains("\"cycle_id\":\"cycle-alias-cli\""));
        assert!(alias_json.contains("\"cycle_alias\":\"Feature_한글-001\""));
        assert!(alias_json.contains("\"cycle_category_key\":\"feature_한글\""));
        assert!(alias_json.contains("\"cycle_sequence\":1"));
        assert!(alias_json.contains("\"cycle_title\":\"별칭 CLI 테스트\""));
    }

    #[test]
    fn reserve_cycle_alias_allocates_category_sequence_and_rejects_collision() {
        let db_path = temp_path("alias-sequence", "sqlite3");
        let reports_dir = temp_dir("alias-sequence-reports");
        let first_options = options(&[
            "--db",
            db_path.to_str().unwrap(),
            "--reports-dir",
            reports_dir.to_str().unwrap(),
            "--cycle-id",
            "cycle-alias-cli-1",
            "--category",
            "feature",
            "--at",
            "unix:1",
        ]);
        let second_options = options(&[
            "--db",
            db_path.to_str().unwrap(),
            "--reports-dir",
            reports_dir.to_str().unwrap(),
            "--cycle-id",
            "cycle-alias-cli-2",
            "--category",
            "Feature",
            "--at",
            "unix:2",
        ]);
        let collision_options = options(&[
            "--db",
            db_path.to_str().unwrap(),
            "--reports-dir",
            reports_dir.to_str().unwrap(),
            "--cycle-id",
            "cycle-alias-cli-3",
            "--alias",
            "feature-001",
            "--at",
            "unix:3",
        ]);
        let service = cycle_alias_service(&first_options).expect("alias service should open");

        reserve_cycle_alias(&service, &first_options).expect("first alias should reserve");
        reserve_cycle_alias(&service, &second_options).expect("second alias should reserve");
        let error = reserve_cycle_alias(&service, &collision_options)
            .expect_err("full alias collision should fail")
            .to_string();
        let alias_json =
            std::fs::read_to_string(reports_dir.join("aliases.json")).expect("index should exist");

        assert!(alias_json.contains("\"cycle_alias\":\"feature-001\""));
        assert!(alias_json.contains("\"cycle_alias\":\"Feature-002\""));
        assert!(error.contains("already reserved"));
        assert!(!alias_json.contains("cycle-alias-cli-3"));
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
                    let _ = std::fmt::Write::write_fmt(
                        &mut escaped,
                        format_args!("\\u{:04x}", u32::from(control)),
                    );
                }
                other => escaped.push(other),
            }
        }
        escaped.push('"');
        escaped
    }

    fn options(args: &[&str]) -> CliOptions {
        CliOptions::parse(&args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>())
    }

    fn dispatch_metadata(role: &str) -> String {
        let text = "execute role task";
        let hash = xavi_domain::development_trace::trace_text_sha256_hex(text);
        format!(
            r#"{{"trace_contract_version":2,"phase_id":"{role}-1","cycle_step":2,"role":"{role}","agent_id":"agent-{role}-1","status":"dispatched","expected_next_kind":"agent_return","content_json":{{"injected_context":"cycle context","instructions":{},"constraints":["stay in scope"],"expected_outputs":["summary"],"context_report_requirement":"required","prompt_verbatim":{{"text":{},"source_type":"orchestra_dispatch","source_ref":"source://dispatch/{role}/2","role":"{role}","agent_id":"agent-{role}-1","hash_sha256":"{hash}","timestamp":"unix:2","order":2}}}}}}"#,
            json_string(text),
            json_string(text)
        )
    }

    fn return_metadata(role: &str, parent_event_id: &str) -> String {
        let text = "role returned";
        let hash = xavi_domain::development_trace::trace_text_sha256_hex(text);
        format!(
            r#"{{"trace_contract_version":2,"phase_id":"{role}-1","cycle_step":3,"role":"{role}","agent_id":"agent-{role}-1","parent_event_id":"{parent_event_id}","status":"returned","result":"success","content_json":{{"returned_summary":{},"changed_files_or_scope":["trace"],"result":"success","context_report":"low","response_verbatim":{{"text":{},"source_type":"agent_result","source_ref":"source://return/{role}/3","role":"{role}","agent_id":"agent-{role}-1","hash_sha256":"{hash}","timestamp":"unix:3","order":3}}}}}}"#,
            json_string(text),
            json_string(text)
        )
    }

    fn test_summary_metadata() -> String {
        command_record_metadata_json(CommandRecordMetadataInput {
            command: "cargo test -p xavi-bootstrap",
            result: "passed",
            result_text: "exit status: 0",
            output: "test output passed",
            exit_code: Some("0"),
            source_ref: "source://test/command-record/5",
            role: "test",
            agent_id: None,
            timestamp: "unix:5",
            order: 5,
        })
        .expect("command record test metadata should build")
    }

    fn user_query_metadata() -> String {
        user_query_metadata_with_verbatim_field("user_request_verbatim")
    }

    fn user_query_metadata_with_verbatim_field(field_name: &str) -> String {
        let text = "append trace entry";
        let hash = xavi_domain::development_trace::trace_text_sha256_hex(text);
        format!(
            r#"{{"trace_contract_version":2,"phase_id":"user-query","cycle_step":1,"role":"user","status":"requested","content_json":{{"{field_name}":{{"text":{},"source_type":"user_prompt","source_ref":"source://user-query/1","role":"user","agent_id":null,"hash_sha256":"{hash}","timestamp":"unix:1","order":1}}}}}}"#,
            json_string(text)
        )
    }

    fn test_trace_service() -> DevelopmentTraceService {
        DevelopmentTraceService::new(
            SqliteDevelopmentTraceStore::open_in_memory()
                .expect("in-memory trace store should open"),
        )
    }

    fn append_test_entry(
        service: &DevelopmentTraceService,
        cycle_id: &str,
        event_id: &str,
        kind: DevelopmentTraceKind,
        role_name: Option<&str>,
        metadata_json: &str,
    ) {
        service
            .append_entry(&NewDevelopmentTraceEntry {
                event_id: event_id.to_owned(),
                cycle_id: cycle_id.to_owned(),
                user_turn_id: None,
                kind,
                role_name: role_name.map(str::to_owned),
                summary: event_id.to_owned(),
                body: event_id.to_owned(),
                metadata_json: metadata_json.to_owned(),
                created_at: "unix:1".to_owned(),
            })
            .expect("test trace entry should append");
    }

    fn temp_path(name: &str, extension: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "xavi-bootstrap-{name}-{}-{}.{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
            extension
        ))
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "xavi-bootstrap-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }
}
