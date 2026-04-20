//! Scripted port implementation for harness-controlled scenarios.

use xavi_application::ports::health_status_reader::HealthStatusReader;
use xavi_domain::health::HealthReport;

/// Port double that always returns a scripted report.
#[derive(Debug, Clone)]
pub(crate) struct ScriptedHealthStatusReader {
    report: HealthReport,
}

impl ScriptedHealthStatusReader {
    /// Creates a new scripted reader.
    #[must_use]
    pub(crate) fn new(report: HealthReport) -> Self {
        Self { report }
    }
}

impl HealthStatusReader for ScriptedHealthStatusReader {
    fn read(&self) -> HealthReport {
        self.report.clone()
    }
}
