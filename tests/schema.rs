//! The published document schema (`spec/schema/v2/`) against what the
//! library emits, with no model.
//!
//! The conformance lanes validate the real parser's output against the
//! same schema; this file keeps the check in every `cargo test`, through
//! a stand-in parser that emits every kind of record field: features,
//! negations, modals, clausal complements, root adverbials, Hearst
//! pairs and a root mood.

mod document_schema;

use matra::Engine;
use matra::domain::{self, Format, Sentence, Token};
use matra::nlp::NlpProvider;

/// One sentence for any input: "Smith never said that animals such as
/// dogs bark." with the arcs a live parse gives it, so every derived
/// field on `Sentence` is populated.
struct RecordParser;

impl NlpProvider for RecordParser {
    fn parse(&self, _text: &str) -> domain::Result<Vec<Sentence>> {
        let tok = |id: usize, text: &str, pos: &str, dep: &str, head: usize, feats: &str| {
            Token::builder(
                id,
                text.to_string(),
                text.to_lowercase(),
                pos.to_string(),
                head,
                dep.to_string(),
            )
            .feats(feats.to_string())
            .is_punct(pos == "PUNCT")
            .build()
        };
        Ok(vec![Sentence::new(
            "Smith never said that animals such as dogs may bark.".to_string(),
            vec![
                tok(1, "Smith", "PROPN", "nsubj", 3, "Number=Sing"),
                tok(2, "never", "ADV", "advmod", 3, "_"),
                tok(
                    3,
                    "said",
                    "VERB",
                    "root",
                    0,
                    "Mood=Ind|Tense=Past|VerbForm=Fin",
                ),
                tok(4, "that", "SCONJ", "mark", 10, "_"),
                tok(5, "animals", "NOUN", "nsubj", 10, "Number=Plur"),
                tok(6, "such", "ADJ", "case", 8, "Degree=Pos"),
                tok(7, "as", "ADP", "fixed", 6, "_"),
                tok(8, "dogs", "NOUN", "nmod", 5, "Number=Plur"),
                tok(9, "may", "AUX", "aux", 10, "VerbForm=Fin"),
                tok(10, "bark", "VERB", "ccomp", 3, "VerbForm=Inf"),
                tok(11, ".", "PUNCT", "punct", 3, "_"),
            ],
        )])
    }

    fn identity(&self) -> domain::Identity {
        domain::Identity::new(
            "record-parser",
            Some("fixture".to_string()),
            Some("0".repeat(64)),
            Some("MIT".to_string()),
        )
    }
}

fn analyzed(text: &str, format: Format) -> serde_json::Value {
    let engine = Engine::new(Box::new(RecordParser));
    let doc = engine
        .analyze(matra::Input::text(text, format))
        .expect("analyze");
    serde_json::to_value(&doc).expect("a document serializes")
}

#[test]
fn a_plain_text_document_matches_the_schema() {
    let doc = analyzed("One paragraph of prose.", Format::PlainText);
    // The stand-in emits every record field, so the check below is over
    // populated arrays and objects rather than empty ones.
    let sentence = &doc["sections"][0]["paragraphs"][0]["sentences"][0];
    for field in [
        "negations",
        "modals",
        "clausal_complements",
        "root_adverbials",
        "hearst_pairs",
    ] {
        assert!(
            !sentence[field].as_array().expect(field).is_empty(),
            "the stand-in parse populates {field}"
        );
    }
    assert!(sentence["root_mood"].is_object());
    document_schema::assert_valid_document(&doc, "plain text");
}

#[test]
fn a_markdown_document_with_a_blockquote_matches_the_schema() {
    let doc = analyzed(
        "# Title\n\nA paragraph.\n\n> A quotation, never parsed.\n\n## Second\n\nMore prose.\n",
        Format::Markdown,
    );
    assert_eq!(doc["provenance"]["format"], "markdown");
    document_schema::assert_valid_document(&doc, "markdown");
}

/// Lists, skipped constructs and their spans are in the schema too.
#[test]
fn a_markdown_document_with_skipped_constructs_matches_the_schema() {
    let doc = analyzed(
        "---\ntitle: x\n---\n\n# Title\n\n- an item\n- another\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n```\ncode\n```\n",
        Format::Markdown,
    );
    let kinds: Vec<&str> = doc["skipped"]
        .as_array()
        .expect("skipped")
        .iter()
        .map(|s| s["kind"].as_str().expect("kind"))
        .collect();
    assert_eq!(kinds, ["front_matter", "table", "code"]);
    assert_eq!(
        doc["sections"][0]["paragraphs"][0]["kind"], "list_item",
        "a list item is a paragraph of its own kind"
    );
    document_schema::assert_valid_document(&doc, "markdown with skipped constructs");
}

/// Block input, as JSON, validates against its own published schema,
/// reads into a document that validates against the document schema, and
/// a block the schema does not allow is refused by the reader too.
#[test]
fn block_input_matches_its_schema_and_reads_into_a_document() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("spec/schema/v2/blocks.schema.json");
    let schema: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("read")).expect("parse");
    let validator = jsonschema::draft202012::new(&schema).expect("a valid JSON Schema");

    let good = serde_json::json!([
        {"kind": "heading", "level": 1, "text": "Field notes"},
        {"kind": "paragraph", "text": "The river rose overnight."}
    ]);
    assert!(validator.is_valid(&good));
    let doc = analyzed(&good.to_string(), Format::Blocks);
    assert_eq!(doc["provenance"]["format"], "blocks");
    assert_eq!(doc["sections"][0]["heading"], "Field notes");
    document_schema::assert_valid_document(&doc, "block input");

    let engine = Engine::new(Box::new(RecordParser));
    for bad in [
        serde_json::json!([{"kind": "table", "text": "x"}]),
        serde_json::json!([{"kind": "heading", "level": 0, "text": "x"}]),
        serde_json::json!([{"kind": "paragraph", "text": "x", "style": "bold"}]),
    ] {
        assert!(!validator.is_valid(&bad), "the schema refuses {bad}");
        let err = engine
            .analyze(matra::Input::text(bad.to_string(), Format::Blocks))
            .expect_err("the reader refuses it too");
        assert_eq!(err.kind(), "invalid_input", "{bad}: {err}");
    }
}

/// A document with no sentences is where the ratios are null, and null
/// has to be something the schema allows.
#[test]
fn an_empty_document_matches_the_schema() {
    let doc = analyzed("", Format::PlainText);
    assert!(doc["metrics"]["mean_sentence_length"].is_null());
    document_schema::assert_valid_document(&doc, "empty");
}

/// The schema names one major, and the library emits that major.
#[test]
fn the_schema_pins_the_major_the_library_emits() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("spec/schema/v2/document.schema.json");
    let schema: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("read")).expect("parse");
    let pattern = schema["$defs"]["Provenance"]["properties"]["schema"]["pattern"]
        .as_str()
        .expect("the schema version carries a pattern");
    let major = domain::SCHEMA_VERSION
        .split('.')
        .next()
        .expect("major.minor");
    assert_eq!(pattern, format!("^{major}\\.[0-9]+$"));
}

/// The closed check is what catches an undocumented field: a document
/// carrying one more key than the schema lists fails.
#[test]
#[should_panic(expected = "does not match schema 2")]
fn an_undocumented_field_fails_the_check() {
    let mut doc = analyzed("Prose.", Format::PlainText);
    doc["metrics"]["undocumented"] = serde_json::json!(1);
    document_schema::assert_valid_document(&doc, "undocumented field");
}
