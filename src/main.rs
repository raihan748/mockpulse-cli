//! mockpulse-cli: Instant zero-config HTTP mock server from OpenAPI/JSON Schema.

use clap::{Parser, Subcommand};
use colored::Colorize;
use mockpulse_cli::chaos::{ChaosInjector, LatencyDistribution, LatencyEngine};
use mockpulse_cli::router::{RadixTrieRouter, RouteEndpoint};
use mockpulse_cli::schema::SpecDocument;
use mockpulse_cli::server::{CorsConfig, MockServer, RequestHandler, ServerMetrics};
use mockpulse_cli::synthesizer::MockSynthesizer;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(
    name = "mockpulse",
    author = "SMK Telkom Developer <dev@smktelkom-mks.sch.id>",
    version,
    about = "Instant zero-config HTTP mock server from OpenAPI/JSON Schema with latency simulation and chaos fault injection",
    long_about = None
)]
pub struct Cli {
    #[arg(short = 's', long = "spec", help = "Path to OpenAPI (v3.x / Swagger 2.0) or JSON Schema file")]
    pub spec: Option<PathBuf>,

    #[arg(short = 'p', long = "port", default_value = "8080", help = "Port to listen on")]
    pub port: u16,

    #[arg(short = 'H', long = "host", default_value = "127.0.0.1", help = "Host address to bind to")]
    pub host: String,

    #[arg(
        short = 'l',
        long = "latency",
        default_value = "0ms",
        help = "Simulated base latency duration (e.g. '250ms', '1s')"
    )]
    pub latency: String,

    #[arg(
        long = "latency-dist",
        default_value = "fixed",
        help = "Latency distribution: 'fixed', 'uniform', or 'gaussian'"
    )]
    pub latency_dist: String,

    #[arg(
        short = 'j',
        long = "jitter",
        default_value = "0ms",
        help = "Latency jitter or standard deviation (e.g. '50ms')"
    )]
    pub jitter: String,

    #[arg(
        short = 'c',
        long = "chaos-rate",
        default_value = "0%",
        help = "Chaos error injection rate percentage (e.g. '10%' or '0.10')"
    )]
    pub chaos_rate: String,

    #[arg(
        long = "chaos-status",
        default_value = "500",
        help = "HTTP status code to return for injected chaos faults (e.g. 500, 503, 504, 429)"
    )]
    pub chaos_status: u16,

    #[arg(long = "no-cors", help = "Disable Cross-Origin Resource Sharing (CORS) headers")]
    pub no_cors: bool,

    #[arg(short = 'q', long = "quiet", help = "Suppress incoming request logs")]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Start HTTP mock server (default)")]
    Serve {
        #[arg(short = 's', long = "spec")]
        spec: Option<PathBuf>,
    },
    #[command(about = "List all detected API routes and HTTP methods")]
    Routes {
        #[arg(short = 's', long = "spec")]
        spec: Option<PathBuf>,
    },
    #[command(about = "Validate specification syntax without running server")]
    Validate {
        #[arg(short = 's', long = "spec")]
        spec: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    let spec_path = cli
        .command
        .as_ref()
        .and_then(|cmd| match cmd {
            Commands::Serve { spec } => spec.clone(),
            Commands::Routes { spec } => spec.clone(),
            Commands::Validate { spec } => spec.clone(),
        })
        .or(cli.spec.clone());

    let spec_file = match spec_path {
        Some(p) => p,
        None => {
            eprintln!(
                "{}",
                "Error: No specification file provided. Use '--spec <PATH>' or pass a file.".red()
            );
            process::exit(1);
        }
    };

    if !spec_file.exists() {
        eprintln!(
            "{} File not found: {}",
            "Error:".red().bold(),
            spec_file.display()
        );
        process::exit(1);
    }

    let spec_doc = match SpecDocument::from_file(&spec_file) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("{} Failed to parse specification: {}", "Error:".red().bold(), e);
            process::exit(1);
        }
    };

    let title = spec_doc
        .spec
        .info
        .as_ref()
        .map(|i| i.title.as_str())
        .unwrap_or("Mockpulse API");
    let version = spec_doc
        .spec
        .info
        .as_ref()
        .map(|i| i.version.as_str())
        .unwrap_or("1.0.0");

    let routes = RouteEndpoint::extract_routes_from_spec(&spec_doc.spec);

    match cli.command {
        Some(Commands::Validate { .. }) => {
            println!("{} Specification valid: {} (v{})", "✔".green().bold(), title.bold(), version);
            println!("  Total endpoints discovered: {}", routes.len());
            process::exit(0);
        }
        Some(Commands::Routes { .. }) => {
            println!("{} Discovered Routes for {}:", "📋".bold(), title.cyan().bold());
            println!("{:<8} {:<35} {:<15} {}", "METHOD", "PATH", "STATUS", "SUMMARY");
            println!("{}", "-".repeat(80).dimmed());
            for r in &routes {
                let m_str = match r.method.as_str() {
                    "GET" => "GET".green(),
                    "POST" => "POST".blue(),
                    "PUT" => "PUT".yellow(),
                    "DELETE" => "DELETE".red(),
                    "PATCH" => "PATCH".magenta(),
                    _ => r.method.as_str().white(),
                };
                let summary = r.summary.as_deref().unwrap_or("-");
                println!(
                    "{:<8} {:<35} {:<15} {}",
                    m_str,
                    r.path_pattern,
                    format!("HTTP {}", r.default_status),
                    summary.dimmed()
                );
            }
            println!("\nTotal: {} routes registered", routes.len());
            process::exit(0);
        }
        _ => {
            // Default: Serve
            start_server(cli, spec_doc, routes);
        }
    }
}

fn start_server(cli: Cli, spec_doc: SpecDocument, routes: Vec<RouteEndpoint>) {
    let base_ms = LatencyEngine::parse_duration(&cli.latency).unwrap_or_else(|e| {
        eprintln!("{} {}", "Warning:".yellow().bold(), e);
        0
    });

    let jitter_ms = LatencyEngine::parse_duration(&cli.jitter).unwrap_or_else(|e| {
        eprintln!("{} {}", "Warning:".yellow().bold(), e);
        0
    });

    let dist = LatencyDistribution::from_str(&cli.latency_dist)
        .unwrap_or(LatencyDistribution::Fixed);

    let latency_engine = LatencyEngine::new(dist, base_ms, jitter_ms);

    let chaos_rate = ChaosInjector::parse_rate(&cli.chaos_rate).unwrap_or_else(|e| {
        eprintln!("{} {}", "Warning:".yellow().bold(), e);
        0.0
    });

    let chaos_injector = ChaosInjector::new(chaos_rate, cli.chaos_status);

    let mut router = RadixTrieRouter::new();
    for route in routes {
        router.insert(route);
    }

    let synthesizer = MockSynthesizer::new();
    let cors = CorsConfig {
        enabled: !cli.no_cors,
        ..Default::default()
    };
    let metrics = Arc::new(ServerMetrics::new());

    let handler = RequestHandler::new(
        Arc::new(router),
        Arc::new(synthesizer),
        Arc::new(spec_doc.resolver),
        Arc::new(chaos_injector),
        Arc::new(latency_engine),
        cors,
        metrics,
        cli.quiet,
    );

    let addr = format!("{}:{}", cli.host, cli.port);

    println!("{}", "═══════════════════════════════════════════════════════════════════".cyan());
    println!("  {} - Zero-Config HTTP Mock Server", "mockpulse-cli".bold().green());
    println!("{}", "═══════════════════════════════════════════════════════════════════".cyan());
    println!("  Spec File    : {}", cli.spec.as_ref().map(|p| p.display().to_string()).unwrap_or_default());
    println!("  Endpoints    : {} routes mapped", handler.router.len());
    println!("  Latency      : {} (dist: {:?}, jitter: {}ms)", format!("{}ms", base_ms).yellow(), dist, jitter_ms);
    println!("  Chaos Rate   : {} (status: {})", format!("{:.1}%", chaos_rate * 100.0).magenta(), cli.chaos_status);
    println!("  CORS Headers : {}", if !cli.no_cors { "Enabled (all origins)".green() } else { "Disabled".yellow() });
    println!("  Metrics URL  : http://{}:{}/_mockpulse/metrics", cli.host, cli.port);
    println!("  Routes URL   : http://{}:{}/_mockpulse/routes", cli.host, cli.port);
    println!("  Health URL   : http://{}:{}/_mockpulse/health", cli.host, cli.port);
    println!("{}", "═══════════════════════════════════════════════════════════════════\n".cyan());

    match MockServer::bind(&addr, handler) {
        Ok(server) => {
            server.run();
        }
        Err(e) => {
            eprintln!("{} Failed to bind server on {}: {}", "Error:".red().bold(), addr, e);
            process::exit(1);
        }
    }
}
