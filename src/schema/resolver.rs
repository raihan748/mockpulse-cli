//! Reference resolution for JSON Schema / OpenAPI references ($ref).

use super::types::{OpenApiSpec, Schema};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct ReferenceResolver {
    schemas: BTreeMap<String, Schema>,
}

impl ReferenceResolver {
    pub fn new(spec: &OpenApiSpec) -> Self {
        let mut schemas = BTreeMap::new();

        // Load OpenAPI 3.x components
        if let Some(ref components) = spec.components {
            for (name, schema) in &components.schemas {
                schemas.insert(format!("#/components/schemas/{}", name), schema.clone());
                schemas.insert(name.clone(), schema.clone());
            }
        }

        // Load Swagger 2.0 definitions
        if let Some(ref definitions) = spec.definitions {
            for (name, schema) in definitions {
                schemas.insert(format!("#/definitions/{}", name), schema.clone());
                schemas.insert(name.clone(), schema.clone());
            }
        }

        Self { schemas }
    }

    pub fn register(&mut self, key: String, schema: Schema) {
        self.schemas.insert(key, schema);
    }

    /// Resolves `$ref` transitively up to a safety depth of 16 to avoid circular stack overflows.
    pub fn resolve<'a>(&'a self, schema: &'a Schema) -> &'a Schema {
        let mut current = schema;
        let mut visited = HashSet::new();
        let mut depth = 0;

        while let Some(ref ref_path) = current.ref_path {
            if visited.contains(ref_path) || depth > 16 {
                break;
            }
            visited.insert(ref_path.clone());
            depth += 1;

            if let Some(resolved) = self.schemas.get(ref_path) {
                current = resolved;
            } else {
                // Try suffix match if e.g. ref_path is "#/components/schemas/User" and only "User" was keyed
                let leaf = ref_path.rsplit('/').next().unwrap_or(ref_path.as_str());
                if let Some(resolved) = self.schemas.get(leaf) {
                    current = resolved;
                } else {
                    break;
                }
            }
        }

        current
    }
}
