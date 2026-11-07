//! Unit tests for schema-driven mock synthesizer and formats.

use mockpulse_cli::schema::resolver::ReferenceResolver;
use mockpulse_cli::schema::types::{OpenApiSpec, Schema};
use mockpulse_cli::synthesizer::formats;
use mockpulse_cli::synthesizer::MockSynthesizer;
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn test_uuid_v4_format() {
    let uuid = formats::uuid_v4();
    assert_eq!(uuid.len(), 36);
    let parts: Vec<&str> = uuid.split('-').collect();
    assert_eq!(parts.len(), 5);
    assert_eq!(parts[0].len(), 8);
    assert_eq!(parts[1].len(), 4);
    assert_eq!(parts[2].len(), 4);
    assert_eq!(parts[3].len(), 4);
    assert_eq!(parts[4].len(), 12);
    // Version 4 check: 13th char (first of 3rd part) is '4'
    assert!(parts[2].starts_with('4'));
}

#[test]
fn test_email_and_network_formats() {
    let email = formats::email();
    assert!(email.contains('@'));
    assert!(email.starts_with("user_"));

    let ip = formats::ipv4();
    let octets: Vec<&str> = ip.split('.').collect();
    assert_eq!(octets.len(), 4);
    for octet in octets {
        let n: u16 = octet.parse().expect("Valid IPv4 octet");
        assert!(n <= 255);
    }
}

#[test]
fn test_synthesizer_with_explicit_example() {
    let mut schema = Schema::default();
    schema.example = Some(json!({ "id": "custom-123", "score": 99.5 }));

    let spec = OpenApiSpec::default();
    let resolver = ReferenceResolver::new(&spec);
    let synth = MockSynthesizer::new();

    let val = synth.synthesize(&schema, &resolver);
    assert_eq!(val["id"], "custom-123");
    assert_eq!(val["score"], 99.5);
}

#[test]
fn test_synthesizer_enum_selection() {
    let mut schema = Schema::default();
    schema.schema_type = Some(json!("string"));
    schema.enum_values = Some(vec![json!("PENDING"), json!("ACTIVE"), json!("SUSPENDED")]);

    let spec = OpenApiSpec::default();
    let resolver = ReferenceResolver::new(&spec);
    let synth = MockSynthesizer::new();

    for _ in 0..10 {
        let val = synth.synthesize(&schema, &resolver);
        let s = val.as_str().unwrap();
        assert!(s == "PENDING" || s == "ACTIVE" || s == "SUSPENDED");
    }
}

#[test]
fn test_synthesizer_integer_and_number_bounds() {
    let mut int_schema = Schema::default();
    int_schema.schema_type = Some(json!("integer"));
    int_schema.minimum = Some(10.0);
    int_schema.maximum = Some(20.0);

    let spec = OpenApiSpec::default();
    let resolver = ReferenceResolver::new(&spec);
    let synth = MockSynthesizer::new();

    for _ in 0..20 {
        let val = synth.synthesize(&int_schema, &resolver);
        let n = val.as_i64().expect("Integer value");
        assert!(n >= 10 && n <= 20);
    }

    let mut num_schema = Schema::default();
    num_schema.schema_type = Some(json!("number"));
    num_schema.minimum = Some(50.0);
    num_schema.maximum = Some(60.0);

    for _ in 0..20 {
        let val = synth.synthesize(&num_schema, &resolver);
        let f = val.as_f64().expect("Number value");
        assert!(f >= 50.0 && f <= 60.0);
    }
}

#[test]
fn test_synthesizer_nested_object_and_arrays() {
    let mut user_schema = Schema::default();
    user_schema.schema_type = Some(json!("object"));

    let mut props = BTreeMap::new();

    let mut id_schema = Schema::default();
    id_schema.schema_type = Some(json!("string"));
    id_schema.format = Some("uuid".to_string());
    props.insert("id".to_string(), id_schema);

    let mut email_schema = Schema::default();
    email_schema.schema_type = Some(json!("string"));
    email_schema.format = Some("email".to_string());
    props.insert("email".to_string(), email_schema);

    let mut tags_schema = Schema::default();
    tags_schema.schema_type = Some(json!("array"));
    let mut tag_item = Schema::default();
    tag_item.schema_type = Some(json!("string"));
    tags_schema.items = Some(Box::new(tag_item));
    tags_schema.min_items = Some(2);
    tags_schema.max_items = Some(4);
    props.insert("tags".to_string(), tags_schema);

    user_schema.properties = props;
    user_schema.required = vec!["id".to_string(), "email".to_string()];

    let spec = OpenApiSpec::default();
    let resolver = ReferenceResolver::new(&spec);
    let synth = MockSynthesizer::new();

    let res = synth.synthesize(&user_schema, &resolver);
    assert!(res.is_object());
    let id_str = res["id"].as_str().expect("UUID string");
    assert_eq!(id_str.len(), 36);

    let email_str = res["email"].as_str().expect("Email string");
    assert!(email_str.contains('@'));

    let tags = res["tags"].as_array().expect("Tags array");
    assert!(tags.len() >= 2 && tags.len() <= 4);
}

#[test]
fn test_synthesizer_all_of_composition() {
    let mut base_schema = Schema::default();
    base_schema.schema_type = Some(json!("object"));
    let mut base_props = BTreeMap::new();
    let mut id_schema = Schema::default();
    id_schema.schema_type = Some(json!("integer"));
    base_props.insert("id".to_string(), id_schema);
    base_schema.properties = base_props;

    let mut ext_schema = Schema::default();
    ext_schema.schema_type = Some(json!("object"));
    let mut ext_props = BTreeMap::new();
    let mut name_schema = Schema::default();
    name_schema.schema_type = Some(json!("string"));
    ext_props.insert("name".to_string(), name_schema);
    ext_schema.properties = ext_props;

    let mut composed_schema = Schema::default();
    composed_schema.all_of = Some(vec![base_schema, ext_schema]);

    let spec = OpenApiSpec::default();
    let resolver = ReferenceResolver::new(&spec);
    let synth = MockSynthesizer::new();

    let val = synth.synthesize(&composed_schema, &resolver);
    assert!(val.get("id").is_some());
    assert!(val.get("name").is_some());
}
