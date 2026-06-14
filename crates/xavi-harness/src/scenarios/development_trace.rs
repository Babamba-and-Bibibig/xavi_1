//! Scenario facade for development trace use cases.

use xavi_application::ports::development_trace_store::DevelopmentTraceStoreResult;
use xavi_application::services::development_trace_service::DevelopmentTraceService;
use xavi_domain::development_trace::{
    DevelopmentTraceEntry, DevelopmentTraceExportFormat, DevelopmentTraceFilter,
    NewDevelopmentTraceEntry,
};

/// Harness scenario for recording and querying development trace data.
pub struct DevelopmentTraceScenario<'a> {
    service: &'a DevelopmentTraceService,
}

impl<'a> DevelopmentTraceScenario<'a> {
    /// Creates a scenario from the application service.
    #[must_use]
    pub fn new(service: &'a DevelopmentTraceService) -> Self {
        Self { service }
    }

    /// Appends an entry to the development trace ledger.
    ///
    /// # Errors
    ///
    /// Returns an error when the backing store cannot persist the entry.
    pub fn append_entry(
        &self,
        entry: &NewDevelopmentTraceEntry,
    ) -> DevelopmentTraceStoreResult<DevelopmentTraceEntry> {
        self.service.append_entry(entry)
    }

    /// Lists entries matching the supplied filter.
    ///
    /// # Errors
    ///
    /// Returns an error when the backing store cannot read matching entries.
    pub fn list_entries(
        &self,
        filter: &DevelopmentTraceFilter,
    ) -> DevelopmentTraceStoreResult<Vec<DevelopmentTraceEntry>> {
        self.service.list_entries(filter)
    }

    /// Shows one entry by event id.
    ///
    /// # Errors
    ///
    /// Returns an error when the backing store cannot read the entry.
    pub fn show_entry(
        &self,
        event_id: &str,
    ) -> DevelopmentTraceStoreResult<Option<DevelopmentTraceEntry>> {
        self.service.show_entry(event_id)
    }

    /// Exports entries matching the supplied filter.
    ///
    /// # Errors
    ///
    /// Returns an error when matching entries cannot be read.
    pub fn export_entries(
        &self,
        filter: &DevelopmentTraceFilter,
        format: DevelopmentTraceExportFormat,
    ) -> DevelopmentTraceStoreResult<String> {
        self.service.export_entries(filter, format)
    }
}
