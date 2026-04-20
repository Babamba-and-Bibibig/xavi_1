//! Domain model for runtime health.

/// Represents the overall health state of the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Normal operating state.
    Healthy,
    /// Partial degradation without total outage.
    Degraded,
    /// Unavailable or critically broken state.
    Unhealthy,
}

/// Immutable report returned by health-related use cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthReport {
    /// Current health state.
    pub status: HealthStatus,
    /// Human-readable status message.
    pub message: String,
}

impl HealthReport {
    /// Creates a healthy report.
    #[must_use]
    pub fn healthy(message: impl Into<String>) -> Self {
        Self { status: HealthStatus::Healthy, message: message.into() }
    }

    /// Creates a degraded report.
    #[must_use]
    pub fn degraded(message: impl Into<String>) -> Self {
        Self { status: HealthStatus::Degraded, message: message.into() }
    }

    /// Creates an unhealthy report.
    #[must_use]
    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self { status: HealthStatus::Unhealthy, message: message.into() }
    }
}
