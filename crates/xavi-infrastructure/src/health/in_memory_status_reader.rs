//! In-memory implementation of the health reader port.

use xavi_application::ports::health_status_reader::HealthStatusReader;
use xavi_domain::health::HealthReport;

/// Simple adapter used for bootstrap and harness defaults.
#[derive(Debug, Clone)]
pub struct InMemoryHealthStatusReader {
    report: HealthReport,
}

impl InMemoryHealthStatusReader {
    /// Creates a healthy default adapter instance.
    #[must_use]
    pub fn healthy() -> Self {
        Self { report: HealthReport::healthy("bootstrap completed") }
    }

    /// Creates an adapter with a custom report.
    #[must_use]
    pub fn with_report(report: HealthReport) -> Self {
        Self { report }
    }
}

impl HealthStatusReader for InMemoryHealthStatusReader {
    fn read(&self) -> HealthReport {
        self.report.clone()
    }
}
