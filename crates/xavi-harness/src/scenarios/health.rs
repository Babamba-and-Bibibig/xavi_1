//! Health-check scenario facade.

use xavi_application::services::health_check_service::HealthCheckService;
use xavi_domain::health::HealthReport;

/// Executes health-related use-case flows through the harness.
pub struct HealthScenario<'a> {
    service: &'a HealthCheckService,
}

impl<'a> HealthScenario<'a> {
    /// Creates a new scenario facade.
    #[must_use]
    pub(crate) fn new(service: &'a HealthCheckService) -> Self {
        Self { service }
    }

    /// Runs the current health-check scenario and returns the report.
    #[must_use]
    pub fn check(&self) -> HealthReport {
        self.service.execute()
    }
}
