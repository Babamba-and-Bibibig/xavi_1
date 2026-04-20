//! Integration tests for the harness-driven health scenario.

use xavi_domain::health::HealthReport;
use xavi_harness::assertions::health::{assert_degraded, assert_healthy, assert_unhealthy};
use xavi_harness::fixtures::health::HealthFixture;
use xavi_harness::{HarnessBuilder, HarnessProfile, TestHarness};

#[test]
fn default_harness_uses_harness_doubles_for_health_checks() {
    let harness = TestHarness::new();

    let report = harness.health().check();

    assert_eq!(harness.profile(), HarnessProfile::HarnessDoubles);
    assert_healthy(&report);
}

#[test]
fn builder_can_switch_named_fixtures_without_repeating_composition_code() {
    let harness = HarnessBuilder::new()
        .with_health_fixture(HealthFixture::degraded_database_reconnecting())
        .build();

    let report = harness.health().check();

    assert_degraded(&report, "db reconnecting");
}

#[test]
fn harness_can_exercise_infrastructure_adapter_profile_when_needed() {
    let harness = HarnessBuilder::new()
        .using_infrastructure_adapters()
        .with_health_report(HealthReport::unhealthy("adapter offline"))
        .build();

    let report = harness.health().check();

    assert_eq!(harness.profile(), HarnessProfile::InfrastructureAdapters);
    assert_unhealthy(&report, "adapter offline");
}
