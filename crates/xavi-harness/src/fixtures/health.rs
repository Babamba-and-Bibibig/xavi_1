//! Health-related fixtures for harness scenarios.

use xavi_domain::health::HealthReport;

/// Named fixture for health-driven scenarios.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthFixture {
    report: HealthReport,
}

impl HealthFixture {
    /// Creates a healthy baseline fixture that mirrors a booted system.
    #[must_use]
    pub fn healthy_bootstrap() -> Self {
        Self { report: HealthReport::healthy("bootstrap completed") }
    }

    /// Creates a degraded fixture for a reconnecting dependency.
    #[must_use]
    pub fn degraded_database_reconnecting() -> Self {
        Self { report: HealthReport::degraded("db reconnecting") }
    }

    /// Creates an unhealthy fixture for an unavailable runtime.
    #[must_use]
    pub fn unhealthy_runtime_unavailable() -> Self {
        Self { report: HealthReport::unhealthy("runtime unavailable") }
    }

    /// Wraps an existing report as a fixture.
    #[must_use]
    pub fn from_report(report: HealthReport) -> Self {
        Self { report }
    }

    /// Returns the underlying report.
    #[must_use]
    pub fn report(&self) -> &HealthReport {
        &self.report
    }

    /// Consumes the fixture and returns the underlying report.
    #[must_use]
    pub fn into_report(self) -> HealthReport {
        self.report
    }
}
