//! Schema-driven mock JSON data synthesizer.

use super::formats;
use crate::schema::resolver::ReferenceResolver;
use crate::schema::types::{Schema, SchemaType};
use rand::Rng;
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Default)]
pub struct MockSynthesizer {
    pub max_depth: usize,
    pub array_min_items: usize,
    pub array_max_items: usize,
}

impl MockSynthesizer {
    pub fn new() -> Self {
        Self {
            max_depth: 8,
            array_min_items: 1,
            array_max_items: 3,
        }
    }

    pub fn synthesize(&self, schema: &Schema, resolver: &ReferenceResolver) -> Value {
        self.synthesize_internal(schema, resolver, 0)
    }

    fn synthesize_internal(
        &self,
        schema: &Schema,
        resolver: &ReferenceResolver,
        depth: usize,
    ) -> Value {
        if depth > self.max_depth {
            return Value::Object(Map::new());
        }

        let resolved_schema = resolver.resolve(schema);

        // 1. Direct example takes top priority
        if let Some(ref ex) = resolved_schema.example {
            return ex.clone();
        }

        // 2. Examples array / map
        if let Some(ref examples) = resolved_schema.examples {
            if let Some(first) = examples.first() {
                return first.clone();
            }
        }

        // 3. Enum values
        if let Some(ref enum_vals) = resolved_schema.enum_values {
            if !enum_vals.is_empty() {
                let mut rng = rand::thread_rng();
                let idx = rng.gen_range(0..enum_vals.len());
                return enum_vals[idx].clone();
            }
        }

        // 4. allOf composition
        if let Some(ref all_of) = resolved_schema.all_of {
            let mut merged_map = Map::new();
            for sub_schema in all_of {
                let val = self.synthesize_internal(sub_schema, resolver, depth + 1);
                if let Value::Object(map) = val {
                    for (k, v) in map {
                        merged_map.insert(k, v);
                    }
                }
            }
            if !merged_map.is_empty() {
                return Value::Object(merged_map);
            }
        }

        // 5. oneOf / anyOf composition
        if let Some(ref one_of) = resolved_schema.one_of {
            if let Some(first) = one_of.first() {
                return self.synthesize_internal(first, resolver, depth + 1);
            }
        }
        if let Some(ref any_of) = resolved_schema.any_of {
            if let Some(first) = any_of.first() {
                return self.synthesize_internal(first, resolver, depth + 1);
            }
        }

        // 6. Schema type synthesis
        let primary_type = resolved_schema.get_primary_type().or_else(|| {
            if !resolved_schema.properties.is_empty() {
                Some(SchemaType::Object)
            } else if resolved_schema.items.is_some() {
                Some(SchemaType::Array)
            } else {
                None
            }
        });

        match primary_type {
            Some(SchemaType::String) => self.synthesize_string(resolved_schema),
            Some(SchemaType::Integer) => self.synthesize_integer(resolved_schema),
            Some(SchemaType::Number) => self.synthesize_number(resolved_schema),
            Some(SchemaType::Boolean) => self.synthesize_boolean(resolved_schema),
            Some(SchemaType::Array) => self.synthesize_array(resolved_schema, resolver, depth),
            Some(SchemaType::Object) => self.synthesize_object(resolved_schema, resolver, depth),
            Some(SchemaType::Null) => Value::Null,
            None => {
                // Default fallback if type is unknown or arbitrary
                if !resolved_schema.properties.is_empty() {
                    self.synthesize_object(resolved_schema, resolver, depth)
                } else if resolved_schema.default.is_some() {
                    resolved_schema.default.clone().unwrap()
                } else {
                    json!({ "id": 1, "status": "active" })
                }
            }
        }
    }

    fn synthesize_string(&self, schema: &Schema) -> Value {
        if let Some(ref fmt) = schema.format {
            match fmt.to_lowercase().as_str() {
                "uuid" => return Value::String(formats::uuid_v4()),
                "email" => return Value::String(formats::email()),
                "date-time" => return Value::String(formats::date_time()),
                "date" => return Value::String(formats::date()),
                "time" => return Value::String(formats::time()),
                "ipv4" => return Value::String(formats::ipv4()),
                "ipv6" => return Value::String(formats::ipv6()),
                "uri" | "url" => return Value::String(formats::uri()),
                "hostname" => return Value::String(formats::hostname()),
                "byte" | "binary" => return Value::String(formats::base64_data()),
                _ => {}
            }
        }

        if let Some(ref title) = schema.title {
            return Value::String(format!("mock_{}", title.to_lowercase().replace(' ', "_")));
        }

        let s = formats::random_string(schema.min_length, schema.max_length);
        Value::String(s)
    }

    fn synthesize_integer(&self, schema: &Schema) -> Value {
        let mut rng = rand::thread_rng();
        let min = schema.minimum.map(|m| m as i64).unwrap_or(1);
        let max = schema.maximum.map(|m| m as i64).unwrap_or(100).max(min);
        let val = rng.gen_range(min..=max);
        Value::Number(serde_json::Number::from(val))
    }

    fn synthesize_number(&self, schema: &Schema) -> Value {
        let mut rng = rand::thread_rng();
        let min = schema.minimum.unwrap_or(1.0);
        let max = schema.maximum.unwrap_or(100.0).max(min);
        let val = rng.gen_range(min..=max);
        let rounded = (val * 100.0).round() / 100.0;
        serde_json::Number::from_f64(rounded)
            .map(Value::Number)
            .unwrap_or_else(|| json!(42.0))
    }

    fn synthesize_boolean(&self, _schema: &Schema) -> Value {
        let mut rng = rand::thread_rng();
        Value::Bool(rng.gen_bool(0.5))
    }

    fn synthesize_array(
        &self,
        schema: &Schema,
        resolver: &ReferenceResolver,
        depth: usize,
    ) -> Value {
        let mut rng = rand::thread_rng();
        let min = schema.min_items.unwrap_or(self.array_min_items);
        let max = schema.max_items.unwrap_or(self.array_max_items).max(min);
        let count = rng.gen_range(min..=max);

        let mut items = Vec::new();
        if let Some(ref item_schema) = schema.items {
            for _ in 0..count {
                items.push(self.synthesize_internal(item_schema, resolver, depth + 1));
            }
        } else {
            for i in 1..=count {
                items.push(json!(format!("item_{}", i)));
            }
        }

        Value::Array(items)
    }

    fn synthesize_object(
        &self,
        schema: &Schema,
        resolver: &ReferenceResolver,
        depth: usize,
    ) -> Value {
        let mut map = Map::new();

        if schema.properties.is_empty() {
            if let Some(ref default) = schema.default {
                return default.clone();
            }
            map.insert("id".to_string(), json!(1));
            map.insert("status".to_string(), json!("ok"));
            return Value::Object(map);
        }

        for (prop_name, prop_schema) in &schema.properties {
            let val = self.synthesize_internal(prop_schema, resolver, depth + 1);
            map.insert(prop_name.clone(), val);
        }

        Value::Object(map)
    }
}
