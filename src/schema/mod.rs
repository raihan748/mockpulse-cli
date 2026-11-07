//! OpenAPI and JSON Schema parsing, validation, and resolution.

pub mod parser;
pub mod resolver;
pub mod types;

pub use parser::{SchemaError, SpecDocument};
pub use resolver::ReferenceResolver;
pub use types::*;
