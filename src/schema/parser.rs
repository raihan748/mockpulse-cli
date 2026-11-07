//! OpenAPI and JSON Schema parser supporting both YAML and JSON.

use super::resolver::ReferenceResolver;
use super::types::{Info, OpenApiSpec, Operation, PathItem, ResponseItem, Schema};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub enum SchemaError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Yaml(serde_yaml::Error),
    InvalidFormat(String),
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaError::Io(err) => write!(f, "I/O error reading specification: {}", err),
            SchemaError::Json(err) => write!(f, "JSON parse error: {}", err),
            SchemaError::Yaml(err) => write!(f, "YAML parse error: {}", err),
            SchemaError::InvalidFormat(msg) => write!(f, "Invalid specification format: {}", msg),
        }
    }
}

impl std::error::Error for SchemaError {}

impl From<std::io::Error> for SchemaError {
    fn from(err: std::io::Error) -> Self {
        SchemaError::Io(err)
    }
}

impl From<serde_json::Error> for SchemaError {
    fn from(err: serde_json::Error) -> Self {
        SchemaError::Json(err)
    }
}

impl From<serde_yaml::Error> for SchemaError {
    fn from(err: serde_yaml::Error) -> Self {
        SchemaError::Yaml(err)
    }
}

pub struct SpecDocument {
    pub spec: OpenApiSpec,
    pub resolver: ReferenceResolver,
}

impl SpecDocument {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, SchemaError> {
        let content = fs::read_to_string(path)?;
        Self::from_str(&content)
    }

    pub fn from_str(content: &str) -> Result<Self, SchemaError> {
        let trimmed = content.trim();

        // 1. Try parsing as OpenAPI 3.x / Swagger 2.0 via YAML or JSON
        if let Ok(spec) = serde_yaml::from_str::<OpenApiSpec>(trimmed) {
            if spec.openapi.is_some() || spec.swagger.is_some() || !spec.paths.is_empty() {
                let resolver = ReferenceResolver::new(&spec);
                return Ok(Self { spec, resolver });
            }
        }

        // 2. Try parsing as single raw JSON Schema
        if let Ok(schema) = serde_yaml::from_str::<Schema>(trimmed) {
            if schema.schema_type.is_some()
                || !schema.properties.is_empty()
                || schema.items.is_some()
                || schema.enum_values.is_some()
            {
                let mut paths = BTreeMap::new();
                let mut responses = BTreeMap::new();
                let mut media_types = BTreeMap::new();
                media_types.insert(
                    "application/json".to_string(),
                    super::types::MediaType {
                        schema: Some(schema.clone()),
                        example: schema.example.clone(),
                        examples: BTreeMap::new(),
                    },
                );

                responses.insert(
                    "200".to_string(),
                    ResponseItem {
                        description: Some("Successful response synthesized from schema".to_string()),
                        content: media_types,
                        schema: Some(schema.clone()),
                        ref_path: None,
                    },
                );

                let operation = Operation {
                    summary: Some("Root mock response".to_string()),
                    description: Some("Auto-generated endpoint from raw JSON Schema".to_string()),
                    operation_id: Some("get_root".to_string()),
                    parameters: Vec::new(),
                    request_body: None,
                    responses,
                };

                let path_item = PathItem {
                    summary: Some("Root endpoint".to_string()),
                    description: None,
                    get: Some(operation),
                    post: None,
                    put: None,
                    delete: None,
                    patch: None,
                    options: None,
                    head: None,
                    parameters: Vec::new(),
                };

                paths.insert("/".to_string(), path_item);

                let spec = OpenApiSpec {
                    openapi: Some("3.0.3".to_string()),
                    swagger: None,
                    info: Some(Info {
                        title: "Mockpulse Synthesized API".to_string(),
                        version: "1.0.0".to_string(),
                        description: Some("Generated dynamically from JSON Schema".to_string()),
                    }),
                    paths,
                    components: None,
                    definitions: None,
                };

                let resolver = ReferenceResolver::new(&spec);
                return Ok(Self { spec, resolver });
            }
        }

        Err(SchemaError::InvalidFormat(
            "Content could not be parsed as OpenAPI 3.x, Swagger 2.0, or JSON Schema".to_string(),
        ))
    }
}
