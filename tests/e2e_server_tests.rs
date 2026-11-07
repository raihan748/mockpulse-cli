//! End-to-end integration tests for the mock HTTP server, routing, and live TCP requests.

use mockpulse_cli::chaos::{ChaosInjector, LatencyEngine};
use mockpulse_cli::router::{RadixTrieRouter, RouteEndpoint};
use mockpulse_cli::schema::SpecDocument;
use mockpulse_cli::server::{CorsConfig, MockServer, RequestHandler, ServerMetrics};
use mockpulse_cli::synthesizer::MockSynthesizer;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

fn build_test_handler() -> RequestHandler {
    let yaml = r#"
openapi: "3.0.3"
info:
  title: "E2E Test API"
  version: "1.0.0"
paths:
  /api/v1/users/{userId}:
    get:
      summary: "Get user by ID"
      responses:
        "200":
          description: "User details"
          content:
            application/json:
              schema:
                type: object
                required:
                  - id
                  - email
                properties:
                  id:
                    type: string
                    format: uuid
                  email:
                    type: string
                    format: email
                  active:
                    type: boolean
  /api/v1/items:
    get:
      summary: "List items"
      responses:
        "200":
          description: "List of items"
          content:
            application/json:
              example:
                items:
                  - id: 1
                    name: "Gadget A"
                  - id: 2
                    name: "Gadget B"
"#;

    let doc = SpecDocument::from_str(yaml).expect("Valid spec");
    let routes = RouteEndpoint::extract_routes_from_spec(&doc.spec);

    let mut router = RadixTrieRouter::new();
    for r in routes {
        router.insert(r);
    }

    let synthesizer = MockSynthesizer::new();
    let chaos = ChaosInjector::new(0.0, 500);
    let latency = LatencyEngine::default();
    let cors = CorsConfig::default();
    let metrics = Arc::new(ServerMetrics::new());

    RequestHandler::new(
        Arc::new(router),
        Arc::new(synthesizer),
        Arc::new(doc.resolver),
        Arc::new(chaos),
        Arc::new(latency),
        cors,
        metrics,
        true, // quiet
    )
}

#[test]
fn test_handler_route_and_params() {
    let handler = build_test_handler();

    let resp = handler.handle("GET", "/api/v1/users/usr_4821");
    assert_eq!(resp.status, 200);
    assert_eq!(resp.content_type, "application/json");

    let val: serde_json::Value = serde_json::from_str(&resp.body).expect("Valid JSON");
    assert!(val.get("id").is_some());
    assert!(val.get("email").is_some());
}

#[test]
fn test_handler_example_payload() {
    let handler = build_test_handler();

    let resp = handler.handle("GET", "/api/v1/items");
    assert_eq!(resp.status, 200);
    let val: serde_json::Value = serde_json::from_str(&resp.body).expect("Valid JSON");
    assert_eq!(val["items"][0]["name"], "Gadget A");
}

#[test]
fn test_handler_internal_endpoints() {
    let handler = build_test_handler();

    let health = handler.handle("GET", "/_mockpulse/health");
    assert_eq!(health.status, 200);
    assert!(health.body.contains("\"status\": \"ok\""));

    let routes = handler.handle("GET", "/_mockpulse/routes");
    assert_eq!(routes.status, 200);
    assert!(routes.body.contains("/api/v1/users/{userId}"));

    let metrics = handler.handle("GET", "/_mockpulse/metrics");
    assert_eq!(metrics.status, 200);
    assert!(metrics.body.contains("total_requests"));
}

#[test]
fn test_handler_cors_and_404() {
    let handler = build_test_handler();

    // CORS OPTIONS
    let cors = handler.handle("OPTIONS", "/api/v1/users/42");
    assert_eq!(cors.status, 204);

    // 404 Not Found
    let nf = handler.handle("GET", "/api/v1/nonexistent");
    assert_eq!(nf.status, 404);
    assert!(nf.body.contains("NotFound"));
}

#[test]
fn test_live_tcp_http_server_e2e() {
    let handler = build_test_handler();

    // Bind to port 0 (OS chooses free ephemeral port)
    let server = MockServer::bind("127.0.0.1:0", handler).expect("Failed to bind test server");
    let port = server.addr.port();
    let (handle, running) = server.spawn();

    // Connect via TcpStream
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port))
        .expect("Failed to connect to test server");

    let request = "GET /api/v1/items HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
    stream.write_all(request.as_bytes()).expect("Write request");

    let mut response = String::new();
    stream.read_to_string(&mut response).expect("Read response");

    assert!(response.contains("HTTP/1.1 200 OK"));
    assert!(response.contains("Gadget A"));
    assert!(response.contains("access-control-allow-origin: *") || response.contains("Access-Control-Allow-Origin: *"));

    // Stop server
    running.store(false, std::sync::atomic::Ordering::SeqCst);
    let _ = handle.join();
}
