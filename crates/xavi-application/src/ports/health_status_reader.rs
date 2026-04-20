//! Port for obtaining the current runtime health.

use xavi_domain::health::HealthReport;

/// Adapter contract for reading health state from the outside world.
pub trait HealthStatusReader: Send + Sync {
    /// Returns the latest health snapshot.
    fn read(&self) -> HealthReport;
}
