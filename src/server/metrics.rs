//! Thread-safe metrics collector for request counts, status codes, and latency percentiles.

use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

#[derive(Debug, Serialize)]
pub struct LatencyPercentiles {
    pub count: usize,
    pub p50_ms: u64,
    pub p90_ms: u64,
    pub p99_ms: u64,
    pub max_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct MetricsSnapshot {
    pub uptime_secs: u64,
    pub total_requests: u64,
    pub chaos_faults_injected: u64,
    pub status_2xx: u64,
    pub status_4xx: u64,
    pub status_5xx: u64,
    pub latency: LatencyPercentiles,
}

#[derive(Debug)]
pub struct ServerMetrics {
    start_time: Instant,
    total_requests: AtomicU64,
    chaos_injected: AtomicU64,
    status_2xx: AtomicU64,
    status_4xx: AtomicU64,
    status_5xx: AtomicU64,
    latencies_ms: Mutex<Vec<u64>>,
}

impl Default for ServerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerMetrics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_requests: AtomicU64::new(0),
            chaos_injected: AtomicU64::new(0),
            status_2xx: AtomicU64::new(0),
            status_4xx: AtomicU64::new(0),
            status_5xx: AtomicU64::new(0),
            latencies_ms: Mutex::new(Vec::new()),
        }
    }

    pub fn record_request(&self, status: u16, latency_ms: u64, is_chaos: bool) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        if is_chaos {
            self.chaos_injected.fetch_add(1, Ordering::Relaxed);
        }

        match status {
            200..=299 => {
                self.status_2xx.fetch_add(1, Ordering::Relaxed);
            }
            400..=499 => {
                self.status_4xx.fetch_add(1, Ordering::Relaxed);
            }
            500..=599 => {
                self.status_5xx.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }

        if let Ok(mut latencies) = self.latencies_ms.lock() {
            // Cap history to last 5,000 samples to keep memory strictly bounded
            if latencies.len() >= 5000 {
                latencies.remove(0);
            }
            latencies.push(latency_ms);
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        let uptime_secs = self.start_time.elapsed().as_secs();
        let total = self.total_requests.load(Ordering::Relaxed);
        let chaos = self.chaos_injected.load(Ordering::Relaxed);
        let s2xx = self.status_2xx.load(Ordering::Relaxed);
        let s4xx = self.status_4xx.load(Ordering::Relaxed);
        let s5xx = self.status_5xx.load(Ordering::Relaxed);

        let mut samples = {
            if let Ok(lats) = self.latencies_ms.lock() {
                lats.clone()
            } else {
                Vec::new()
            }
        };

        samples.sort_unstable();
        let count = samples.len();

        let (p50_ms, p90_ms, p99_ms, max_ms) = if count == 0 {
            (0, 0, 0, 0)
        } else {
            let p50 = samples[(count as f64 * 0.50) as usize];
            let p90 = samples[((count as f64 * 0.90) as usize).min(count - 1)];
            let p99 = samples[((count as f64 * 0.99) as usize).min(count - 1)];
            let max = *samples.last().unwrap_or(&0);
            (p50, p90, p99, max)
        };

        MetricsSnapshot {
            uptime_secs,
            total_requests: total,
            chaos_faults_injected: chaos,
            status_2xx: s2xx,
            status_4xx: s4xx,
            status_5xx: s5xx,
            latency: LatencyPercentiles {
                count,
                p50_ms,
                p90_ms,
                p99_ms,
                max_ms,
            },
        }
    }
}
