//! Chaos engineering fault injector for simulating random HTTP errors and failures.

use rand::Rng;
use serde_json::json;

#[derive(Debug, Clone)]
pub struct ChaosConfig {
    pub rate: f64, // 0.0 to 1.0
    pub status_code: u16,
    pub custom_message: Option<String>,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self {
            rate: 0.0,
            status_code: 500,
            custom_message: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ChaosDecision {
    Pass,
    Fault {
        status: u16,
        body: String,
        content_type: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct ChaosInjector {
    pub config: ChaosConfig,
}

impl ChaosInjector {
    pub fn new(rate: f64, status_code: u16) -> Self {
        Self {
            config: ChaosConfig {
                rate: rate.clamp(0.0, 1.0),
                status_code,
                custom_message: None,
            },
        }
    }

    /// Parses rate strings like "10%", "0.10", "25", "0.5".
    pub fn parse_rate(s: &str) -> Result<f64, String> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Ok(0.0);
        }

        if let Some(percent_str) = trimmed.strip_suffix('%') {
            let num = percent_str
                .trim()
                .parse::<f64>()
                .map_err(|e| format!("Invalid chaos percentage '{}': {}", s, e))?;
            Ok((num / 100.0).clamp(0.0, 1.0))
        } else {
            let num = trimmed
                .parse::<f64>()
                .map_err(|e| format!("Invalid chaos rate '{}': {}", s, e))?;
            if num > 1.0 && num <= 100.0 {
                // If user supplied integer like 10 instead of 10%
                Ok((num / 100.0).clamp(0.0, 1.0))
            } else {
                Ok(num.clamp(0.0, 1.0))
            }
        }
    }

    pub fn should_inject(&self) -> bool {
        if self.config.rate <= 0.0 {
            return false;
        }
        if self.config.rate >= 1.0 {
            return true;
        }

        let mut rng = rand::thread_rng();
        rng.gen_bool(self.config.rate)
    }

    pub fn evaluate(&self) -> ChaosDecision {
        if !self.should_inject() {
            return ChaosDecision::Pass;
        }

        let status = self.config.status_code;
        let message = self.config.custom_message.clone().unwrap_or_else(|| {
            match status {
                500 => "Internal Server Error (Synthetic Chaos Injection)".to_string(),
                503 => "Service Unavailable (Downstream dependency simulated failure)".to_string(),
                504 => "Gateway Timeout (Simulated upstream timeout)".to_string(),
                429 => "Too Many Requests (Rate limit threshold reached)".to_string(),
                _ => format!("Simulated Fault HTTP {}", status),
            }
        });

        let payload = json!({
            "error": "ChaosFaultInjected",
            "status": status,
            "message": message,
            "chaos_rate": self.config.rate,
            "engine": "mockpulse-cli"
        });

        ChaosDecision::Fault {
            status,
            body: serde_json::to_string_pretty(&payload).unwrap_or_default(),
            content_type: "application/json".to_string(),
        }
    }
}
