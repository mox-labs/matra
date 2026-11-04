//! The published schemas, as the test suites check against them.
//!
//! Included with `mod document_schema;` by every runner whose output is a
//! document or an outcome: the parse conformance lane, the outcome lane,
//! the command line's envelope lane, and the crate's own model-free lanes
//! in `src/lib.rs`, which include it by path.
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

/// Validate `value` against `spec/schema/v2/{name}.schema.json`, closed as
/// described above, and panic naming every violation.
///
/// `name` is `document`, `summary`, `keyphrases` or `clusters`. The
/// outcome schemas refer to the document schema's definitions by its
/// `$id`, so the document schema, closed the same way, is registered
/// under that `$id` for every check.
pub(crate) fn assert_valid(name: &str, value: &serde_json::Value, context: &str) {
    let document = closed_schema("document");
    let id = document["$id"]
        .as_str()
        .expect("the document schema names its $id")
        .to_string();
    let registry = jsonschema::Registry::new()
        .add(id, document)
        .and_then(|r| r.prepare())
        .unwrap_or_else(|e| panic!("the document schema does not register: {e}"));
    let schema = closed_schema(name);
    let validator = jsonschema::options()
        .with_registry(&registry)
        .build(&schema)
        .unwrap_or_else(|e| panic!("{name}.schema.json is not a valid JSON Schema: {e}"));
    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|e| format!("{} at {}", e, e.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "{context}: the {name} does not match schema 2:\n  {}",
        errors.join("\n  ")
    );
}

/// `spec/schema/v2/{name}.schema.json`, with every object closed.
fn closed_schema(name: &str) -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("spec/schema/v2")
        .join(format!("{name}.schema.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let mut schema: serde_json::Value = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()));
    close_objects(&mut schema);
    schema
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
