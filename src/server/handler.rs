//! Request handler orchestrating route matching, latency engine, chaos injector, and data synthesizer.

use crate::chaos::{ChaosDecision, ChaosInjector, LatencyEngine};
use crate::router::{HttpMethod, RadixTrieRouter};
use crate::schema::ReferenceResolver;
use crate::server::metrics::ServerMetrics;
use crate::server::middleware::CorsConfig;
use crate::synthesizer::MockSynthesizer;
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub content_type: String,
    pub is_chaos: bool,
    pub latency_ms: u64,
}

pub struct RequestHandler {
    pub router: Arc<RadixTrieRouter>,
    pub synthesizer: Arc<MockSynthesizer>,
    pub resolver: Arc<ReferenceResolver>,
    pub chaos: Arc<ChaosInjector>,
    pub latency: Arc<LatencyEngine>,
    pub cors: CorsConfig,
    pub metrics: Arc<ServerMetrics>,
    pub quiet: bool,
}

impl RequestHandler {
    pub fn new(
        router: Arc<RadixTrieRouter>,
        synthesizer: Arc<MockSynthesizer>,
        resolver: Arc<ReferenceResolver>,
        chaos: Arc<ChaosInjector>,
        latency: Arc<LatencyEngine>,
        cors: CorsConfig,
        metrics: Arc<ServerMetrics>,
        quiet: bool,
    ) -> Self {
        Self {
            router,
            synthesizer,
            resolver,
            chaos,
            latency,
            cors,
            metrics,
            quiet,
        }
    }

    /// Handles an incoming HTTP request and returns an `HttpResponse`.
    pub fn handle(&self, method_str: &str, raw_url: &str) -> HttpResponse {
        let start = Instant::now();

        // 1. Strip query string from path
        let path = raw_url.split('?').next().unwrap_or(raw_url);

        // 2. Handle internal admin / introspection endpoints
        if method_str.eq_ignore_ascii_case("GET") {
            if path == "/_mockpulse/health" {
                let snap = self.metrics.snapshot();
                let body = json!({
                    "status": "ok",
                    "uptime_secs": snap.uptime_secs,
                    "engine": "mockpulse-cli",
                    "version": env!("CARGO_PKG_VERSION")
                });
                return HttpResponse {
                    status: 200,
                    body: serde_json::to_string_pretty(&body).unwrap_or_default(),
                    content_type: "application/json".to_string(),
                    is_chaos: false,
                    latency_ms: 0,
                };
            }

            if path == "/_mockpulse/metrics" {
                let snap = self.metrics.snapshot();
                return HttpResponse {
                    status: 200,
                    body: serde_json::to_string_pretty(&snap).unwrap_or_default(),
                    content_type: "application/json".to_string(),
                    is_chaos: false,
                    latency_ms: 0,
                };
            }

            if path == "/_mockpulse/routes" {
                let routes: Vec<_> = self
                    .router
                    .all_routes()
                    .iter()
                    .map(|r| {
                        json!({
                            "method": r.method.as_str(),
                            "path": r.path_pattern,
                            "summary": r.summary,
                            "operation_id": r.operation_id,
                            "default_status": r.default_status,
                            "content_type": r.content_type,
                        })
                    })
                    .collect();

                let body = json!({
                    "total_routes": routes.len(),
                    "routes": routes
                });

                return HttpResponse {
                    status: 200,
                    body: serde_json::to_string_pretty(&body).unwrap_or_default(),
                    content_type: "application/json".to_string(),
                    is_chaos: false,
                    latency_ms: 0,
                };
            }
        }

        // 3. Handle CORS Preflight OPTIONS
        if method_str.eq_ignore_ascii_case("OPTIONS") && self.cors.enabled {
            return HttpResponse {
                status: 204,
                body: String::new(),
                content_type: "text/plain".to_string(),
                is_chaos: false,
                latency_ms: 0,
            };
        }

        // 4. Parse HTTP Method
        let method = match HttpMethod::from_str(method_str) {
            Some(m) => m,
            None => {
                let err_body = json!({
                    "error": "MethodNotAllowed",
                    "message": format!("HTTP method '{}' is not supported", method_str)
                });
                return HttpResponse {
                    status: 405,
                    body: serde_json::to_string_pretty(&err_body).unwrap_or_default(),
                    content_type: "application/json".to_string(),
                    is_chaos: false,
                    latency_ms: 0,
                };
            }
        };

        // 5. Route lookup in Radix Trie
        let route_match = match self.router.lookup(&method, path) {
            Some(m) => m,
            None => {
                let registered = self.router.all_routes();
                let hints: Vec<String> = registered
                    .iter()
                    .take(5)
                    .map(|r| format!("{} {}", r.method, r.path_pattern))
                    .collect();

                let not_found_body = json!({
                    "error": "NotFound",
                    "message": format!("No mock route matched for '{} {}'", method_str, path),
                    "registered_routes_sample": hints,
                    "tip": "Visit GET /_mockpulse/routes to view all registered API mock routes"
                });

                let elapsed_ms = start.elapsed().as_millis() as u64;
                self.metrics.record_request(404, elapsed_ms, false);
                return HttpResponse {
                    status: 404,
                    body: serde_json::to_string_pretty(&not_found_body).unwrap_or_default(),
                    content_type: "application/json".to_string(),
                    is_chaos: false,
                    latency_ms: elapsed_ms,
                };
            }
        };

        // 6. Apply Latency Simulation
        self.latency.apply();

        // 7. Evaluate Chaos Fault Injection
        match self.chaos.evaluate() {
            ChaosDecision::Fault {
                status,
                body,
                content_type,
            } => {
                let total_ms = start.elapsed().as_millis() as u64;
                self.metrics.record_request(status, total_ms, true);
                HttpResponse {
                    status,
                    body,
                    content_type,
                    is_chaos: true,
                    latency_ms: total_ms,
                }
            }
            ChaosDecision::Pass => {
                // 8. Generate mock response from schema or example
                let endpoint = route_match.endpoint;
                let status = endpoint.default_status;
                let content_type = endpoint.content_type.clone();

                let body = if let Some(ref ex) = endpoint.example_response {
                    serde_json::to_string_pretty(ex).unwrap_or_default()
                } else if let Some(ref schema) = endpoint.response_schema {
                    let synth_value = self.synthesizer.synthesize(schema, &self.resolver);
                    serde_json::to_string_pretty(&synth_value).unwrap_or_default()
                } else {
                    json!({
                        "message": "Mock response generated by mockpulse-cli",
                        "route": endpoint.path_pattern,
                        "status": status
                    })
                    .to_string()
                };

                let total_ms = start.elapsed().as_millis() as u64;
                self.metrics.record_request(status, total_ms, false);

                HttpResponse {
                    status,
                    body,
                    content_type,
                    is_chaos: false,
                    latency_ms: total_ms,
                }
            }
        }
    }
}
