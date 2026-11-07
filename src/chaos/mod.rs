//! Chaos fault injector and latency simulation engines.

pub mod injector;
pub mod latency;

pub use injector::{ChaosConfig, ChaosDecision, ChaosInjector};
pub use latency::{LatencyDistribution, LatencyEngine};
