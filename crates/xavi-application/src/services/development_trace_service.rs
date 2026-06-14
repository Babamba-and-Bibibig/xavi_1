//! Development trace use cases.

use std::fmt::Write as _;

use crate::ports::development_trace_store::{DevelopmentTraceStore, DevelopmentTraceStoreResult};
use xavi_domain::development_trace::{
    DevelopmentTraceEntry, DevelopmentTraceExportFormat, DevelopmentTraceFilter,
    DevelopmentTraceKind, NewDevelopmentTraceEntry, validate_trace_entry_contract_append,
};

/// Application service for recording and exporting development trace entries.
pub struct DevelopmentTraceService {
    store: Box<dyn DevelopmentTraceStore>,
}

impl DevelopmentTraceService {
    /// Creates a service from a trace store adapter.
    #[must_use]
    pub fn new(store: impl DevelopmentTraceStore + 'static) -> Self {
        Self { store: Box::new(store) }
    }

    /// Appends an immutable trace entry.
    ///
    /// # Errors
    ///
    /// Returns an error when persistence rejects or cannot store the entry.
    pub fn append_entry(
        &self,
        entry: &NewDevelopmentTraceEntry,
    ) -> DevelopmentTraceStoreResult<DevelopmentTraceEntry> {
        validate_entry_before_append(entry)?;
        self.store.append_entry(entry)
    }

    /// Lists trace entries matching the supplied filter.
    ///
    /// # Errors
    ///
    /// Returns an error when persistence cannot read matching entries.
    pub fn list_entries(
        &self,
        filter: &DevelopmentTraceFilter,
    ) -> DevelopmentTraceStoreResult<Vec<DevelopmentTraceEntry>> {
        self.store.list_entries(filter)
    }

    /// Lists the latest trace entries matching the supplied filter, ordered oldest to newest.
    ///
    /// # Errors
    ///
    /// Returns an error when persistence cannot read matching entries.
    pub fn list_latest_entries(
        &self,
        filter: &DevelopmentTraceFilter,
    ) -> DevelopmentTraceStoreResult<Vec<DevelopmentTraceEntry>> {
        self.store.list_latest_entries(filter)
    }

    /// Shows one trace entry by event id.
    ///
    /// # Errors
    ///
    /// Returns an error when persistence cannot read the entry.
    pub fn show_entry(
        &self,
        event_id: &str,
    ) -> DevelopmentTraceStoreResult<Option<DevelopmentTraceEntry>> {
        self.store.get_entry_by_event_id(event_id)
    }

    /// Exports trace entries matching the supplied filter.
    ///
    /// # Errors
    ///
    /// Returns an error when matching entries cannot be read.
    pub fn export_entries(
        &self,
        filter: &DevelopmentTraceFilter,
        format: DevelopmentTraceExportFormat,
    ) -> DevelopmentTraceStoreResult<String> {
        let entries = self.store.list_entries(filter)?;
        let exported = match format {
            DevelopmentTraceExportFormat::Markdown => export_markdown(filter, &entries),
            DevelopmentTraceExportFormat::Jsonl => export_jsonl(&entries),
        };
        Ok(exported)
    }
}

fn validate_entry_before_append(
    entry: &NewDevelopmentTraceEntry,
) -> DevelopmentTraceStoreResult<()> {
    if matches!(entry.kind, DevelopmentTraceKind::AgentDispatch | DevelopmentTraceKind::AgentReturn)
        && entry.role_name.as_deref().map_or("", str::trim).is_empty()
    {
        return Err(format!("{} requires role_name before append", entry.kind.as_str()).into());
    }
    validate_trace_entry_contract_append(
        entry.kind,
        entry.role_name.as_deref(),
        &entry.body,
        &entry.metadata_json,
    )
    .map_err(|error| format!("trace metadata contract validation failed: {error}").into())
}

fn export_markdown(filter: &DevelopmentTraceFilter, entries: &[DevelopmentTraceEntry]) -> String {
    let mut output = "# Development Trace Export\n\n".to_owned();
    if let Some(cycle_id) = &filter.cycle_id {
        let _ = writeln!(output, "- cycle_id: `{cycle_id}`");
    }
    if let Some(kind) = filter.kind {
        let _ = writeln!(output, "- kind: `{}`", kind.as_str());
    }
    let _ = writeln!(output, "- entries: `{}`\n", entries.len());

    for entry in entries {
        let _ = writeln!(output, "## {}. {}\n", entry.id, entry.kind.as_str());
        let _ = writeln!(output, "- event_id: `{}`", entry.event_id);
        let _ = writeln!(output, "- cycle_id: `{}`", entry.cycle_id);
        if let Some(user_turn_id) = &entry.user_turn_id {
            let _ = writeln!(output, "- user_turn_id: `{user_turn_id}`");
        }
        if let Some(role_name) = &entry.role_name {
            let _ = writeln!(output, "- role_name: `{role_name}`");
        }
        let _ = writeln!(output, "- created_at: `{}`\n", entry.created_at);
        output.push_str("### Summary\n\n");
        output.push_str(&entry.summary);
        output.push_str("\n\n### Body\n\n");
        output.push_str(&entry.body);
        output.push_str("\n\n### Metadata JSON\n\n```text\n");
        output.push_str(&entry.metadata_json);
        output.push_str("\n```\n\n");
    }

    output
}

fn export_jsonl(entries: &[DevelopmentTraceEntry]) -> String {
    let mut output = String::new();
    for entry in entries {
        output.push('{');
        let _ = write!(output, "\"id\":{},", entry.id);
        output.push_str("\"event_id\":");
        output.push_str(&json_string(&entry.event_id));
        output.push_str(",\"cycle_id\":");
        output.push_str(&json_string(&entry.cycle_id));
        output.push_str(",\"user_turn_id\":");
        output.push_str(&json_optional_string(entry.user_turn_id.as_deref()));
        output.push_str(",\"kind\":");
        output.push_str(&json_string(entry.kind.as_str()));
        output.push_str(",\"role_name\":");
        output.push_str(&json_optional_string(entry.role_name.as_deref()));
        output.push_str(",\"summary\":");
        output.push_str(&json_string(&entry.summary));
        output.push_str(",\"body\":");
        output.push_str(&json_string(&entry.body));
        output.push_str(",\"metadata_json\":");
        output.push_str(&json_string(&entry.metadata_json));
        output.push_str(",\"created_at\":");
        output.push_str(&json_string(&entry.created_at));
        output.push_str("}\n");
    }
    output
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

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::ports::development_trace_store::DevelopmentTraceStore;

    #[test]
    fn service_rejects_contract_covered_append_without_metadata() {
        let service = DevelopmentTraceService::new(InMemoryTraceStore::default());
        let error = service
            .append_entry(&trace_entry(DevelopmentTraceKind::AgentDispatch, Some("planning"), "{}"))
            .expect_err("missing contract metadata should fail")
            .to_string();

        assert!(error.contains("trace metadata contract validation failed"));
        assert!(error.contains("trace_contract_version"));
    }

    #[test]
    fn service_rejects_metadata_role_mismatch() {
        let service = DevelopmentTraceService::new(InMemoryTraceStore::default());
        let error = service
            .append_entry(&trace_entry(
                DevelopmentTraceKind::AgentDispatch,
                Some("codegen"),
                r#"{"trace_contract_version":1,"phase_id":"planning-1","cycle_step":1,"role":"planning","agent_id":"agent-1","status":"dispatched","expected_next_kind":"agent_return"}"#,
            ))
            .expect_err("role mismatch should fail")
            .to_string();

        assert!(error.contains("does not match --role"));
    }

    #[test]
    fn service_rejects_agent_return_without_role_name() {
        let service = DevelopmentTraceService::new(InMemoryTraceStore::default());
        let error = service
            .append_entry(&trace_entry(
                DevelopmentTraceKind::AgentReturn,
                None,
                r#"{"trace_contract_version":1,"phase_id":"codegen-1","cycle_step":2,"role":"codegen","agent_id":"agent-1","parent_event_id":"dispatch-1","status":"returned","result":"success"}"#,
            ))
            .expect_err("agent return without role_name should fail")
            .to_string();

        assert!(error.contains("requires role_name"));
    }

    #[test]
    fn service_rejects_user_query_without_required_content_sections() {
        let service = DevelopmentTraceService::new(InMemoryTraceStore::default());
        let error = service
            .append_entry(&trace_entry(
                DevelopmentTraceKind::UserQuery,
                Some("user"),
                r#"{"trace_contract_version":1,"phase_id":"user-query","cycle_step":1,"role":"user","status":"requested","content_json":{"user_request":"go"}}"#,
            ))
            .expect_err("incomplete user query content should fail")
            .to_string();

        assert!(error.contains("trace metadata contract validation failed"));
        assert!(error.contains("missing_content_section"));
    }

    #[test]
    fn service_allows_legacy_contractless_non_required_entries() {
        let service = DevelopmentTraceService::new(InMemoryTraceStore::default());
        let stored = service
            .append_entry(&trace_entry(
                DevelopmentTraceKind::ProjectKnowledgeNote,
                Some("orchestra"),
                "{}",
            ))
            .expect("legacy contractless non-required entry should append");

        assert_eq!(stored.kind, DevelopmentTraceKind::ProjectKnowledgeNote);
    }

    #[derive(Default)]
    struct InMemoryTraceStore {
        entries: Mutex<Vec<DevelopmentTraceEntry>>,
    }

    impl DevelopmentTraceStore for InMemoryTraceStore {
        fn append_entry(
            &self,
            entry: &NewDevelopmentTraceEntry,
        ) -> DevelopmentTraceStoreResult<DevelopmentTraceEntry> {
            let mut entries = self.entries.lock().expect("store mutex should lock");
            let stored = DevelopmentTraceEntry {
                id: i64::try_from(entries.len() + 1).expect("test id should fit"),
                event_id: entry.event_id.clone(),
                cycle_id: entry.cycle_id.clone(),
                user_turn_id: entry.user_turn_id.clone(),
                kind: entry.kind,
                role_name: entry.role_name.clone(),
                summary: entry.summary.clone(),
                body: entry.body.clone(),
                metadata_json: entry.metadata_json.clone(),
                created_at: entry.created_at.clone(),
            };
            entries.push(stored.clone());
            Ok(stored)
        }

        fn list_entries(
            &self,
            filter: &DevelopmentTraceFilter,
        ) -> DevelopmentTraceStoreResult<Vec<DevelopmentTraceEntry>> {
            let mut entries =
                matching_entries(&self.entries.lock().expect("store mutex should lock"), filter);
            if let Some(limit) = filter.limit {
                entries.truncate(limit);
            }
            Ok(entries)
        }

        fn list_latest_entries(
            &self,
            filter: &DevelopmentTraceFilter,
        ) -> DevelopmentTraceStoreResult<Vec<DevelopmentTraceEntry>> {
            let mut entries =
                matching_entries(&self.entries.lock().expect("store mutex should lock"), filter);
            if let Some(limit) = filter.limit {
                if entries.len() > limit {
                    entries = entries.split_off(entries.len() - limit);
                }
            }
            Ok(entries)
        }

        fn get_entry_by_event_id(
            &self,
            event_id: &str,
        ) -> DevelopmentTraceStoreResult<Option<DevelopmentTraceEntry>> {
            Ok(self
                .entries
                .lock()
                .expect("store mutex should lock")
                .iter()
                .find(|entry| entry.event_id == event_id)
                .cloned())
        }
    }

    fn matching_entries(
        entries: &[DevelopmentTraceEntry],
        filter: &DevelopmentTraceFilter,
    ) -> Vec<DevelopmentTraceEntry> {
        entries
            .iter()
            .filter(|entry| {
                filter.cycle_id.as_ref().is_none_or(|cycle_id| entry.cycle_id == *cycle_id)
                    && filter.kind.is_none_or(|kind| entry.kind == kind)
            })
            .cloned()
            .collect()
    }

    fn trace_entry(
        kind: DevelopmentTraceKind,
        role_name: Option<&str>,
        metadata_json: &str,
    ) -> NewDevelopmentTraceEntry {
        NewDevelopmentTraceEntry {
            event_id: "event-1".to_owned(),
            cycle_id: "cycle-1".to_owned(),
            user_turn_id: None,
            kind,
            role_name: role_name.map(str::to_owned),
            summary: "summary".to_owned(),
            body: "body".to_owned(),
            metadata_json: metadata_json.to_owned(),
            created_at: "unix:1".to_owned(),
        }
    }
}
