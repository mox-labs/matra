//! The outcome lane: `summarize`, `keyphrases` and `clusters` over a
//! document from a conformance fixture, `spec/tests/outcomes/outcomes.json`.
//!
//! The fixture carries a whole `Document`, produced once by the pinned
//! parser, so the outcome functions run over a real parse with no model
//! and no parser in the build: this lane runs under
//! `cargo test --no-default-features`, which is the build the outcome
//! functions promise to work in. Each case pins what one call returns, and
//! every result is validated against its published schema.
//!
//! `clusters` takes the fixture's own vectors, one per sentence, through
//! an embedder that returns them, so what is pinned is the clustering,
//! not an embedding model.
//!
//! The ignored test checks the committed document against the pinned
//! parser, and with `MATRA_REGENERATE_OUTCOMES=1` rewrites the fixture's
//! document and every case's expectation from the implementation instead.
//! A regenerated expectation is a snapshot until someone has read it.

mod document_schema;

use std::path::PathBuf;

use matra::domain::{
    Cluster, Document, Embedding, Format, Identity, Keyphrase, Parameters, SimilarityEdge, Span,
};
use matra::embed::Embedder;
use matra::{KeyphraseMethod, SummaryMethod};
use serde::{Deserialize, Serialize};

const TOLERANCE: f64 = 1e-6;

#[derive(Deserialize)]
struct Fixture {
    input: String,
    format: Format,
    document: Document,
    embedder: Identity,
    embeddings: Vec<Vec<f32>>,
    summaries: Vec<SummaryCase>,
    keyphrases: Vec<KeyphraseCase>,
    clusters: Vec<ClusterCase>,
}

#[derive(Deserialize, Serialize)]
struct SummaryCase {
    n: usize,
    method: String,
    expect: Vec<ExpectedSentence>,
}

#[derive(Deserialize, Serialize)]
struct ExpectedSentence {
    index: usize,
    span: Span,
    score: f64,
}

#[derive(Deserialize, Serialize)]
struct KeyphraseCase {
    n: usize,
    method: String,
    expect: Vec<Keyphrase>,
}

#[derive(Deserialize, Serialize)]
struct ClusterCase {
    threshold: f64,
    expect: ExpectedClusters,
}

#[derive(Deserialize, Serialize)]
struct ExpectedClusters {
    clusters: Vec<Cluster>,
    edges: Vec<SimilarityEdge>,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spec/tests/outcomes/outcomes.json")
}

fn fixture() -> Fixture {
    let path = fixture_path();
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("malformed fixture {}: {e}", path.display()))
}

fn summary_method(name: &str) -> SummaryMethod {
    match name {
        "tfidf" => SummaryMethod::TfIdf,
        "textrank" => SummaryMethod::TextRank,
        other => panic!("the fixture names a summary method matra lacks: {other}"),
    }
}

fn keyphrase_method(name: &str) -> KeyphraseMethod {
    match name {
        "rake" => KeyphraseMethod::Rake,
        "yake" => KeyphraseMethod::Yake,
        other => panic!("the fixture names a keyphrase method matra lacks: {other}"),
    }
}

/// Returns the fixture's vectors, one per sentence.
struct Vectors<'a> {
    vectors: &'a [Vec<f32>],
    identity: &'a Identity,
}

impl Embedder for Vectors<'_> {
    fn embed(&self, texts: &[&str]) -> matra::domain::Result<Vec<Embedding>> {
        assert_eq!(texts.len(), self.vectors.len(), "one vector per sentence");
        Ok(self.vectors.iter().cloned().map(Embedding).collect())
    }

    fn identity(&self) -> Identity {
        self.identity.clone()
    }
}

/// A derived result's provenance is the document's, plus the method and
/// the parameters it ran with.
fn assert_derived(
    got: &matra::domain::Provenance,
    doc: &Document,
    method: &str,
    parameters: &Parameters,
) {
    assert_eq!(got.method.as_deref(), Some(method));
    assert_eq!(got.parameters.as_ref(), Some(parameters));
    assert_eq!(got.matra, doc.provenance.matra);
    assert_eq!(got.schema, doc.provenance.schema);
    assert_eq!(got.parser, doc.provenance.parser);
    assert_eq!(got.format, doc.provenance.format);
    assert_eq!(got.source, doc.provenance.source);
}

#[test]
fn the_fixture_document_matches_the_document_schema() {
    let fixture = fixture();
    let raw: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fixture_path()).unwrap()).unwrap();
    document_schema::assert_valid("document", &raw["document"], "the fixture's document");
    assert_eq!(fixture.document.provenance.format, fixture.format);
    assert_eq!(
        fixture.document.provenance.source.bytes,
        fixture.input.len()
    );
    assert!(
        fixture.document.sentences().count() > 3,
        "the fixture is a document worth summarizing"
    );
}

#[test]
fn summaries_match_the_fixture() {
    let fixture = fixture();
    let doc = &fixture.document;
    assert!(!fixture.summaries.is_empty());
    for case in &fixture.summaries {
        let at = format!("summarize n={} {}", case.n, case.method);
        let got = matra::summarize_with(doc, case.n, summary_method(&case.method)).unwrap();
        assert_derived(
            &got.provenance,
            doc,
            &case.method,
            &serde_json::from_value(serde_json::json!({ "n": case.n })).unwrap(),
        );
        document_schema::assert_valid("summary", &serde_json::to_value(&got).unwrap(), &at);
        assert_eq!(got.sentences.len(), case.expect.len(), "{at}: count");
        for (g, e) in got.sentences.iter().zip(&case.expect) {
            assert_eq!(g.index, e.index, "{at}: index");
            assert_eq!(g.span, e.span, "{at}: span");
            assert!(
                (g.score - e.score).abs() < TOLERANCE,
                "{at}: score {}",
                g.score
            );
            let sentence = doc.sentences().nth(g.index).unwrap();
            assert_eq!(g.text, sentence.text, "{at}: text");
            assert_eq!(
                &fixture.input[g.span.byte_start..g.span.byte_end],
                sentence.text,
                "{at}: the span slices the input to the sentence"
            );
        }
    }
    // The default is the shipped one, recorded by name.
    let default = matra::summarize(doc, 2).unwrap();
    assert_eq!(default.provenance.method.as_deref(), Some("tfidf"));
}

#[test]
fn keyphrases_match_the_fixture() {
    let fixture = fixture();
    let doc = &fixture.document;
    assert!(!fixture.keyphrases.is_empty());
    for case in &fixture.keyphrases {
        let at = format!("keyphrases n={} {}", case.n, case.method);
        let got = matra::keyphrases_with(doc, case.n, keyphrase_method(&case.method)).unwrap();
        assert_derived(
            &got.provenance,
            doc,
            &case.method,
            &serde_json::from_value(serde_json::json!({ "n": case.n })).unwrap(),
        );
        document_schema::assert_valid("keyphrases", &serde_json::to_value(&got).unwrap(), &at);
        assert_eq!(got.phrases.len(), case.expect.len(), "{at}: count");
        for (g, e) in got.phrases.iter().zip(&case.expect) {
            assert_eq!(g.phrase, e.phrase, "{at}");
            assert!(
                (g.score - e.score).abs() < TOLERANCE,
                "{at}: {} score",
                g.phrase
            );
            assert_eq!(
                g.occurrences, e.occurrences,
                "{at}: {} occurrences",
                g.phrase
            );
        }
    }
    let default = matra::keyphrases(doc, 3).unwrap();
    assert_eq!(default.provenance.method.as_deref(), Some("rake"));
}

#[test]
fn clusters_match_the_fixture() {
    let fixture = fixture();
    let doc = &fixture.document;
    let embedder = Vectors {
        vectors: &fixture.embeddings,
        identity: &fixture.embedder,
    };
    assert!(!fixture.clusters.is_empty());
    for case in &fixture.clusters {
        let at = format!("clusters threshold={}", case.threshold);
        let got = matra::clusters(doc, &embedder, case.threshold).unwrap();
        assert_derived(
            &got.provenance,
            doc,
            "cosine",
            &serde_json::from_value(serde_json::json!({
                "threshold": case.threshold,
                "embedder": fixture.embedder,
            }))
            .unwrap(),
        );
        document_schema::assert_valid("clusters", &serde_json::to_value(&got).unwrap(), &at);
        assert_eq!(got.clusters, case.expect.clusters, "{at}: clusters");
        assert_eq!(got.edges.len(), case.expect.edges.len(), "{at}: edges");
        for (g, e) in got.edges.iter().zip(&case.expect.edges) {
            assert_eq!((g.a, g.b), (e.a, e.b), "{at}: edge");
            // The vectors are f32, so a score is compared in f32 space.
            assert_eq!(
                g.score as f32, e.score as f32,
                "{at}: edge ({}, {})",
                g.a, g.b
            );
        }
    }
}

/// The committed document is what the pinned parser produces from the
/// input, apart from the matra version that produced it. With
/// `MATRA_REGENERATE_OUTCOMES=1` the fixture is rewritten instead: the
/// document, and every case's expectation from the implementation.
#[cfg(feature = "udpipe")]
#[test]
#[ignore = "requires the UDPipe model"]
fn the_fixture_document_is_what_the_pinned_parser_produces() {
    let path = fixture_path();
    let raw: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    // Read the input alone, so a fixture whose document no longer
    // deserializes can still be regenerated.
    let input = raw["input"].as_str().unwrap().to_string();
    let format: Format = serde_json::from_value(raw["format"].clone()).unwrap();
    let engine = matra::Engine::new().expect("the UDPipe model loads");
    let fresh = engine.analyze(matra::Input::text(input, format)).unwrap();

    if std::env::var_os("MATRA_REGENERATE_OUTCOMES").is_some() {
        regenerate(&path, &raw, &fresh);
        return;
    }

    // The matra version moves with every release; the parse does not.
    let mut fresh_json = serde_json::to_value(&fresh).unwrap();
    fresh_json["provenance"]["matra"] = raw["document"]["provenance"]["matra"].clone();
    assert_eq!(
        fresh_json, raw["document"],
        "the pinned parser no longer produces the committed document; regenerate with \
         MATRA_REGENERATE_OUTCOMES=1 and read what changed"
    );
}

/// Rewrite the fixture at `path` around `doc`: the cases keep their
/// parameters and take what the implementation returns as their
/// expectation. The keys are written in a fixed order, the document last,
/// so a regeneration diffs only where a value moved.
#[cfg(feature = "udpipe")]
fn regenerate(path: &std::path::Path, raw: &serde_json::Value, doc: &Document) {
    #[derive(Serialize)]
    struct Written<'a> {
        name: &'a serde_json::Value,
        description: &'a serde_json::Value,
        input: &'a serde_json::Value,
        format: &'a serde_json::Value,
        embedder: &'a Identity,
        embeddings: &'a [Vec<f32>],
        summaries: Vec<SummaryCase>,
        keyphrases: Vec<KeyphraseCase>,
        clusters: Vec<ClusterCase>,
        document: &'a Document,
    }

    let cases = |key: &str| raw[key].as_array().unwrap().iter();
    let summaries = cases("summaries")
        .map(|case| {
            let n = case["n"].as_u64().unwrap() as usize;
            let method = case["method"].as_str().unwrap().to_string();
            let got = matra::summarize_with(doc, n, summary_method(&method)).unwrap();
            let expect = got
                .sentences
                .iter()
                .map(|s| ExpectedSentence {
                    index: s.index,
                    span: s.span,
                    score: s.score,
                })
                .collect();
            SummaryCase { n, method, expect }
        })
        .collect();
    let keyphrases = cases("keyphrases")
        .map(|case| {
            let n = case["n"].as_u64().unwrap() as usize;
            let method = case["method"].as_str().unwrap().to_string();
            let expect = matra::keyphrases_with(doc, n, keyphrase_method(&method))
                .unwrap()
                .phrases;
            KeyphraseCase { n, method, expect }
        })
        .collect();
    let embeddings: Vec<Vec<f32>> = serde_json::from_value(raw["embeddings"].clone()).unwrap();
    let embedder: Identity = serde_json::from_value(raw["embedder"].clone()).unwrap();
    let vectors = Vectors {
        vectors: &embeddings,
        identity: &embedder,
    };
    let clusters = cases("clusters")
        .map(|case| {
            let threshold = case["threshold"].as_f64().unwrap();
            let got = matra::clusters(doc, &vectors, threshold).unwrap();
            ClusterCase {
                threshold,
                expect: ExpectedClusters {
                    clusters: got.clusters,
                    edges: got.edges,
                },
            }
        })
        .collect();
    let written = Written {
        name: &raw["name"],
        description: &raw["description"],
        input: &raw["input"],
        format: &raw["format"],
        embedder: &embedder,
        embeddings: &embeddings,
        summaries,
        keyphrases,
        clusters,
        document: doc,
    };
    std::fs::write(path, serde_json::to_string_pretty(&written).unwrap() + "\n").unwrap();
}
