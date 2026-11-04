//! The published document schema, as the test suites check against it.
//!
//! Included with `mod document_schema;` by every runner whose output is a
//! document: the parse conformance lane, the corpus lane, the command
//! line's envelope lane and the schema's own tests.
//!
//! The check is stricter than the published schema in one way. The
//! schema lets objects carry fields it does not list, because a minor
//! version adds fields and a reader of the older minor must still
//! accept them. matra's own output must not carry a field the schema
//! for its own version does not list, or the schema is incomplete. So
//! every object definition that lists its properties is closed here
//! (`additionalProperties: false`) before validating, and a field that
//! is emitted but undocumented fails the test that emitted it.

use std::path::PathBuf;

/// Validate `document` against `spec/schema/v2/document.schema.json`,
/// closed as described above, and panic naming every violation.
pub(crate) fn assert_valid_document(document: &serde_json::Value, context: &str) {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spec/schema/v2/document.schema.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let mut schema: serde_json::Value = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()));
    close_objects(&mut schema);
    let validator = jsonschema::draft202012::new(&schema)
        .unwrap_or_else(|e| panic!("{} is not a valid JSON Schema: {e}", path.display()));
    let errors: Vec<String> = validator
        .iter_errors(document)
        .map(|e| format!("{} at {}", e, e.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "{context}: the document does not match schema 2:\n  {}",
        errors.join("\n  ")
    );
}

/// Set `additionalProperties: false` on every object schema that lists
/// `properties` and says nothing about additional ones.
fn close_objects(node: &mut serde_json::Value) {
    match node {
        serde_json::Value::Object(map) => {
            if map.contains_key("properties") && !map.contains_key("additionalProperties") {
                map.insert(
                    "additionalProperties".to_string(),
                    serde_json::Value::Bool(false),
                );
            }
            for value in map.values_mut() {
                close_objects(value);
            }
        }
        serde_json::Value::Array(items) => {
            for value in items {
                close_objects(value);
            }
        }
        _ => {}
    }
}
