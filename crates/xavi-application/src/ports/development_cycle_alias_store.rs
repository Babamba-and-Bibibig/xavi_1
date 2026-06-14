//! Port for development cycle alias persistence.

use xavi_domain::development_cycle::{DevelopmentCycleAlias, DevelopmentCycleAliasReservation};

/// Error type returned by development cycle alias persistence adapters.
pub type DevelopmentCycleAliasStoreError = Box<dyn std::error::Error + Send + Sync>;

/// Result type returned by development cycle alias persistence adapters.
pub type DevelopmentCycleAliasStoreResult<T> = Result<T, DevelopmentCycleAliasStoreError>;

/// Adapter contract for reserving and resolving human-readable cycle aliases.
pub trait DevelopmentCycleAliasStore: Send + Sync {
    /// Reserves a cycle alias.
    ///
    /// # Errors
    ///
    /// Returns an error when validation fails, a collision is detected, or persistence fails.
    fn reserve_cycle_alias(
        &self,
        reservation: &DevelopmentCycleAliasReservation,
    ) -> DevelopmentCycleAliasStoreResult<DevelopmentCycleAlias>;

    /// Resolves an alias to its stored reservation.
    ///
    /// # Errors
    ///
    /// Returns an error when persistence cannot read the alias table.
    fn get_cycle_alias_by_alias(
        &self,
        cycle_alias: &str,
    ) -> DevelopmentCycleAliasStoreResult<Option<DevelopmentCycleAlias>>;

    /// Reads the alias reserved for a canonical cycle id.
    ///
    /// # Errors
    ///
    /// Returns an error when persistence cannot read the alias table.
    fn get_cycle_alias_by_cycle_id(
        &self,
        cycle_id: &str,
    ) -> DevelopmentCycleAliasStoreResult<Option<DevelopmentCycleAlias>>;

    /// Lists all aliases in stable index order.
    ///
    /// # Errors
    ///
    /// Returns an error when persistence cannot read the alias table.
    fn list_cycle_aliases(&self) -> DevelopmentCycleAliasStoreResult<Vec<DevelopmentCycleAlias>>;
}
