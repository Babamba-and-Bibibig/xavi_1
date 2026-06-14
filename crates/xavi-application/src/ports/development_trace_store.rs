//! Port for append-only development trace persistence.

use xavi_domain::development_trace::{
    DevelopmentTraceEntry, DevelopmentTraceFilter, NewDevelopmentTraceEntry,
};

/// Error type returned by development trace persistence adapters.
pub type DevelopmentTraceStoreError = Box<dyn std::error::Error + Send + Sync>;

/// Result type returned by development trace persistence adapters.
pub type DevelopmentTraceStoreResult<T> = Result<T, DevelopmentTraceStoreError>;

/// Adapter contract for the natural-language development trace ledger.
pub trait DevelopmentTraceStore: Send + Sync {
    /// Appends a new immutable trace entry.
    ///
    /// # Errors
    ///
    /// Returns an error when the backing store cannot persist the entry.
    fn append_entry(
        &self,
        entry: &NewDevelopmentTraceEntry,
    ) -> DevelopmentTraceStoreResult<DevelopmentTraceEntry>;

    /// Lists trace entries matching a filter.
    ///
    /// # Errors
    ///
    /// Returns an error when the backing store cannot read matching entries.
    fn list_entries(
        &self,
        filter: &DevelopmentTraceFilter,
    ) -> DevelopmentTraceStoreResult<Vec<DevelopmentTraceEntry>>;

    /// Lists the latest matching trace entries, returned in ascending row id order.
    ///
    /// # Errors
    ///
    /// Returns an error when the backing store cannot read matching entries.
    fn list_latest_entries(
        &self,
        filter: &DevelopmentTraceFilter,
    ) -> DevelopmentTraceStoreResult<Vec<DevelopmentTraceEntry>>;

    /// Returns one trace entry by caller-supplied event id.
    ///
    /// # Errors
    ///
    /// Returns an error when the backing store cannot read the entry.
    fn get_entry_by_event_id(
        &self,
        event_id: &str,
    ) -> DevelopmentTraceStoreResult<Option<DevelopmentTraceEntry>>;
}
