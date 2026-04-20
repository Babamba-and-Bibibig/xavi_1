//! Health-check use case.

use crate::ports::health_status_reader::HealthStatusReader;
use xavi_domain::health::HealthReport;

/// Primary application use case for querying current system health.
pub struct HealthCheckService {
    reader: Box<dyn HealthStatusReader>,
}

impl HealthCheckService {
    /// Creates a new use case from an outbound port implementation.
    #[must_use]
    pub fn new(reader: impl HealthStatusReader + 'static) -> Self {
        Self { reader: Box::new(reader) }
    }

    /// Executes the use case.
    #[must_use]
    pub fn execute(&self) -> HealthReport {
        self.reader.read()
    }
}
