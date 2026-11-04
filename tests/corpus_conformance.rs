//! The corpus item fixture (EP-0010 M5): the two shapes one document's
//! result takes when it crosses a binding, and the kind vocabulary a
//! consumer branches on, both pinned by `spec/tests/corpus/items.json`.
//!
//! No model is required, so this runs everywhere. The fixture's directory
//! walk is the one expectation that needs a parser, and the walk's shape
//! and order do not depend on what the parser finds; it runs inside the
//! crate (`src/lib.rs`, `a_directory_walk_yields_one_item_per_file`)
//! with a stand-in parser that returns no sentences, since the parser
//! trait is private to the crate.

use std::fs;
use std::path::PathBuf;

use matra::domain::{self, Error, Format};

#[derive(serde::Deserialize)]
struct Fixture {
    name: String,
    error_kinds: Vec<String>,
    item_shapes: ItemShapes,
    provisioning: Provisioning,
}

#[derive(serde::Deserialize)]
struct Provisioning {
    kinds: std::collections::BTreeMap<String, String>,
    // Read only by the udpipe-gated test below: the condition it
    // reproduces is a model directory that cannot be created, and there
    // is no model constructor to call without that feature.
    #[cfg_attr(not(feature = "udpipe"), allow(dead_code))]
    unwritable_model_dir: UnwritableModelDir,
}

#[derive(serde::Deserialize)]
#[cfg_attr(not(feature = "udpipe"), allow(dead_code))]
struct UnwritableModelDir {
    expect: ProvisioningExpect,
}

#[derive(serde::Deserialize)]
#[cfg_attr(not(feature = "udpipe"), allow(dead_code))]
struct ProvisioningExpect {
    kind: String,
    message_contains: Vec<String>,
}

#[derive(serde::Deserialize)]
struct ItemShapes {
    document: Vec<String>,
    error: Vec<String>,
    error_object: Vec<String>,
}

fn fixture() -> Fixture {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spec/tests/corpus/items.json");
    let raw =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("malformed fixture {}: {e}", path.display()))
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
        Error::UnsupportedFormat {
            format: Some(Format::Pdf),
            feature: None,
        },
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
    let document = domain::Document::new(
        domain::Provenance::new(
            domain::Identity::new("empty", None, None, None),
            Format::PlainText,
            domain::ProvenanceSource::new(Some(PathBuf::from("a.txt")), 0, ""),
        ),
        Vec::new(),
        Vec::new(),
    );

    let value = serde_json::to_value(&document).expect("serialize document");
    let mut keys: Vec<String> = value
        .as_object()
        .expect("a document is an object")
        .keys()
        .cloned()
        .collect();
    keys.sort();
    let mut expected = fixture.item_shapes.document.clone();
    expected.sort();
    assert_eq!(keys, expected);

    assert_eq!(fixture.item_shapes.error, ["path", "error"]);
    assert_eq!(fixture.item_shapes.error_object, ["kind", "message"]);
}

/// The provisioning classification RFC-0015 records, on the one failure
/// a runner can produce with no network and no model.
///
/// This row is not itself the contract change: `create_dir_all` already
/// converted through `Error::Io` before RFC-0015, so the kind here was
/// always `io`. The reclassification RFC-0015 records is DNS, TLS and a
/// full disk, which no runner exercises, and this row is the assertable
/// neighbour that keeps the vocabulary honest. What is new is the
/// message: `io error: Permission denied (os error 13)` was the whole of
/// it, with the directory sitting in a variable one line away.
#[cfg(feature = "udpipe")]
#[test]
fn a_model_directory_that_cannot_be_created_is_an_io_failure() {
    let fixture = fixture();
    let expect = &fixture.provisioning.unwritable_model_dir.expect;
    assert_eq!(
        fixture
            .provisioning
            .kinds
            .get("filesystem")
            .map(String::as_str),
        Some(expect.kind.as_str()),
        "the fixture's own rows have to agree with each other"
    );

    let parent = tempfile::tempdir().expect("tempdir");
    let blocked = parent.path().join("not-a-directory");
    fs::write(&blocked, b"x").expect("write");

    let cfg = matra::config::Config::from_sources(
        |key| (key == "HOME").then(|| parent.path().display().to_string()),
        None,
    )
    .expect("config")
    .with_model_dir(blocked.join("models"));
    let err = match matra::Engine::from_config(&cfg) {
        Ok(_) => panic!("a model directory under a regular file cannot be created"),
        Err(err) => err,
    };

    assert_eq!(err.kind(), expect.kind, "{err}");
    let message = err.to_string();
    for fragment in &expect.message_contains {
        assert!(
            message.contains(fragment),
            "{fragment:?} missing from {message}"
        );
    }
    assert!(
        message.contains(&blocked.display().to_string()),
        "the message names the path: {message}"
    );
}

/// The kinds the fixture names are kinds the vocabulary actually has. A
/// row naming a string no `Error` variant reports would pin a contract
/// nothing can satisfy.
#[test]
fn every_provisioning_kind_is_in_the_vocabulary() {
    let fixture = fixture();
    for (condition, kind) in &fixture.provisioning.kinds {
        assert!(
            fixture.error_kinds.contains(kind),
            "{condition} names {kind}, which is not one of the published kinds"
        );
    }
}
