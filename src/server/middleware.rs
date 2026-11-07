//! Middleware components: CORS headers and colored terminal request logger.

use colored::Colorize;

#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub enabled: bool,
    pub allow_origin: String,
    pub allow_methods: String,
    pub allow_headers: String,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allow_origin: "*".to_string(),
            allow_methods: "GET, POST, PUT, DELETE, PATCH, OPTIONS, HEAD".to_string(),
            allow_headers: "Content-Type, Authorization, Accept, X-Requested-With".to_string(),
        }
    }
}

pub struct RequestLogger;

impl RequestLogger {
    pub fn log(
        method: &str,
        path: &str,
        status: u16,
        latency_ms: u64,
        is_chaos: bool,
        quiet: bool,
    ) {
        if quiet {
            return;
        }

        let status_str = match status {
            200..=299 => status.to_string().green().bold(),
            300..=399 => status.to_string().cyan().bold(),
            400..=499 => status.to_string().yellow().bold(),
            500..=599 => status.to_string().red().bold(),
            _ => status.to_string().white().bold(),
        };

        let method_str = match method.to_uppercase().as_str() {
            "GET" => "GET ".blue().bold(),
            "POST" => "POST".green().bold(),
            "PUT" => "PUT ".yellow().bold(),
            "DELETE" => "DEL ".red().bold(),
            "PATCH" => "PAT ".magenta().bold(),
            "OPTIONS" => "OPT ".cyan(),
            _ => method.white(),
        };

        let latency_str = format!("({}ms)", latency_ms).dimmed();

        if is_chaos {
            let tag = "[CHAOS FAULT]".magenta().bold();
            println!(
                "{} {} {} {} {} {}",
                tag, method_str, path, status_str, latency_str, "⚡ synthetic failure injected".italic().magenta()
            );
        } else {
            let tag = "[MOCK]".dimmed();
            println!("{} {} {} {} {}", tag, method_str, path, status_str, latency_str);
        }
    }
}
