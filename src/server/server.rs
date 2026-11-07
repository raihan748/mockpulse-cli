//! HTTP server listener wrapping tiny_http with multi-threading and graceful handling.

use super::handler::{HttpResponse, RequestHandler};
use super::middleware::RequestLogger;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tiny_http::{Header, Response, Server, StatusCode};

pub struct MockServer {
    pub addr: SocketAddr,
    server: Arc<Server>,
    handler: Arc<RequestHandler>,
    running: Arc<AtomicBool>,
}

impl MockServer {
    pub fn bind(addr: &str, handler: RequestHandler) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let server = Server::http(addr)?;
        let socket_addr = server.server_addr().to_ip().ok_or_else(|| "Failed to get server IP".to_string())?;

        Ok(Self {
            addr: socket_addr,
            server: Arc::new(server),
            handler: Arc::new(handler),
            running: Arc::new(AtomicBool::new(true)),
        })
    }

    pub fn shutdown(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.server.unblock();
    }

    /// Spawns the server loop on a background thread.
    pub fn spawn(self) -> (thread::JoinHandle<()>, Arc<AtomicBool>) {
        let running = Arc::clone(&self.running);
        let server = Arc::clone(&self.server);
        let handler = Arc::clone(&self.handler);

        let handle = thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                match server.recv_timeout(Duration::from_millis(200)) {
                    Ok(Some(request)) => {
                        let method = request.method().as_str().to_string();
                        let url = request.url().to_string();

                        let http_resp = handler.handle(&method, &url);

                        RequestLogger::log(
                            &method,
                            &url,
                            http_resp.status,
                            http_resp.latency_ms,
                            http_resp.is_chaos,
                            handler.quiet,
                        );

                        Self::send_response(request, &http_resp, &handler);
                    }
                    Ok(None) => {}
                    Err(_) => {
                        if !running.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                }
            }
        });

        (handle, self.running)
    }

    /// Runs the server synchronously on the current thread (blocking).
    pub fn run(&self) {
        println!("🚀 MockPulse server listening on http://{}", self.addr);
        println!("   Press Ctrl+C to stop.\n");

        while self.running.load(Ordering::Relaxed) {
            match self.server.recv_timeout(Duration::from_millis(200)) {
                Ok(Some(request)) => {
                    let method = request.method().as_str().to_string();
                    let url = request.url().to_string();

                    let http_resp = self.handler.handle(&method, &url);

                    RequestLogger::log(
                        &method,
                        &url,
                        http_resp.status,
                        http_resp.latency_ms,
                        http_resp.is_chaos,
                        self.handler.quiet,
                    );

                    Self::send_response(request, &http_resp, &self.handler);
                }
                Ok(None) => {}
                Err(_) => {
                    if !self.running.load(Ordering::Relaxed) {
                        break;
                    }
                }
            }
        }
    }

    fn send_response(
        request: tiny_http::Request,
        http_resp: &HttpResponse,
        handler: &RequestHandler,
    ) {
        let mut response = Response::from_string(&http_resp.body)
            .with_status_code(StatusCode(http_resp.status));

        // Set Content-Type
        if let Ok(h) = Header::from_bytes(&b"Content-Type"[..], http_resp.content_type.as_bytes()) {
            response.add_header(h);
        }

        // Set X-Powered-By
        if let Ok(h) = Header::from_bytes(&b"X-Powered-By"[..], &b"mockpulse-cli"[..]) {
            response.add_header(h);
        }

        // Set CORS headers if enabled
        if handler.cors.enabled {
            if let Ok(h) = Header::from_bytes(&b"Access-Control-Allow-Origin"[..], handler.cors.allow_origin.as_bytes()) {
                response.add_header(h);
            }
            if let Ok(h) = Header::from_bytes(&b"Access-Control-Allow-Methods"[..], handler.cors.allow_methods.as_bytes()) {
                response.add_header(h);
            }
            if let Ok(h) = Header::from_bytes(&b"Access-Control-Allow-Headers"[..], handler.cors.allow_headers.as_bytes()) {
                response.add_header(h);
            }
        }

        let _ = request.respond(response);
    }
}
