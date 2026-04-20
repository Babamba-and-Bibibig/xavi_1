//! Strong test harness for integration and scenario-based verification.

pub mod assertions;
pub mod builder;
mod doubles;
pub mod fixtures;
mod harness;
pub mod scenarios;

pub use builder::HarnessBuilder;
pub use harness::{HarnessProfile, TestHarness};
