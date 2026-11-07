//! HTTP route definitions and endpoint metadata.

use crate::schema::types::{OpenApiSpec, Operation, Schema};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Options,
    Head,
}

impl HttpMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(HttpMethod::Get),
            "POST" => Some(HttpMethod::Post),
            "PUT" => Some(HttpMethod::Put),
            "DELETE" => Some(HttpMethod::Delete),
            "PATCH" => Some(HttpMethod::Patch),
            "OPTIONS" => Some(HttpMethod::Options),
            "HEAD" => Some(HttpMethod::Head),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Head => "HEAD",
        }
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEndpoint {
    pub method: HttpMethod,
    pub path_pattern: String,
    pub summary: Option<String>,
    pub operation_id: Option<String>,
    pub default_status: u16,
    pub response_schema: Option<Schema>,
    pub example_response: Option<serde_json::Value>,
    pub content_type: String,
}

impl RouteEndpoint {
    pub fn extract_routes_from_spec(spec: &OpenApiSpec) -> Vec<RouteEndpoint> {
        let mut routes = Vec::new();

        for (path, path_item) in &spec.paths {
            for (method_str, operation) in path_item.operations() {
                if let Some(method) = HttpMethod::from_str(method_str) {
                    let endpoint = Self::from_operation(method, path.clone(), operation);
                    routes.push(endpoint);
                }
            }
        }

        routes
    }

    pub fn from_operation(
        method: HttpMethod,
        path_pattern: String,
        operation: &Operation,
    ) -> Self {
        let mut default_status = 200;
        let mut response_schema = None;
        let mut example_response = None;
        let mut content_type = "application/json".to_string();

        // Find primary response (200, 201, 202, or "default")
        let preferred_codes = ["200", "201", "202", "204", "default"];
        for code in preferred_codes {
            if let Some(resp) = operation.responses.get(code) {
                if let Ok(status) = code.parse::<u16>() {
                    default_status = status;
                } else if code == "default" {
                    default_status = 200;
                }

                // Check content map
                for (ct, media) in &resp.content {
                    content_type = ct.clone();
                    if media.example.is_some() {
                        example_response = media.example.clone();
                    } else if let Some((_, ex_obj)) = media.examples.iter().next() {
                        example_response = ex_obj.value.clone();
                    }
                    if media.schema.is_some() {
                        response_schema = media.schema.clone();
                    }
                    break;
                }

                // Check Swagger 2.0 schema fallback
                if response_schema.is_none() && resp.schema.is_some() {
                    response_schema = resp.schema.clone();
                }

                break;
            }
        }

        // If no preferred code matched, take the first available response
        if response_schema.is_none() && example_response.is_none() {
            if let Some((code, resp)) = operation.responses.iter().next() {
                if let Ok(status) = code.parse::<u16>() {
                    default_status = status;
                }
                for (ct, media) in &resp.content {
                    content_type = ct.clone();
                    example_response = media.example.clone();
                    response_schema = media.schema.clone();
                    break;
                }
                if response_schema.is_none() && resp.schema.is_some() {
                    response_schema = resp.schema.clone();
                }
            }
        }

        RouteEndpoint {
            method,
            path_pattern,
            summary: operation.summary.clone(),
            operation_id: operation.operation_id.clone(),
            default_status,
            response_schema,
            example_response,
            content_type,
        }
    }
}
