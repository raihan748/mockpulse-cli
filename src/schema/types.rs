//! OpenAPI and JSON Schema data structures.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level OpenAPI or Swagger specification document.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenApiSpec {
    pub openapi: Option<String>,
    pub swagger: Option<String>,
    pub info: Option<Info>,
    #[serde(default)]
    pub paths: BTreeMap<String, PathItem>,
    pub components: Option<Components>,
    pub definitions: Option<BTreeMap<String, Schema>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Info {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Components {
    #[serde(default)]
    pub schemas: BTreeMap<String, Schema>,
    #[serde(default)]
    pub responses: BTreeMap<String, ResponseItem>,
    #[serde(default)]
    pub parameters: BTreeMap<String, Parameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathItem {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub get: Option<Operation>,
    pub post: Option<Operation>,
    pub put: Option<Operation>,
    pub delete: Option<Operation>,
    pub patch: Option<Operation>,
    pub options: Option<Operation>,
    pub head: Option<Operation>,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
}

impl PathItem {
    pub fn operations(&self) -> Vec<(&'static str, &Operation)> {
        let mut ops = Vec::new();
        if let Some(ref op) = self.get {
            ops.push(("GET", op));
        }
        if let Some(ref op) = self.post {
            ops.push(("POST", op));
        }
        if let Some(ref op) = self.put {
            ops.push(("PUT", op));
        }
        if let Some(ref op) = self.delete {
            ops.push(("DELETE", op));
        }
        if let Some(ref op) = self.patch {
            ops.push(("PATCH", op));
        }
        if let Some(ref op) = self.options {
            ops.push(("OPTIONS", op));
        }
        if let Some(ref op) = self.head {
            ops.push(("HEAD", op));
        }
        ops
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Operation {
    pub summary: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "operationId")]
    pub operation_id: Option<String>,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
    #[serde(rename = "requestBody")]
    pub request_body: Option<RequestBody>,
    #[serde(default)]
    pub responses: BTreeMap<String, ResponseItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub in_location: Option<String>,
    pub description: Option<String>,
    pub required: Option<bool>,
    pub schema: Option<Schema>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestBody {
    pub description: Option<String>,
    pub required: Option<bool>,
    #[serde(default)]
    pub content: BTreeMap<String, MediaType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseItem {
    pub description: Option<String>,
    #[serde(default)]
    pub content: BTreeMap<String, MediaType>,
    pub schema: Option<Schema>, // Swagger 2.0 fallback
    #[serde(rename = "$ref")]
    pub ref_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MediaType {
    pub schema: Option<Schema>,
    pub example: Option<serde_json::Value>,
    #[serde(default)]
    pub examples: BTreeMap<String, ExampleObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExampleObject {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    String,
    Integer,
    Number,
    Boolean,
    Array,
    Object,
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Schema {
    #[serde(rename = "type")]
    pub schema_type: Option<serde_json::Value>, // Can be single string or array of strings
    pub format: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub default: Option<serde_json::Value>,
    pub example: Option<serde_json::Value>,
    pub examples: Option<Vec<serde_json::Value>>,
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<serde_json::Value>>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    #[serde(rename = "minLength")]
    pub min_length: Option<usize>,
    #[serde(rename = "maxLength")]
    pub max_length: Option<usize>,
    #[serde(rename = "minItems")]
    pub min_items: Option<usize>,
    #[serde(rename = "maxItems")]
    pub max_items: Option<usize>,
    pub items: Option<Box<Schema>>,
    #[serde(default)]
    pub properties: BTreeMap<String, Schema>,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(rename = "additionalProperties")]
    pub additional_properties: Option<serde_json::Value>,
    #[serde(rename = "$ref")]
    pub ref_path: Option<String>,
    #[serde(rename = "allOf")]
    pub all_of: Option<Vec<Schema>>,
    #[serde(rename = "oneOf")]
    pub one_of: Option<Vec<Schema>>,
    #[serde(rename = "anyOf")]
    pub any_of: Option<Vec<Schema>>,
}

impl Schema {
    pub fn get_primary_type(&self) -> Option<SchemaType> {
        match self.schema_type.as_ref()? {
            serde_json::Value::String(s) => match s.as_str() {
                "string" => Some(SchemaType::String),
                "integer" => Some(SchemaType::Integer),
                "number" => Some(SchemaType::Number),
                "boolean" => Some(SchemaType::Boolean),
                "array" => Some(SchemaType::Array),
                "object" => Some(SchemaType::Object),
                "null" => Some(SchemaType::Null),
                _ => None,
            },
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        if s != "null" {
                            match s {
                                "string" => return Some(SchemaType::String),
                                "integer" => return Some(SchemaType::Integer),
                                "number" => return Some(SchemaType::Number),
                                "boolean" => return Some(SchemaType::Boolean),
                                "array" => return Some(SchemaType::Array),
                                "object" => return Some(SchemaType::Object),
                                _ => {}
                            }
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }
}
