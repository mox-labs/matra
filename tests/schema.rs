//! The published schemas (`spec/schema/v2/`) against the library, with no
//! model and no parser.
//!
//! What the pipeline emits is checked against the schemas inside the
//! crate (`src/lib.rs`, the schema lane), where a stand-in parser can be
//! wired; the conformance lanes check the real parser's output. This file
//! holds what needs neither: the version the schemas name, and how they
//! refer to one another.

use std::path::PathBuf;

use matra::domain;

fn schema(name: &str) -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("spec/schema/v2")
        .join(format!("{name}.schema.json"));
    serde_json::from_str(&std::fs::read_to_string(path).expect("read")).expect("parse")
}

/// The schema names one major, and the library emits that major.
#[test]
fn the_schema_pins_the_major_the_library_emits() {
    let document = schema("document");
    let pattern = document["$defs"]["Provenance"]["properties"]["schema"]["pattern"]
        .as_str()
        .expect("the schema version carries a pattern");
    let major = domain::SCHEMA_VERSION
        .split('.')
        .next()
        .expect("major.minor");
    assert_eq!(pattern, format!("^{major}\\.[0-9]+$"));
}

/// Every outcome schema lives beside the document schema, under the same
/// major, and takes its provenance from the document schema's one
/// definition rather than a copy of it.
#[test]
fn every_outcome_schema_takes_its_provenance_from_the_document_schema() {
    let id = schema("document")["$id"]
        .as_str()
        .expect("the document schema names its $id")
        .to_string();
    for name in ["summary", "keyphrases", "clusters"] {
        let outcome = schema(name);
        assert_eq!(
            outcome["$id"],
            id.replace("document.schema.json", &format!("{name}.schema.json")),
            "{name}"
        );
        let provenance = outcome["properties"]["provenance"]["allOf"][0]["$ref"]
            .as_str()
            .expect("the provenance is a reference");
        assert_eq!(provenance, format!("{id}#/$defs/Provenance"), "{name}");
    }
}
