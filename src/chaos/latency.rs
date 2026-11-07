//! Latency simulation engine supporting Fixed, Uniform, and Gaussian distributions.

use rand::Rng;
use std::f64::consts::PI;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatencyDistribution {
    Fixed,
    Uniform,
    Gaussian,
}

impl LatencyDistribution {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "fixed" => Some(LatencyDistribution::Fixed),
            "uniform" | "jitter" => Some(LatencyDistribution::Uniform),
            "gaussian" | "normal" => Some(LatencyDistribution::Gaussian),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LatencyEngine {
    pub distribution: LatencyDistribution,
    pub base_ms: u64,
    pub jitter_ms: u64,
}

impl Default for LatencyEngine {
    fn default() -> Self {
        Self {
            distribution: LatencyDistribution::Fixed,
            base_ms: 0,
            jitter_ms: 0,
        }
    }
}

impl LatencyEngine {
    pub fn new(distribution: LatencyDistribution, base_ms: u64, jitter_ms: u64) -> Self {
        Self {
            distribution,
            base_ms,
            jitter_ms,
        }
    }

    pub fn fixed(ms: u64) -> Self {
        Self {
            distribution: LatencyDistribution::Fixed,
            base_ms: ms,
            jitter_ms: 0,
        }
    }

    pub fn uniform(base_ms: u64, jitter_ms: u64) -> Self {
        Self {
            distribution: LatencyDistribution::Uniform,
            base_ms,
            jitter_ms,
        }
    }

    pub fn gaussian(mean_ms: u64, std_dev_ms: u64) -> Self {
        Self {
            distribution: LatencyDistribution::Gaussian,
            base_ms: mean_ms,
            jitter_ms: std_dev_ms,
        }
    }

    /// Parses human-readable durations like "250ms", "1.5s", "500us", "100".
    pub fn parse_duration(s: &str) -> Result<u64, String> {
        let trimmed = s.trim().to_lowercase();
        if trimmed.is_empty() {
            return Ok(0);
        }

        if let Some(num_str) = trimmed.strip_suffix("ms") {
            num_str
                .trim()
                .parse::<f64>()
                .map(|f| f.max(0.0) as u64)
                .map_err(|e| format!("Invalid ms duration '{}': {}", s, e))
        } else if let Some(num_str) = trimmed.strip_suffix('s') {
            num_str
                .trim()
                .parse::<f64>()
                .map(|f| (f.max(0.0) * 1000.0) as u64)
                .map_err(|e| format!("Invalid second duration '{}': {}", s, e))
        } else if let Some(num_str) = trimmed.strip_suffix("us") {
            num_str
                .trim()
                .parse::<f64>()
                .map(|f| (f.max(0.0) / 1000.0) as u64)
                .map_err(|e| format!("Invalid us duration '{}': {}", s, e))
        } else {
            // Assume raw milliseconds if no suffix
            trimmed
                .parse::<f64>()
                .map(|f| f.max(0.0) as u64)
                .map_err(|e| format!("Invalid numeric duration '{}': {}", s, e))
        }
    }

    /// Samples a single simulated duration in milliseconds based on the configured distribution.
    pub fn sample_ms(&self) -> u64 {
        if self.base_ms == 0 && self.jitter_ms == 0 {
            return 0;
        }

        let mut rng = rand::thread_rng();

        match self.distribution {
            LatencyDistribution::Fixed => self.base_ms,
            LatencyDistribution::Uniform => {
                if self.jitter_ms == 0 {
                    self.base_ms
                } else {
                    let min = self.base_ms.saturating_sub(self.jitter_ms);
                    let max = self.base_ms.saturating_add(self.jitter_ms);
                    rng.gen_range(min..=max)
                }
            }
            LatencyDistribution::Gaussian => {
                if self.jitter_ms == 0 {
                    self.base_ms
                } else {
                    // Box-Muller transform: Z = sqrt(-2 ln U1) * cos(2 pi U2)
                    let u1: f64 = rng.gen_range(1e-7..1.0);
                    let u2: f64 = rng.gen_range(0.0..1.0);
                    let z = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();

                    let sampled = (self.base_ms as f64) + z * (self.jitter_ms as f64);
                    sampled.max(0.0).round() as u64
                }
            }
        }
    }

    pub fn sample_duration(&self) -> Duration {
        Duration::from_millis(self.sample_ms())
    }

    /// Pauses current thread execution by the sampled latency duration.
    pub fn apply(&self) -> Duration {
        let duration = self.sample_duration();
        if duration.as_millis() > 0 {
            std::thread::sleep(duration);
        }
        duration
    }
}
