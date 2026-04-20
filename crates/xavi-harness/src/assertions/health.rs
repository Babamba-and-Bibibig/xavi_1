//! Health-specific assertion helpers.

use xavi_domain::health::{HealthReport, HealthStatus};

/// Asserts that the report is healthy.
///
/// # Panics
///
/// Panics if the report status is not [`HealthStatus::Healthy`].
pub fn assert_healthy(report: &HealthReport) {
    assert_eq!(report.status, HealthStatus::Healthy);
}

/// Asserts that the report is degraded and matches the expected message.
///
/// # Panics
///
/// Panics if the report status is not [`HealthStatus::Degraded`] or the
/// message does not match `expected_message`.
pub fn assert_degraded(report: &HealthReport, expected_message: &str) {
    assert_eq!(report.status, HealthStatus::Degraded);
    assert_eq!(report.message, expected_message);
}

/// Asserts that the report is unhealthy and matches the expected message.
///
/// # Panics
///
/// Panics if the report status is not [`HealthStatus::Unhealthy`] or the
/// message does not match `expected_message`.
pub fn assert_unhealthy(report: &HealthReport, expected_message: &str) {
    assert_eq!(report.status, HealthStatus::Unhealthy);
    assert_eq!(report.message, expected_message);
}
