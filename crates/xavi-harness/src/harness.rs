//! Core harness runtime and composition root.

use crate::builder::HarnessBuilder;
use crate::fixtures::health::HealthFixture;
use crate::scenarios::development_trace::DevelopmentTraceScenario;
use crate::scenarios::health::HealthScenario;
use xavi_application::services::development_trace_service::DevelopmentTraceService;
use xavi_application::services::health_check_service::HealthCheckService;
use xavi_domain::health::HealthReport;
use xavi_infrastructure::development_trace::sqlite_development_trace_store::SqliteDevelopmentTraceStore;

/// Identifies which outer-layer composition style the harness uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarnessProfile {
    /// Uses harness-owned port doubles.
    HarnessDoubles,
    /// Uses infrastructure crate adapters.
    InfrastructureAdapters,
}

/// Reusable outer-layer composition root for scenario-based tests.
pub struct TestHarness {
    profile: HarnessProfile,
    health_check_service: HealthCheckService,
    development_trace_service: DevelopmentTraceService,
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl TestHarness {
    /// Creates a default harness using harness-owned doubles.
    #[must_use]
    pub fn new() -> Self {
        HarnessBuilder::new().build()
    }

    /// Creates a builder for explicit harness configuration.
    #[must_use]
    pub fn builder() -> HarnessBuilder {
        HarnessBuilder::new()
    }

    /// Creates a harness using the given report fixture.
    #[must_use]
    pub fn with_health_report(report: HealthReport) -> Self {
        HarnessBuilder::new().with_health_report(report).build()
    }

    /// Creates a harness using the given named fixture.
    #[must_use]
    pub fn with_health_fixture(fixture: HealthFixture) -> Self {
        HarnessBuilder::new().with_health_fixture(fixture).build()
    }

    /// Returns the composition profile used by the harness.
    #[must_use]
    pub fn profile(&self) -> HarnessProfile {
        self.profile
    }

    /// Returns the health scenario facade.
    #[must_use]
    pub fn health(&self) -> HealthScenario<'_> {
        HealthScenario::new(&self.health_check_service)
    }

    /// Returns the development trace scenario facade.
    #[must_use]
    pub fn development_trace(&self) -> DevelopmentTraceScenario<'_> {
        DevelopmentTraceScenario::new(&self.development_trace_service)
    }

    /// Builds a harness from an already-composed application service.
    #[must_use]
    pub(crate) fn from_service(
        profile: HarnessProfile,
        health_check_service: HealthCheckService,
    ) -> Self {
        let development_trace_store = SqliteDevelopmentTraceStore::open_in_memory()
            .expect("in-memory development trace store should initialize");
        Self {
            profile,
            health_check_service,
            development_trace_service: DevelopmentTraceService::new(development_trace_store),
        }
    }
}
