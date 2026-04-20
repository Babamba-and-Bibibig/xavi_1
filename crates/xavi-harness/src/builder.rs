//! Harness builder for selecting composition style and fixtures.

use crate::doubles::scripted_health_status_reader::ScriptedHealthStatusReader;
use crate::fixtures::health::HealthFixture;
use crate::harness::{HarnessProfile, TestHarness};
use xavi_application::services::health_check_service::HealthCheckService;
use xavi_domain::health::HealthReport;
use xavi_infrastructure::health::in_memory_status_reader::InMemoryHealthStatusReader;

#[derive(Debug, Clone)]
enum HealthReaderSource {
    HarnessDouble(HealthReport),
    InfrastructureAdapter(HealthReport),
}

/// Builds a [`TestHarness`] with explicit fixtures and composition profile.
#[derive(Debug, Clone)]
pub struct HarnessBuilder {
    health_reader_source: HealthReaderSource,
}

impl Default for HarnessBuilder {
    fn default() -> Self {
        Self {
            health_reader_source: HealthReaderSource::HarnessDouble(
                HealthFixture::healthy_bootstrap().into_report(),
            ),
        }
    }
}

impl HarnessBuilder {
    /// Creates a new builder with harness-owned doubles and healthy defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Uses a named fixture while keeping the current composition profile.
    #[must_use]
    pub fn with_health_fixture(mut self, fixture: HealthFixture) -> Self {
        self.health_reader_source = match self.health_reader_source {
            HealthReaderSource::HarnessDouble(_) => {
                HealthReaderSource::HarnessDouble(fixture.into_report())
            }
            HealthReaderSource::InfrastructureAdapter(_) => {
                HealthReaderSource::InfrastructureAdapter(fixture.into_report())
            }
        };
        self
    }

    /// Uses an explicit report while keeping the current composition profile.
    #[must_use]
    pub fn with_health_report(self, report: HealthReport) -> Self {
        self.with_health_fixture(HealthFixture::from_report(report))
    }

    /// Switches to harness-owned test doubles.
    #[must_use]
    pub fn using_harness_doubles(mut self) -> Self {
        let report = match &self.health_reader_source {
            HealthReaderSource::HarnessDouble(report)
            | HealthReaderSource::InfrastructureAdapter(report) => report.clone(),
        };
        self.health_reader_source = HealthReaderSource::HarnessDouble(report);
        self
    }

    /// Switches to infrastructure adapters while preserving the current fixture.
    #[must_use]
    pub fn using_infrastructure_adapters(mut self) -> Self {
        let report = match &self.health_reader_source {
            HealthReaderSource::HarnessDouble(report)
            | HealthReaderSource::InfrastructureAdapter(report) => report.clone(),
        };
        self.health_reader_source = HealthReaderSource::InfrastructureAdapter(report);
        self
    }

    /// Builds the final harness composition.
    #[must_use]
    pub fn build(self) -> TestHarness {
        match self.health_reader_source {
            HealthReaderSource::HarnessDouble(report) => TestHarness::from_service(
                HarnessProfile::HarnessDoubles,
                HealthCheckService::new(ScriptedHealthStatusReader::new(report)),
            ),
            HealthReaderSource::InfrastructureAdapter(report) => TestHarness::from_service(
                HarnessProfile::InfrastructureAdapters,
                HealthCheckService::new(InMemoryHealthStatusReader::with_report(report)),
            ),
        }
    }
}
