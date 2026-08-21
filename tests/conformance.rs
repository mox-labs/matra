//! Conformance tests: the Rust crust against the shared spec.
//!
//! Every fixture in `spec/tests/` is run through the Rust API and checked
//! against the same expectations the Python crust checks. A difference
//! between crusts is a binding defect, not a behaviour difference.
//!
//! Requires the UDPipe model, so these are `#[ignore]` by default:
//!
//!     cargo test --test conformance -- --ignored

#![cfg(feature = "udpipe")]

use std::fs;
use std::path::PathBuf;

use matra::Engine;
use matra::domain::{Document, Format, RawDocument};
use matra::nlp::udpipe::Udpipe;
use serde::Deserialize;

const TOLERANCE: f64 = 1e-6;

#[derive(Deserialize)]
struct Fixture {
    name: String,
    input: String,
    format: String,
    expect: Expect,
}

#[derive(Deserialize)]
struct Expect {
    total_sentences: usize,
    total_words: usize,
    paragraph_count: usize,
    sentences: Vec<ExpectedSentence>,
    vocabulary_ttr: Option<f64>,
    nominalization_ratio: Option<f64>,
    passive_ratio: Option<f64>,
}

#[derive(Deserialize)]
struct ExpectedSentence {
    text: String,
    token_count: usize,
    /// Expected negation cues, when the fixture pins them. `None` means
    /// the fixture predates the field and the check is skipped.
    negations: Option<Vec<ExpectedNegation>>,
    tokens: Vec<ExpectedToken>,
}

#[derive(Deserialize)]
struct ExpectedNegation {
    cue_id: usize,
    cue_lemma: String,
    head_id: usize,
}

#[derive(Deserialize)]
struct ExpectedToken {
    id: usize,
    text: String,
    lemma: String,
    pos: String,
    head: usize,
    dep: String,
}

fn spec_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("spec")
        .join("tests")
}

fn model_dir() -> PathBuf {
    std::env::var_os("MATRA_MODEL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").expect("HOME"))
                .join(".matra")
                .join("models")
        })
}

fn load_fixtures() -> Vec<Fixture> {
    let dir = spec_dir();
    let mut fixtures: Vec<Fixture> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension()? != "json" {
                return None;
            }
            let raw = fs::read_to_string(&path).ok()?;
            Some(
                serde_json::from_str(&raw)
                    .unwrap_or_else(|e| panic!("malformed fixture {}: {e}", path.display())),
            )
        })
        .collect();
    fixtures.sort_by(|a, b| a.name.cmp(&b.name));
    assert!(
        !fixtures.is_empty(),
        "no fixtures found in {}",
        dir.display()
    );
    fixtures
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < TOLERANCE
}

#[test]
#[ignore = "requires the UDPipe model"]
fn rust_crust_conforms_to_spec() {
    let nlp = Udpipe::english(model_dir()).expect("load english model");
    let engine = Engine::new(Box::new(nlp), matra::standard_decomposers());

    for fixture in load_fixtures() {
        let format = match fixture.format.as_str() {
            "markdown" => Format::Markdown,
            "plain" => Format::PlainText,
            other => panic!("{}: unknown format {other}", fixture.name),
        };
        let doc: Document = engine
            .analyze_one(RawDocument::new(fixture.input.clone(), None, format))
            .unwrap_or_else(|e| panic!("{}: analyze failed: {e}", fixture.name))
            .analysis;

        let e = &fixture.expect;
        let name = &fixture.name;

        assert_eq!(
            doc.total_sentences(),
            e.total_sentences,
            "{name}: sentence count"
        );
        assert_eq!(doc.total_words(), e.total_words, "{name}: word count");
        assert_eq!(
            doc.paragraph_count(),
            e.paragraph_count,
            "{name}: paragraph count"
        );

        let sentences: Vec<_> = doc.sentences().collect();
        assert_eq!(
            sentences.len(),
            e.sentences.len(),
            "{name}: sentences returned"
        );

        for (i, (got, want)) in sentences.iter().zip(&e.sentences).enumerate() {
            assert_eq!(got.text, want.text, "{name}: sentence {i} text");
            if let Some(want_negs) = &want.negations {
                assert_eq!(
                    got.negations.len(),
                    want_negs.len(),
                    "{name}: sentence {i} negation count"
                );
                for (j, (n, w)) in got.negations.iter().zip(want_negs).enumerate() {
                    let at = format!("{name}: sentence {i} negation {j}");
                    assert_eq!(n.cue_id, w.cue_id, "{at} cue_id");
                    assert_eq!(n.cue_lemma, w.cue_lemma, "{at} cue_lemma");
                    assert_eq!(n.head_id, w.head_id, "{at} head_id");
                }
            }
            assert_eq!(
                got.tokens.len(),
                want.token_count,
                "{name}: sentence {i} token count"
            );
            for (j, (t, w)) in got.tokens.iter().zip(&want.tokens).enumerate() {
                let at = format!("{name}: sentence {i} token {j}");
                assert_eq!(t.id, w.id, "{at} id");
                assert_eq!(t.text, w.text, "{at} text");
                assert_eq!(t.lemma, w.lemma, "{at} lemma");
                assert_eq!(t.pos, w.pos, "{at} pos");
                assert_eq!(t.head, w.head, "{at} head");
                assert_eq!(t.dep, w.dep, "{at} dep");
            }
        }

        if let Some(want) = e.passive_ratio {
            let got = doc.passive_ratio.expect("passive_ratio present");
            assert!(close(got, want), "{name}: passive_ratio {got} != {want}");
        }
        if let Some(want) = e.vocabulary_ttr {
            let got = doc.vocabulary_ttr.expect("vocabulary_ttr present");
            assert!(close(got, want), "{name}: vocabulary_ttr {got} != {want}");
        }
        if let Some(want) = e.nominalization_ratio {
            let got = doc
                .nominalization_ratio
                .expect("nominalization_ratio present");
            assert!(
                close(got, want),
                "{name}: nominalization_ratio {got} != {want}"
            );
        }
    }
}
