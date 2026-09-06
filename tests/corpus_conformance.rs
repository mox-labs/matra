//! The corpus item fixture (i10 M5): the two shapes one document's
//! result takes when it crosses a binding, and the kind vocabulary a
//! consumer branches on, both pinned by `spec/tests/corpus/items.json`.
//!
//! No model is required, so this runs everywhere. The parse is stubbed
//! with a provider that returns no sentences, because what the fixture
//! pins is the shape of the item and the order of the walk, not the
//! parse inside it: the parse fixtures in `spec/tests/*.json` do that.

use std::fs;
use std::path::PathBuf;

use matra::domain::{self, Error, Format};
use matra::nlp::NlpProvider;

#[derive(serde::Deserialize)]
struct Fixture {
    name: String,
    error_kinds: Vec<String>,
    item_shapes: ItemShapes,
    directory: Directory,
}

#[derive(serde::Deserialize)]
struct ItemShapes {
    entry: Vec<String>,
    error: Vec<String>,
    error_object: Vec<String>,
}

#[derive(serde::Deserialize)]
struct Directory {
    files: Vec<FixtureFile>,
    expect: Vec<Expect>,
}

#[derive(serde::Deserialize)]
struct FixtureFile {
    name: String,
    text: Option<String>,
    bytes: Option<Vec<u8>>,
}

#[derive(serde::Deserialize)]
struct Expect {
    name: String,
    shape: String,
    kind: Option<String>,
}

fn fixture() -> Fixture {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spec/tests/corpus/items.json");
    let raw =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("malformed fixture {}: {e}", path.display()))
}

/// Returns no sentences. The walk's shape and order are what is under
/// test here, and neither depends on what the parser found.
struct EmptyNlp;
impl NlpProvider for EmptyNlp {
    fn parse(&self, _text: &str) -> domain::Result<Vec<domain::Sentence>> {
        Ok(Vec::new())
    }
}

/// Every variant's `kind()`, in the fixture's order. Constructing each
/// one is the point: a variant added to `domain::Error` without a kind
/// fails to compile inside the crate, and a variant whose kind drifts
/// from the published vocabulary fails here.
#[test]
fn error_kind_vocabulary_matches_the_fixture() {
    let fixture = fixture();
    let variants = [
        Error::ModelNotFound(PathBuf::from("english.udpipe")),
        Error::ModelInvalid("truncated".to_string()),
        Error::ParseFailed("no parse".to_string()),
        Error::InputTooLarge {
            limit: 1,
            actual: 2,
            what: "input",
        },
        Error::UnsupportedFormat(Format::Pdf),
        Error::InvalidInput("bad argument".to_string()),
        Error::Io(std::io::Error::from(std::io::ErrorKind::NotFound)),
    ];

    assert_eq!(
        variants.len(),
        fixture.error_kinds.len(),
        "fixture {} names {} kinds for {} variants",
        fixture.name,
        fixture.error_kinds.len(),
        variants.len()
    );
    for (variant, expected) in variants.iter().zip(&fixture.error_kinds) {
        assert_eq!(variant.kind(), expected, "{variant}");
    }
}

/// The success item's keys are serde's, so they can be read off the
/// wire form directly. The failure item has no wire form (a
/// `DocumentError` wraps `std::io::Error`), so what this asserts is that
/// the fixture names the two facts a binding has to project by hand.
#[test]
fn the_item_shapes_are_the_ones_the_fixture_names() {
    let fixture = fixture();
    let entry = domain::CorpusEntry::new(
        Some(PathBuf::from("a.txt")),
        domain::Document::new(Vec::new()),
    );

    let value = serde_json::to_value(&entry).expect("serialize entry");
    let mut keys: Vec<String> = value
        .as_object()
        .expect("entry is an object")
        .keys()
        .cloned()
        .collect();
    keys.sort();
    let mut expected = fixture.item_shapes.entry.clone();
    expected.sort();
    assert_eq!(keys, expected);

    assert_eq!(fixture.item_shapes.error, ["path", "error"]);
    assert_eq!(fixture.item_shapes.error_object, ["kind", "message"]);
}

/// One readable file and one whose bytes are not UTF-8: two items, in
/// path order, the second an error item of the kind the fixture names.
/// The failure is reported rather than raised, and it does not end the
/// walk.
#[test]
fn a_directory_walk_yields_one_item_per_file() {
    let fixture = fixture();
    let dir = tempfile::tempdir().expect("tempdir");
    for file in &fixture.directory.files {
        let path = dir.path().join(&file.name);
        match (&file.text, &file.bytes) {
            (Some(text), None) => fs::write(&path, text).expect("write"),
            (None, Some(bytes)) => fs::write(&path, bytes).expect("write"),
            _ => panic!(
                "fixture file {} needs exactly one of text, bytes",
                file.name
            ),
        }
    }

    let engine = matra::Engine::new(Box::new(EmptyNlp), matra::standard_decomposers());
    let ingest = matra::Ingest::path(dir.path()).expect("listing the directory");
    let items: Vec<_> = engine.analyze(ingest).collect();

    assert_eq!(items.len(), fixture.directory.expect.len());
    for (item, expect) in items.iter().zip(&fixture.directory.expect) {
        match (item, expect.shape.as_str()) {
            (Ok(entry), "entry") => {
                let path = entry.path.as_ref().expect("a walked document has a path");
                assert_eq!(path.file_name().unwrap(), expect.name.as_str());
            }
            (Err(err), "error") => {
                let path = err.path.as_ref().expect("a walked document has a path");
                assert_eq!(path.file_name().unwrap(), expect.name.as_str());
                assert_eq!(err.error.kind(), expect.kind.as_deref().unwrap());
                assert!(!err.error.to_string().is_empty());
            }
            (got, shape) => panic!(
                "{} expected shape {shape}, got {}",
                expect.name,
                match got {
                    Ok(_) => "entry",
                    Err(_) => "error",
                }
            ),
        }
    }
}
