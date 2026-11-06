//! The FFI shape fixture (EP-0009 M5): `matra::clusters` over a document
//! whose embedder returns the fixture's hand-built vectors must produce
//! exactly the clusters, edges and parameters in
//! `spec/tests/semantic/clusters.json`. No model is required, so this
//! runs everywhere, a build without default features included. Scores
//! are compared in f32 space, the precision the vectors carry: a JSON
//! shortest-repr float and an f32 widened to f64 denote the same f32
//! while differing as f64.

use std::fs;
use std::path::PathBuf;

use matra::domain::{
    Cluster, Document, Embedding, Format, Identity, Paragraph, ParagraphKind, Parameters,
    Provenance, ProvenanceSource, Section, Sentence, SimilarityEdge, Span,
};
use matra::embed::Embedder;

#[derive(serde::Deserialize)]
struct Fixture {
    name: String,
    embeddings: Vec<Vec<f32>>,
    threshold: f64,
    embedder: Identity,
    expect: Expect,
}

#[derive(serde::Deserialize)]
struct Expect {
    parameters: Parameters,
    clusters: Vec<Cluster>,
    edges: Vec<SimilarityEdge>,
}

/// Returns the vectors it was built with, one per sentence, whatever the
/// sentences say: the clustering, not an embedding model, is under test.
struct Vectors {
    vectors: Vec<Vec<f32>>,
    identity: Identity,
}

impl Embedder for Vectors {
    fn embed(&self, texts: &[&str]) -> matra::domain::Result<Vec<Embedding>> {
        assert_eq!(texts.len(), self.vectors.len(), "one vector per sentence");
        Ok(self.vectors.iter().cloned().map(Embedding).collect())
    }

    fn identity(&self) -> Identity {
        self.identity.clone()
    }
}

/// A document of one paragraph holding one sentence per text: what
/// `clusters` reads is the document's sentences, in order.
fn document_of(texts: &[String]) -> Document {
    let mut paragraph = Paragraph::new(String::new(), ParagraphKind::Paragraph, Span::default());
    paragraph.sentences = texts
        .iter()
        .map(|text| Sentence::new(text.clone(), Vec::new()))
        .collect();
    Document::new(
        Provenance::new(
            Identity::new("fixture", None, None, None),
            Format::PlainText,
            ProvenanceSource::new(None, 0, "0".repeat(64)),
        ),
        vec![Section::new(None, 0, None, vec![paragraph])],
        Vec::new(),
    )
}

/// Clusters and edges equal, with scores compared in f32 space.
fn assert_same(
    got: &matra::domain::Clusters,
    clusters: &[Cluster],
    edges: &[SimilarityEdge],
    at: &str,
) {
    assert_eq!(got.clusters, clusters, "{at}: clusters");
    assert_eq!(got.edges.len(), edges.len(), "{at}: edge count");
    for (g, e) in got.edges.iter().zip(edges) {
        assert_eq!((g.a, g.b), (e.a, e.b), "{at}: edge");
        assert_eq!(
            g.score as f32, e.score as f32,
            "{at}: edge ({}, {})",
            g.a, g.b
        );
    }
}

#[test]
fn clusters_matches_the_shape_fixture() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spec/tests/semantic/clusters.json");
    let raw =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let fixture: Fixture = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("malformed fixture {}: {e}", path.display()));

    let texts: Vec<String> = (0..fixture.embeddings.len())
        .map(|i| format!("Sentence {i}."))
        .collect();
    let doc = document_of(&texts);
    let embedder = Vectors {
        vectors: fixture.embeddings,
        identity: fixture.embedder,
    };
    let got = matra::clusters(&doc, &embedder, fixture.threshold).unwrap();

    assert_eq!(
        got.provenance.parameters.as_ref(),
        Some(&fixture.expect.parameters),
        "fixture {}: parameters",
        fixture.name
    );
    assert_eq!(got.provenance.source, doc.provenance.source);
    assert_same(
        &got,
        &fixture.expect.clusters,
        &fixture.expect.edges,
        &fixture.name,
    );
}

/// Reference-model conformance (EP-0009 M6): exact-vector and exact-cluster
/// assertions against potion-base-8M, pinned by artifact digest in
/// `spec/tests/semantic/reference-model.json`. Ignored because the model
/// (~30 MB) is fetched on first run and the suite must not need a
/// network by default; run it with
/// `cargo test --features model2vec --test semantic_conformance -- --ignored`.
///
/// Since EP-0010 M4 the model arrives through `Model2Vec::potion_base_8m`,
/// which downloads it into the resolved directory when it is absent and
/// verifies the same digest this fixture asserts. `MATRA_MODEL2VEC_DIR`
/// names a models directory when set, the model landing in its
/// `potion-base-8M` subdirectory; without it the directory is the one
/// `Config` resolves.
#[cfg(feature = "model2vec")]
mod reference_model {
    use std::fs;
    use std::path::PathBuf;

    use matra::domain::{Cluster, SimilarityEdge};
    use matra::embed::Embedder;
    use matra::embed::model2vec::Model2Vec;
    use sha2::{Digest, Sha256};

    #[derive(serde::Deserialize)]
    struct RefFixture {
        model: ModelPin,
        inputs: Vec<String>,
        vectors_sha256: String,
        cases: Vec<Case>,
    }
    #[derive(serde::Deserialize)]
    struct ModelPin {
        name: String,
        artifact_hash: String,
        dimensions: usize,
    }
    #[derive(serde::Deserialize)]
    struct Case {
        threshold: f64,
        clusters: Vec<Cluster>,
        edges: Vec<SimilarityEdge>,
    }

    /// The pinned model, from `MATRA_MODEL2VEC_DIR` when it is set and
    /// from the resolved configuration otherwise. Both go through the
    /// pinned-download constructor, so a first run provisions and every
    /// later one loads from disk.
    fn reference_model() -> Model2Vec {
        match std::env::var_os("MATRA_MODEL2VEC_DIR") {
            Some(dir) => Model2Vec::potion_base_8m(PathBuf::from(dir)).expect("model"),
            None => {
                let cfg = matra::config::Config::resolve().expect("config");
                Model2Vec::from_config(&cfg).expect("model")
            }
        }
    }

    #[test]
    #[ignore = "downloads the potion-base-8M model on first run"]
    fn reference_model_vectors_and_clusters_are_exact() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("spec/tests/semantic/reference-model.json");
        let fixture: RefFixture =
            serde_json::from_str(&fs::read_to_string(&path).expect("fixture")).expect("parse");

        let m = reference_model();
        assert_eq!(
            m.model_hash(),
            fixture.model.artifact_hash,
            "artifact digest mismatch: not the pinned {} release",
            fixture.model.name
        );
        assert_eq!(m.dimensions(), fixture.model.dimensions);
        assert_eq!(
            m.identity(),
            matra::domain::Identity::new(
                "model2vec",
                Some(fixture.model.name.clone()),
                Some(m.model_hash().to_string()),
                Some("MIT".to_string()),
            ),
            "the pinned digest names the model and its licence"
        );

        let texts: Vec<&str> = fixture.inputs.iter().map(String::as_str).collect();
        let embs = m.embed(&texts).expect("embed");
        let mut h = Sha256::new();
        for e in &embs {
            for v in &e.0 {
                h.update(v.to_le_bytes());
            }
        }
        // sha2 0.11 no longer formats a digest with `{:x}`; render the hex
        // by hand, lowercase as the fixture pins it.
        let vectors_sha256: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            vectors_sha256, fixture.vectors_sha256,
            "vector bytes drifted: the bit-determinism contract is broken"
        );

        // The same inputs as a document's sentences, through the outcome
        // function a caller uses: the model embeds them there.
        let doc = super::document_of(&fixture.inputs);
        for case in fixture.cases {
            let got = matra::clusters(&doc, &m, case.threshold).expect("cluster");
            assert_eq!(
                got.provenance
                    .parameters
                    .as_ref()
                    .and_then(|p| p.embedder.as_ref())
                    .and_then(|identity| identity.sha256.as_deref()),
                Some(m.model_hash())
            );
            super::assert_same(
                &got,
                &case.clusters,
                &case.edges,
                &format!("threshold {}", case.threshold),
            );
        }
    }
}
