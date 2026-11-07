//! Unit tests for chaos engineering fault injector and latency distributions.

use mockpulse_cli::chaos::{ChaosDecision, ChaosInjector, LatencyEngine};

#[test]
fn test_latency_duration_parsing() {
    assert_eq!(LatencyEngine::parse_duration("250ms").unwrap(), 250);
    assert_eq!(LatencyEngine::parse_duration("1.5s").unwrap(), 1500);
    assert_eq!(LatencyEngine::parse_duration("5000us").unwrap(), 5);
    assert_eq!(LatencyEngine::parse_duration("100").unwrap(), 100);
    assert_eq!(LatencyEngine::parse_duration("").unwrap(), 0);
    assert!(LatencyEngine::parse_duration("invalid").is_err());
}

#[test]
fn test_fixed_and_uniform_latency_sampling() {
    let fixed = LatencyEngine::fixed(120);
    for _ in 0..10 {
        assert_eq!(fixed.sample_ms(), 120);
    }

    let uniform = LatencyEngine::uniform(200, 50);
    for _ in 0..50 {
        let s = uniform.sample_ms();
        assert!(s >= 150 && s <= 250);
    }
}

#[test]
fn test_gaussian_latency_sampling() {
    let gaussian = LatencyEngine::gaussian(300, 40);
    let mut sum: u64 = 0;
    let n = 200;
    for _ in 0..n {
        let s = gaussian.sample_ms();
        sum += s;
    }
    let mean = sum as f64 / n as f64;
    // Mean should reasonably be within 300 ± 20ms
    assert!((mean - 300.0).abs() < 25.0);
}

#[test]
fn test_chaos_rate_parsing() {
    assert_eq!(ChaosInjector::parse_rate("10%").unwrap(), 0.10);
    assert_eq!(ChaosInjector::parse_rate("25.5%").unwrap(), 0.255);
    assert_eq!(ChaosInjector::parse_rate("0.15").unwrap(), 0.15);
    assert_eq!(ChaosInjector::parse_rate("50").unwrap(), 0.50);
    assert_eq!(ChaosInjector::parse_rate("").unwrap(), 0.0);
    assert!(ChaosInjector::parse_rate("abc").is_err());
}

#[test]
fn test_chaos_rate_boundary_conditions() {
    let zero_injector = ChaosInjector::new(0.0, 500);
    for _ in 0..50 {
        match zero_injector.evaluate() {
            ChaosDecision::Pass => {}
            ChaosDecision::Fault { .. } => panic!("0% chaos should never trigger fault"),
        }
    }

    let full_injector = ChaosInjector::new(1.0, 503);
    for _ in 0..20 {
        match full_injector.evaluate() {
            ChaosDecision::Fault { status, body, .. } => {
                assert_eq!(status, 503);
                assert!(body.contains("ChaosFaultInjected"));
            }
            ChaosDecision::Pass => panic!("100% chaos must always trigger fault"),
        }
    }
}

#[test]
fn test_chaos_statistical_distribution() {
    // 30% error rate over 1,000 trials
    let injector = ChaosInjector::new(0.30, 500);
    let mut faults = 0;
    let trials = 1000;
    for _ in 0..trials {
        if let ChaosDecision::Fault { .. } = injector.evaluate() {
            faults += 1;
        }
    }

    let rate = faults as f64 / trials as f64;
    // Statistical 3-sigma tolerance for n=1000, p=0.30 (stddev ~= 0.0145)
    assert!((rate - 0.30).abs() < 0.06);
}
