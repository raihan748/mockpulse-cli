//! Unit tests for OpenAPI 3.x, Swagger 2.0, and JSON Schema parsing and reference resolution.

use mockpulse_cli::schema::types::{OpenApiSpec, SchemaType};
use mockpulse_cli::schema::{ReferenceResolver, SpecDocument};

#[test]
fn test_parse_openapi_3_yaml() {
    let yaml = r##"
openapi: "3.0.3"
info:
  title: "Petstore Mock API"
  version: "1.0.0"
paths:
  /pets:
    get:
      summary: "List all pets"
      operationId: "listPets"
      responses:
        "200":
          description: "A paged array of pets"
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: "#/components/schemas/Pet"
components:
  schemas:
    Pet:
      type: object
      required:
        - id
        - name
      properties:
        id:
          type: integer
          minimum: 1
        name:
          type: string
        tag:
          type: string
"##;

    let doc = SpecDocument::from_str(yaml).expect("Failed to parse OpenAPI YAML");
    assert_eq!(doc.spec.openapi.as_deref(), Some("3.0.3"));
    assert_eq!(doc.spec.info.as_ref().unwrap().title, "Petstore Mock API");
    assert!(doc.spec.paths.contains_key("/pets"));

    let pet_schema = doc.resolver.resolve(
        doc.spec.paths["/pets"]
            .get
            .as_ref()
            .unwrap()
            .responses["200"]
            .content["application/json"]
            .schema
            .as_ref()
            .unwrap()
            .items
            .as_ref()
            .unwrap(),
    );
    assert_eq!(pet_schema.get_primary_type(), Some(SchemaType::Object));
    assert!(pet_schema.properties.contains_key("name"));
    assert!(pet_schema.properties.contains_key("id"));
}

#[test]
fn test_parse_openapi_3_json() {
    let json = r##"{
      "openapi": "3.1.0",
      "info": {
        "title": "User Service",
        "version": "2.1.0"
      },
      "paths": {
        "/users/{id}": {
          "get": {
            "summary": "Get user by ID",
            "responses": {
              "200": {
                "description": "User found",
                "content": {
                  "application/json": {
                    "schema": {
                      "type": "object",
                      "properties": {
                        "id": { "type": "string", "format": "uuid" },
                        "email": { "type": "string", "format": "email" },
                        "active": { "type": "boolean" }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }"##;

    let doc = SpecDocument::from_str(json).expect("Failed to parse OpenAPI JSON");
    assert_eq!(doc.spec.openapi.as_deref(), Some("3.1.0"));
    assert!(doc.spec.paths.contains_key("/users/{id}"));
}

#[test]
fn test_parse_swagger_2_definitions() {
    let yaml = r##"
swagger: "2.0"
info:
  title: "Legacy Swagger API"
  version: "0.9.0"
paths:
  /orders:
    post:
      summary: "Create order"
      responses:
        "201":
          description: "Created"
          schema:
            $ref: "#/definitions/OrderResponse"
definitions:
  OrderResponse:
    type: object
    properties:
      orderId:
        type: string
        format: uuid
      total:
        type: number
"##;

    let doc = SpecDocument::from_str(yaml).expect("Failed to parse Swagger 2.0");
    assert_eq!(doc.spec.swagger.as_deref(), Some("2.0"));
    let op = doc.spec.paths["/orders"].post.as_ref().unwrap();
    let schema = op.responses["201"].schema.as_ref().unwrap();
    let resolved = doc.resolver.resolve(schema);
    assert_eq!(resolved.get_primary_type(), Some(SchemaType::Object));
    assert!(resolved.properties.contains_key("orderId"));
}

#[test]
fn test_parse_raw_json_schema() {
    let schema_json = r##"{
      "type": "object",
      "title": "StandaloneProduct",
      "properties": {
        "sku": { "type": "string" },
        "price": { "type": "number", "minimum": 0.99 }
      },
      "required": ["sku", "price"]
    }"##;

    let doc = SpecDocument::from_str(schema_json).expect("Failed to parse raw JSON Schema");
    assert!(doc.spec.paths.contains_key("/"));
    let root_op = doc.spec.paths["/"].get.as_ref().unwrap();
    let schema = root_op.responses["200"].content["application/json"]
        .schema
        .as_ref()
        .unwrap();
    assert_eq!(schema.get_primary_type(), Some(SchemaType::Object));
    assert!(schema.properties.contains_key("sku"));
}

#[test]
fn test_resolver_circular_reference_prevention() {
    let mut spec = OpenApiSpec::default();
    let mut comp = mockpulse_cli::schema::types::Components::default();

    let mut node_schema = mockpulse_cli::schema::types::Schema::default();
    node_schema.schema_type = Some(serde_json::Value::String("object".to_string()));
    node_schema.ref_path = Some("#/components/schemas/Node".to_string());

    comp.schemas.insert("Node".to_string(), node_schema.clone());
    spec.components = Some(comp);

    let resolver = ReferenceResolver::new(&spec);
    let resolved = resolver.resolve(&node_schema);
    // Should safely terminate and return without recursion stack overflow
    assert!(resolved.ref_path.is_some());
}
