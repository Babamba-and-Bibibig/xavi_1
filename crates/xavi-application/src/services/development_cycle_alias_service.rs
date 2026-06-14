//! Development cycle alias use cases.

use crate::ports::development_cycle_alias_store::{
    DevelopmentCycleAliasStore, DevelopmentCycleAliasStoreResult,
};
use xavi_domain::development_cycle::{
    DevelopmentCycleAlias, DevelopmentCycleAliasReservation, validate_development_cycle_alias,
    validate_development_cycle_id,
};

/// Application service for reserving and resolving cycle aliases.
pub struct DevelopmentCycleAliasService {
    store: Box<dyn DevelopmentCycleAliasStore>,
}

impl DevelopmentCycleAliasService {
    /// Creates a service from an alias store adapter.
    #[must_use]
    pub fn new(store: impl DevelopmentCycleAliasStore + 'static) -> Self {
        Self { store: Box::new(store) }
    }

    /// Reserves a cycle alias.
    ///
    /// # Errors
    ///
    /// Returns an error when the request is invalid, already reserved, or cannot be persisted.
    pub fn reserve_alias(
        &self,
        reservation: &DevelopmentCycleAliasReservation,
    ) -> DevelopmentCycleAliasStoreResult<DevelopmentCycleAlias> {
        self.store.reserve_cycle_alias(reservation)
    }

    /// Resolves a full alias.
    ///
    /// # Errors
    ///
    /// Returns an error when the alias is invalid or cannot be read.
    pub fn resolve_alias(
        &self,
        cycle_alias: &str,
    ) -> DevelopmentCycleAliasStoreResult<Option<DevelopmentCycleAlias>> {
        validate_development_cycle_alias(cycle_alias)
            .map_err(|error| format!("invalid cycle alias: {error}"))?;
        self.store.get_cycle_alias_by_alias(cycle_alias)
    }

    /// Reads the alias for a canonical cycle id.
    ///
    /// # Errors
    ///
    /// Returns an error when the cycle id is unsafe or cannot be read.
    pub fn alias_for_cycle_id(
        &self,
        cycle_id: &str,
    ) -> DevelopmentCycleAliasStoreResult<Option<DevelopmentCycleAlias>> {
        validate_development_cycle_id(cycle_id)
            .map_err(|error| format!("invalid cycle id: {error}"))?;
        self.store.get_cycle_alias_by_cycle_id(cycle_id)
    }

    /// Lists aliases for report index generation.
    ///
    /// # Errors
    ///
    /// Returns an error when aliases cannot be read.
    pub fn list_aliases(&self) -> DevelopmentCycleAliasStoreResult<Vec<DevelopmentCycleAlias>> {
        self.store.list_cycle_aliases()
    }
}
