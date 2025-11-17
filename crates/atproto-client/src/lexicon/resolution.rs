//! Lexicon reference resolution
//!
//! This module handles resolving references within and across Lexicon schemas.
//! References can be local (`#defName`) or external (`nsid#defName`).
//!
//! # Example
//!
//! ```rust
//! use atproto_client::lexicon::{SchemaRegistry, LexiconDoc};
//!
//! // Create a schema registry
//! let mut registry = SchemaRegistry::new();
//!
//! // Load a schema
//! # let doc = LexiconDoc::new("com.example.test");
//! registry.register(doc);
//!
//! // Resolve a reference
//! let resolved = registry.resolve_ref("com.example.test", "#main");
//! ```

use super::schema::{LexiconDef, LexiconDoc};
use super::types::LexType;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Errors that can occur during reference resolution
#[derive(Debug, Error)]
pub enum RefResolutionError {
    /// Schema not found in registry
    #[error("Schema not found: {0}")]
    SchemaNotFound(String),

    /// Definition not found in schema
    #[error("Definition '{def}' not found in schema '{nsid}'")]
    DefNotFound {
        /// The NSID of the schema
        nsid: String,
        /// The definition name
        def: String,
    },

    /// Invalid reference format
    #[error("Invalid reference format: {0}")]
    InvalidRef(String),

    /// Circular reference detected
    #[error("Circular reference detected: {0}")]
    CircularReference(String),

    /// Reference resolution depth exceeded
    #[error("Reference resolution depth exceeded (max: {max}, path: {path})")]
    DepthExceeded {
        /// Maximum allowed depth
        max: usize,
        /// Resolution path
        path: String,
    },
}

/// Result type for reference resolution operations
pub type Result<T> = std::result::Result<T, RefResolutionError>;

/// Schema registry for caching and resolving Lexicon schemas
///
/// The registry stores loaded schemas and provides reference resolution
/// across schemas. It detects circular references and enforces depth limits.
///
/// # Example
///
/// ```rust
/// use atproto_client::lexicon::{SchemaRegistry, LexiconDoc};
///
/// let mut registry = SchemaRegistry::new();
///
/// // Register schemas
/// # let schema1 = LexiconDoc::new("com.example.foo");
/// # let schema2 = LexiconDoc::new("com.example.bar");
/// registry.register(schema1);
/// registry.register(schema2);
///
/// // Check if schema exists
/// assert!(registry.contains("com.example.foo"));
/// ```
#[derive(Debug, Clone)]
pub struct SchemaRegistry {
    /// Loaded schemas indexed by NSID
    schemas: HashMap<String, LexiconDoc>,

    /// Maximum resolution depth to prevent infinite loops
    max_depth: usize,
}

impl SchemaRegistry {
    /// Default maximum resolution depth
    pub const DEFAULT_MAX_DEPTH: usize = 100;

    /// Create a new empty schema registry
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
            max_depth: Self::DEFAULT_MAX_DEPTH,
        }
    }

    /// Create a new schema registry with custom max depth
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            schemas: HashMap::new(),
            max_depth,
        }
    }

    /// Register a schema in the registry
    ///
    /// If a schema with the same NSID already exists, it will be replaced.
    pub fn register(&mut self, doc: LexiconDoc) {
        self.schemas.insert(doc.id.clone(), doc);
    }

    /// Check if a schema is registered
    pub fn contains(&self, nsid: &str) -> bool {
        self.schemas.contains_key(nsid)
    }

    /// Get a schema by NSID
    pub fn get(&self, nsid: &str) -> Option<&LexiconDoc> {
        self.schemas.get(nsid)
    }

    /// Get a mutable reference to a schema by NSID
    pub fn get_mut(&mut self, nsid: &str) -> Option<&mut LexiconDoc> {
        self.schemas.get_mut(nsid)
    }

    /// Remove a schema from the registry
    pub fn unregister(&mut self, nsid: &str) -> Option<LexiconDoc> {
        self.schemas.remove(nsid)
    }

    /// Get the number of registered schemas
    pub fn len(&self) -> usize {
        self.schemas.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }

    /// Clear all schemas from the registry
    pub fn clear(&mut self) {
        self.schemas.clear();
    }

    /// Resolve a reference within a schema context
    ///
    /// # Arguments
    ///
    /// * `context_nsid` - The NSID of the schema where the reference appears
    /// * `ref_str` - The reference string (e.g., "#main" or "com.atproto.repo.strongRef#main")
    ///
    /// # Example
    ///
    /// ```rust
    /// use atproto_client::lexicon::SchemaRegistry;
    ///
    /// let registry = SchemaRegistry::new();
    ///
    /// // Resolve local reference
    /// // let def = registry.resolve_ref("com.example.post", "#main")?;
    ///
    /// // Resolve external reference
    /// // let def = registry.resolve_ref("com.example.post", "com.atproto.repo.strongRef#main")?;
    /// ```
    pub fn resolve_ref(&self, context_nsid: &str, ref_str: &str) -> Result<&LexiconDef> {
        let mut visited = HashSet::new();
        self.resolve_ref_internal(context_nsid, ref_str, &mut visited, 0)
    }

    /// Internal recursive reference resolution with circular detection
    fn resolve_ref_internal(
        &self,
        context_nsid: &str,
        ref_str: &str,
        visited: &mut HashSet<String>,
        depth: usize,
    ) -> Result<&LexiconDef> {
        // Check depth limit
        if depth > self.max_depth {
            return Err(RefResolutionError::DepthExceeded {
                max: self.max_depth,
                path: visited.iter().cloned().collect::<Vec<_>>().join(" -> "),
            });
        }

        // Parse the reference
        let (nsid, def_name) = parse_ref(ref_str, context_nsid)?;

        // Create a unique key for this reference
        let ref_key = format!("{}#{}", nsid, def_name);

        // Check for circular references
        if visited.contains(&ref_key) {
            return Err(RefResolutionError::CircularReference(ref_key));
        }

        visited.insert(ref_key.clone());

        // Get the schema
        let schema = self
            .schemas
            .get(&nsid)
            .ok_or_else(|| RefResolutionError::SchemaNotFound(nsid.clone()))?;

        // Get the definition
        let def = schema.defs.get(&def_name).ok_or_else(|| {
            RefResolutionError::DefNotFound {
                nsid: nsid.clone(),
                def: def_name.clone(),
            }
        })?;

        // Recursively resolve any refs within the definition
        // Collect all refs from the definition and validate they can be resolved
        let refs = collect_refs_from_def(def);
        for ref_str in refs {
            // Recursively resolve each ref found within the definition
            self.resolve_ref_internal(&nsid, &ref_str, visited, depth + 1)?;
        }

        Ok(def)
    }

    /// Resolve all references in a schema
    ///
    /// This validates that all references in the schema can be resolved.
    /// Returns an error if any reference cannot be resolved.
    pub fn validate_schema(&self, nsid: &str) -> Result<()> {
        let schema = self
            .schemas
            .get(nsid)
            .ok_or_else(|| RefResolutionError::SchemaNotFound(nsid.to_string()))?;

        // Walk through all definitions and resolve all refs
        for def in schema.defs.values() {
            let refs = collect_refs_from_def(def);
            for ref_str in refs {
                // Attempt to resolve each ref to ensure it's valid
                self.resolve_ref(nsid, &ref_str)?;
            }
        }

        Ok(())
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a reference string into (nsid, def_name)
///
/// # Format
///
/// - Local reference: `#defName` -> Uses context_nsid
/// - External reference: `nsid#defName` -> Uses specified nsid
///
/// # Examples
///
/// ```
/// # use atproto_client::lexicon::resolution::parse_ref;
/// let (nsid, def) = parse_ref("#main", "com.example.test").unwrap();
/// assert_eq!(nsid, "com.example.test");
/// assert_eq!(def, "main");
///
/// let (nsid, def) = parse_ref("com.atproto.repo.strongRef#main", "com.example.test").unwrap();
/// assert_eq!(nsid, "com.atproto.repo.strongRef");
/// assert_eq!(def, "main");
/// ```
pub fn parse_ref(ref_str: &str, context_nsid: &str) -> Result<(String, String)> {
    if !ref_str.contains('#') {
        return Err(RefResolutionError::InvalidRef(format!(
            "Reference must contain '#': {}",
            ref_str
        )));
    }

    let parts: Vec<&str> = ref_str.split('#').collect();
    if parts.len() != 2 {
        return Err(RefResolutionError::InvalidRef(format!(
            "Reference must have exactly one '#': {}",
            ref_str
        )));
    }

    let (nsid_part, def_name) = (parts[0], parts[1]);

    if def_name.is_empty() {
        return Err(RefResolutionError::InvalidRef(format!(
            "Definition name cannot be empty: {}",
            ref_str
        )));
    }

    let nsid = if nsid_part.is_empty() {
        // Local reference
        context_nsid.to_string()
    } else {
        // External reference
        nsid_part.to_string()
    };

    Ok((nsid, def_name.to_string()))
}

/// Collect all reference strings from a LexiconDef
///
/// This recursively traverses the definition structure and extracts
/// all reference strings found in any nested types.
fn collect_refs_from_def(def: &LexiconDef) -> Vec<String> {
    let mut refs = Vec::new();

    match def {
        LexiconDef::Record(record) => {
            // Collect refs from record schema (which is an object)
            for prop_type in record.record.properties.values() {
                refs.extend(collect_refs_from_type(prop_type));
            }
        }
        LexiconDef::Query(query) => {
            // Collect refs from parameters
            if let Some(params) = &query.parameters {
                for param_type in params.properties.values() {
                    refs.extend(collect_refs_from_type(param_type));
                }
            }
            // Collect refs from output body
            if let Some(output) = &query.output {
                if let Some(schema) = &output.schema {
                    refs.extend(collect_refs_from_type(schema));
                }
            }
        }
        LexiconDef::Procedure(procedure) => {
            // Collect refs from parameters
            if let Some(params) = &procedure.parameters {
                for param_type in params.properties.values() {
                    refs.extend(collect_refs_from_type(param_type));
                }
            }
            // Collect refs from input body
            if let Some(input) = &procedure.input {
                if let Some(schema) = &input.schema {
                    refs.extend(collect_refs_from_type(schema));
                }
            }
            // Collect refs from output body
            if let Some(output) = &procedure.output {
                if let Some(schema) = &output.schema {
                    refs.extend(collect_refs_from_type(schema));
                }
            }
        }
        LexiconDef::Subscription(subscription) => {
            // Collect refs from parameters
            if let Some(params) = &subscription.parameters {
                for param_type in params.properties.values() {
                    refs.extend(collect_refs_from_type(param_type));
                }
            }
            // Collect refs from message schema
            if let Some(message) = &subscription.message {
                refs.extend(collect_refs_from_type(message));
            }
        }
        LexiconDef::Object(object) => {
            // Collect refs from object properties
            for prop_type in object.properties.values() {
                refs.extend(collect_refs_from_type(prop_type));
            }
        }
        LexiconDef::Array(array) => {
            // Collect refs from array items
            refs.extend(collect_refs_from_type(&array.items));
        }
        LexiconDef::Union(union) => {
            // Union directly contains ref strings
            refs.extend(union.refs.clone());
        }
        // These types don't contain nested refs
        LexiconDef::Token(_)
        | LexiconDef::String(_)
        | LexiconDef::Integer(_)
        | LexiconDef::Boolean(_)
        | LexiconDef::Bytes(_)
        | LexiconDef::CidLink(_)
        | LexiconDef::Blob(_)
        | LexiconDef::Unknown(_) => {
            // No refs in these simple types
        }
    }

    refs
}

/// Collect all reference strings from a LexType
///
/// This recursively traverses the type structure and extracts
/// all reference strings found in any nested types.
fn collect_refs_from_type(lex_type: &LexType) -> Vec<String> {
    let mut refs = Vec::new();

    match lex_type {
        LexType::Ref(ref_type) => {
            // Found a ref - add it to the collection
            refs.push(ref_type.ref_to.clone());
        }
        LexType::Array(array) => {
            // Recursively collect refs from array items
            refs.extend(collect_refs_from_type(&array.items));
        }
        LexType::Object(object) => {
            // Recursively collect refs from object properties
            for prop_type in object.properties.values() {
                refs.extend(collect_refs_from_type(prop_type));
            }
        }
        LexType::Union(union) => {
            // Union directly contains ref strings
            refs.extend(union.refs.clone());
        }
        // These types don't contain nested refs
        LexType::Null
        | LexType::Boolean(_)
        | LexType::Integer(_)
        | LexType::String(_)
        | LexType::Bytes(_)
        | LexType::CidLink(_)
        | LexType::Blob(_)
        | LexType::Token(_)
        | LexType::Unknown(_) => {
            // No refs in these simple types
        }
    }

    refs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexicon::types::LexToken;

    #[test]
    fn test_parse_ref_local() {
        let (nsid, def) = parse_ref("#main", "com.example.test").unwrap();
        assert_eq!(nsid, "com.example.test");
        assert_eq!(def, "main");
    }

    #[test]
    fn test_parse_ref_external() {
        let (nsid, def) = parse_ref("com.atproto.repo.strongRef#main", "com.example.test").unwrap();
        assert_eq!(nsid, "com.atproto.repo.strongRef");
        assert_eq!(def, "main");
    }

    #[test]
    fn test_parse_ref_invalid_no_hash() {
        let result = parse_ref("invalid", "com.example.test");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RefResolutionError::InvalidRef(_)
        ));
    }

    #[test]
    fn test_parse_ref_invalid_multiple_hash() {
        let result = parse_ref("com.example#test#main", "com.example.test");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ref_invalid_empty_def() {
        let result = parse_ref("#", "com.example.test");
        assert!(result.is_err());
    }

    #[test]
    fn test_schema_registry_new() {
        let registry = SchemaRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_schema_registry_register() {
        let mut registry = SchemaRegistry::new();
        let doc = LexiconDoc::new("com.example.test");

        registry.register(doc);
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("com.example.test"));
    }

    #[test]
    fn test_schema_registry_get() {
        let mut registry = SchemaRegistry::new();
        let doc = LexiconDoc::new("com.example.test");
        registry.register(doc);

        let retrieved = registry.get("com.example.test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "com.example.test");
    }

    #[test]
    fn test_schema_registry_unregister() {
        let mut registry = SchemaRegistry::new();
        let doc = LexiconDoc::new("com.example.test");
        registry.register(doc);

        let removed = registry.unregister("com.example.test");
        assert!(removed.is_some());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_schema_registry_clear() {
        let mut registry = SchemaRegistry::new();
        registry.register(LexiconDoc::new("com.example.test1"));
        registry.register(LexiconDoc::new("com.example.test2"));

        registry.clear();
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_resolve_ref_local() {
        let mut registry = SchemaRegistry::new();

        let doc = LexiconDoc::new("com.example.test").with_def(
            "main",
            LexiconDef::Token(LexToken {
                type_name: "token".to_string(),
                description: Some("Test token".to_string()),
            }),
        );

        registry.register(doc);

        let result = registry.resolve_ref("com.example.test", "#main");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolve_ref_external() {
        let mut registry = SchemaRegistry::new();

        let doc1 = LexiconDoc::new("com.example.test1").with_def(
            "main",
            LexiconDef::Token(LexToken {
                type_name: "token".to_string(),
                description: None,
            }),
        );

        let doc2 = LexiconDoc::new("com.example.test2");

        registry.register(doc1);
        registry.register(doc2);

        let result = registry.resolve_ref("com.example.test2", "com.example.test1#main");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolve_ref_schema_not_found() {
        let registry = SchemaRegistry::new();

        let result = registry.resolve_ref("com.example.test", "#main");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RefResolutionError::SchemaNotFound(_)
        ));
    }

    #[test]
    fn test_resolve_ref_def_not_found() {
        let mut registry = SchemaRegistry::new();
        let doc = LexiconDoc::new("com.example.test");
        registry.register(doc);

        let result = registry.resolve_ref("com.example.test", "#main");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RefResolutionError::DefNotFound { .. }
        ));
    }

    #[test]
    fn test_circular_reference_detection() {
        let mut registry = SchemaRegistry::new();

        // Create a schema with a self-referencing definition
        // This is a simplified test - actual circular refs would be in the Ref type
        let doc = LexiconDoc::new("com.example.test").with_def(
            "main",
            LexiconDef::Token(LexToken {
                type_name: "token".to_string(),
                description: None,
            }),
        );

        registry.register(doc);

        // Create a visited set with the reference already in it to simulate circular ref
        let mut visited = HashSet::new();
        visited.insert("com.example.test#main".to_string());

        let result = registry.resolve_ref_internal("com.example.test", "#main", &mut visited, 0);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RefResolutionError::CircularReference(_)
        ));
    }

    #[test]
    fn test_depth_limit() {
        let registry = SchemaRegistry::with_max_depth(5);
        let mut visited = HashSet::new();

        let result = registry.resolve_ref_internal("com.example.test", "#main", &mut visited, 10);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RefResolutionError::DepthExceeded { .. }
        ));
    }

    #[test]
    fn test_complex_schema_with_multiple_refs() {
        use crate::lexicon::types::{LexObject, LexRefType, LexString, LexType};
        use std::collections::HashMap;

        let mut registry = SchemaRegistry::new();

        // Create a base type schema
        let mut string_properties = HashMap::new();
        string_properties.insert(
            "value".to_string(),
            LexType::String(LexString {
                type_name: "string".to_string(),
                description: None,
                format: None,
                constraints: Default::default(),
            }),
        );

        let base_schema = LexiconDoc::new("com.example.types").with_def(
            "stringValue",
            LexiconDef::Object(LexObject {
                type_name: "object".to_string(),
                description: Some("A string value object".to_string()),
                properties: string_properties,
                required: Some(vec!["value".to_string()]),
                nullable: None,
            }),
        );

        // Create a schema that references the base type
        let mut post_properties = HashMap::new();
        post_properties.insert(
            "title".to_string(),
            LexType::Ref(LexRefType {
                type_name: "ref".to_string(),
                description: None,
                ref_to: "com.example.types#stringValue".to_string(),
            }),
        );

        let post_schema = LexiconDoc::new("com.example.post").with_def(
            "main",
            LexiconDef::Object(LexObject {
                type_name: "object".to_string(),
                description: Some("A post object".to_string()),
                properties: post_properties,
                required: Some(vec!["title".to_string()]),
                nullable: None,
            }),
        );

        registry.register(base_schema);
        registry.register(post_schema);

        // Resolve local reference in post schema
        let result = registry.resolve_ref("com.example.post", "#main");
        assert!(result.is_ok());

        // Resolve external reference to base type
        let result = registry.resolve_ref("com.example.post", "com.example.types#stringValue");
        assert!(result.is_ok());
    }

    #[test]
    fn test_union_type_with_refs() {
        use crate::lexicon::types::LexUnion;

        let mut registry = SchemaRegistry::new();

        // Create schemas for union variants
        let text_schema = LexiconDoc::new("com.example.embed.text").with_def(
            "main",
            LexiconDef::Token(LexToken {
                type_name: "token".to_string(),
                description: Some("Text embed".to_string()),
            }),
        );

        let image_schema = LexiconDoc::new("com.example.embed.image").with_def(
            "main",
            LexiconDef::Token(LexToken {
                type_name: "token".to_string(),
                description: Some("Image embed".to_string()),
            }),
        );

        // Create a schema with a union type
        let post_schema = LexiconDoc::new("com.example.post").with_def(
            "main",
            LexiconDef::Union(LexUnion {
                type_name: "union".to_string(),
                description: Some("Post embed union".to_string()),
                refs: vec![
                    "com.example.embed.text#main".to_string(),
                    "com.example.embed.image#main".to_string(),
                ],
                closed: Some(true),
            }),
        );

        registry.register(text_schema);
        registry.register(image_schema);
        registry.register(post_schema);

        // Resolve union type
        let result = registry.resolve_ref("com.example.post", "#main");
        assert!(result.is_ok());

        // Resolve union variant references
        let result = registry.resolve_ref("com.example.post", "com.example.embed.text#main");
        assert!(result.is_ok());

        let result = registry.resolve_ref("com.example.post", "com.example.embed.image#main");
        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_object_refs() {
        use crate::lexicon::types::{LexObject, LexRefType, LexType};
        use std::collections::HashMap;

        let mut registry = SchemaRegistry::new();

        // Create nested type schemas
        let author_schema = LexiconDoc::new("com.example.types.author").with_def(
            "main",
            LexiconDef::Token(LexToken {
                type_name: "token".to_string(),
                description: Some("Author".to_string()),
            }),
        );

        let mut post_properties = HashMap::new();
        post_properties.insert(
            "author".to_string(),
            LexType::Ref(LexRefType {
                type_name: "ref".to_string(),
                description: None,
                ref_to: "com.example.types.author#main".to_string(),
            }),
        );

        let post_schema = LexiconDoc::new("com.example.types.post").with_def(
            "main",
            LexiconDef::Object(LexObject {
                type_name: "object".to_string(),
                description: Some("Post with author ref".to_string()),
                properties: post_properties,
                required: Some(vec!["author".to_string()]),
                nullable: None,
            }),
        );

        let mut feed_properties = HashMap::new();
        feed_properties.insert(
            "post".to_string(),
            LexType::Ref(LexRefType {
                type_name: "ref".to_string(),
                description: None,
                ref_to: "com.example.types.post#main".to_string(),
            }),
        );

        let feed_schema = LexiconDoc::new("com.example.feed").with_def(
            "main",
            LexiconDef::Object(LexObject {
                type_name: "object".to_string(),
                description: Some("Feed with post ref".to_string()),
                properties: feed_properties,
                required: Some(vec!["post".to_string()]),
                nullable: None,
            }),
        );

        registry.register(author_schema);
        registry.register(post_schema);
        registry.register(feed_schema);

        // Resolve nested references
        let result = registry.resolve_ref("com.example.feed", "com.example.types.post#main");
        assert!(result.is_ok());

        let result = registry.resolve_ref("com.example.feed", "com.example.types.author#main");
        assert!(result.is_ok());
    }

    #[test]
    fn test_registry_replace_schema() {
        let mut registry = SchemaRegistry::new();

        // Register initial version
        let doc_v1 = LexiconDoc::new("com.example.test").with_def(
            "main",
            LexiconDef::Token(LexToken {
                type_name: "token".to_string(),
                description: Some("Version 1".to_string()),
            }),
        );

        registry.register(doc_v1);
        assert_eq!(registry.len(), 1);

        // Replace with new version
        let doc_v2 = LexiconDoc::new("com.example.test").with_def(
            "main",
            LexiconDef::Token(LexToken {
                type_name: "token".to_string(),
                description: Some("Version 2".to_string()),
            }),
        );

        registry.register(doc_v2);
        assert_eq!(registry.len(), 1); // Still only one schema

        // Verify it's the new version
        let schema = registry.get("com.example.test").unwrap();
        let def = schema.defs.get("main").unwrap();
        match def {
            LexiconDef::Token(token) => {
                assert_eq!(token.description, Some("Version 2".to_string()));
            }
            _ => panic!("Expected Token definition"),
        }
    }

    #[test]
    fn test_validate_schema_exists() {
        let mut registry = SchemaRegistry::new();
        let doc = LexiconDoc::new("com.example.test");
        registry.register(doc);

        // Validate existing schema
        let result = registry.validate_schema("com.example.test");
        assert!(result.is_ok());

        // Validate non-existent schema
        let result = registry.validate_schema("com.example.nonexistent");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RefResolutionError::SchemaNotFound(_)
        ));
    }

    #[test]
    fn test_validate_schema_with_refs() {
        use crate::lexicon::types::{LexObject, LexRefType, LexString, LexType};
        use std::collections::HashMap;

        let mut registry = SchemaRegistry::new();

        // Create a base type schema
        let mut string_properties = HashMap::new();
        string_properties.insert(
            "value".to_string(),
            LexType::String(LexString {
                type_name: "string".to_string(),
                description: None,
                format: None,
                constraints: Default::default(),
            }),
        );

        let base_schema = LexiconDoc::new("com.example.types").with_def(
            "stringValue",
            LexiconDef::Object(LexObject {
                type_name: "object".to_string(),
                description: Some("A string value object".to_string()),
                properties: string_properties,
                required: Some(vec!["value".to_string()]),
                nullable: None,
            }),
        );

        // Create a schema that references the base type
        let mut post_properties = HashMap::new();
        post_properties.insert(
            "title".to_string(),
            LexType::Ref(LexRefType {
                type_name: "ref".to_string(),
                description: None,
                ref_to: "com.example.types#stringValue".to_string(),
            }),
        );

        let post_schema = LexiconDoc::new("com.example.post").with_def(
            "main",
            LexiconDef::Object(LexObject {
                type_name: "object".to_string(),
                description: Some("A post object".to_string()),
                properties: post_properties,
                required: Some(vec!["title".to_string()]),
                nullable: None,
            }),
        );

        registry.register(base_schema);
        registry.register(post_schema);

        // Validate the post schema - should succeed because base type exists
        let result = registry.validate_schema("com.example.post");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_schema_with_missing_refs() {
        use crate::lexicon::types::{LexObject, LexRefType, LexType};
        use std::collections::HashMap;

        let mut registry = SchemaRegistry::new();

        // Create a schema that references a non-existent type
        let mut post_properties = HashMap::new();
        post_properties.insert(
            "title".to_string(),
            LexType::Ref(LexRefType {
                type_name: "ref".to_string(),
                description: None,
                ref_to: "com.example.missing#stringValue".to_string(),
            }),
        );

        let post_schema = LexiconDoc::new("com.example.post").with_def(
            "main",
            LexiconDef::Object(LexObject {
                type_name: "object".to_string(),
                description: Some("A post object".to_string()),
                properties: post_properties,
                required: Some(vec!["title".to_string()]),
                nullable: None,
            }),
        );

        registry.register(post_schema);

        // Validate the post schema - should fail because referenced type doesn't exist
        let result = registry.validate_schema("com.example.post");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RefResolutionError::SchemaNotFound(_)
        ));
    }

    #[test]
    fn test_validate_schema_with_nested_refs() {
        use crate::lexicon::types::{LexArray, LexObject, LexRefType, LexType};
        use std::collections::HashMap;

        let mut registry = SchemaRegistry::new();

        // Create a base item type
        let item_schema = LexiconDoc::new("com.example.item").with_def(
            "main",
            LexiconDef::Token(LexToken {
                type_name: "token".to_string(),
                description: Some("Item".to_string()),
            }),
        );

        // Create a schema with nested array of refs
        let mut collection_properties = HashMap::new();
        collection_properties.insert(
            "items".to_string(),
            LexType::Array(LexArray {
                type_name: "array".to_string(),
                description: None,
                items: Box::new(LexType::Ref(LexRefType {
                    type_name: "ref".to_string(),
                    description: None,
                    ref_to: "com.example.item#main".to_string(),
                })),
                constraints: Default::default(),
            }),
        );

        let collection_schema = LexiconDoc::new("com.example.collection").with_def(
            "main",
            LexiconDef::Object(LexObject {
                type_name: "object".to_string(),
                description: Some("Collection of items".to_string()),
                properties: collection_properties,
                required: Some(vec!["items".to_string()]),
                nullable: None,
            }),
        );

        registry.register(item_schema);
        registry.register(collection_schema);

        // Validate the collection schema - should succeed
        let result = registry.validate_schema("com.example.collection");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_schema_with_union_refs() {
        use crate::lexicon::types::LexUnion;

        let mut registry = SchemaRegistry::new();

        // Create variant schemas
        let text_schema = LexiconDoc::new("com.example.text").with_def(
            "main",
            LexiconDef::Token(LexToken {
                type_name: "token".to_string(),
                description: Some("Text".to_string()),
            }),
        );

        let image_schema = LexiconDoc::new("com.example.image").with_def(
            "main",
            LexiconDef::Token(LexToken {
                type_name: "token".to_string(),
                description: Some("Image".to_string()),
            }),
        );

        // Create a union schema
        let union_schema = LexiconDoc::new("com.example.content").with_def(
            "main",
            LexiconDef::Union(LexUnion {
                type_name: "union".to_string(),
                description: Some("Content union".to_string()),
                refs: vec![
                    "com.example.text#main".to_string(),
                    "com.example.image#main".to_string(),
                ],
                closed: Some(true),
            }),
        );

        registry.register(text_schema);
        registry.register(image_schema);
        registry.register(union_schema);

        // Validate the union schema - should succeed
        let result = registry.validate_schema("com.example.content");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_schema_with_union_missing_ref() {
        use crate::lexicon::types::LexUnion;

        let mut registry = SchemaRegistry::new();

        // Create only one variant schema
        let text_schema = LexiconDoc::new("com.example.text").with_def(
            "main",
            LexiconDef::Token(LexToken {
                type_name: "token".to_string(),
                description: Some("Text".to_string()),
            }),
        );

        // Create a union schema that references a missing type
        let union_schema = LexiconDoc::new("com.example.content").with_def(
            "main",
            LexiconDef::Union(LexUnion {
                type_name: "union".to_string(),
                description: Some("Content union".to_string()),
                refs: vec![
                    "com.example.text#main".to_string(),
                    "com.example.missing#main".to_string(), // This doesn't exist
                ],
                closed: Some(true),
            }),
        );

        registry.register(text_schema);
        registry.register(union_schema);

        // Validate the union schema - should fail
        let result = registry.validate_schema("com.example.content");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RefResolutionError::SchemaNotFound(_)
        ));
    }

    #[test]
    fn test_recursive_ref_resolution_in_nested_objects() {
        use crate::lexicon::types::{LexObject, LexRefType, LexType};
        use std::collections::HashMap;

        let mut registry = SchemaRegistry::new();

        // Create deeply nested type schemas
        let level3_schema = LexiconDoc::new("com.example.level3").with_def(
            "main",
            LexiconDef::Token(LexToken {
                type_name: "token".to_string(),
                description: Some("Level 3".to_string()),
            }),
        );

        let mut level2_properties = HashMap::new();
        level2_properties.insert(
            "nested".to_string(),
            LexType::Ref(LexRefType {
                type_name: "ref".to_string(),
                description: None,
                ref_to: "com.example.level3#main".to_string(),
            }),
        );

        let level2_schema = LexiconDoc::new("com.example.level2").with_def(
            "main",
            LexiconDef::Object(LexObject {
                type_name: "object".to_string(),
                description: Some("Level 2".to_string()),
                properties: level2_properties,
                required: Some(vec!["nested".to_string()]),
                nullable: None,
            }),
        );

        let mut level1_properties = HashMap::new();
        level1_properties.insert(
            "nested".to_string(),
            LexType::Ref(LexRefType {
                type_name: "ref".to_string(),
                description: None,
                ref_to: "com.example.level2#main".to_string(),
            }),
        );

        let level1_schema = LexiconDoc::new("com.example.level1").with_def(
            "main",
            LexiconDef::Object(LexObject {
                type_name: "object".to_string(),
                description: Some("Level 1".to_string()),
                properties: level1_properties,
                required: Some(vec!["nested".to_string()]),
                nullable: None,
            }),
        );

        registry.register(level3_schema);
        registry.register(level2_schema);
        registry.register(level1_schema);

        // Resolve level1 - should recursively resolve through level2 to level3
        let result = registry.resolve_ref("com.example.level1", "#main");
        assert!(result.is_ok());

        // Validate all schemas
        assert!(registry.validate_schema("com.example.level1").is_ok());
        assert!(registry.validate_schema("com.example.level2").is_ok());
        assert!(registry.validate_schema("com.example.level3").is_ok());
    }
}
