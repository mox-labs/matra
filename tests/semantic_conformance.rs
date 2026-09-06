//! The FFI shape fixture (i9 M5): `semantic_clusters` over the fixture's
//! hand-built vectors must produce exactly the serialized value in
//! `spec/tests/semantic/clusters.json`. No model is required, so this
//! runs everywhere. Comparison happens in the typed struct (f32 space),
//! because JSON shortest-repr floats and f32-to-f64 widening disagree in
//! the last bits while denoting the same f32.

use std::fs;
use std::path::PathBuf;

use matra::domain::{Embedding, SemanticClusters};

#[derive(serde::Deserialize)]
struct Fixture {
    name: String,
    embeddings: Vec<Vec<f32>>,
    threshold: f32,
    model_hash: String,
    expect: SemanticClusters,
}

#[test]
fn semantic_clusters_matches_the_shape_fixture() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spec/tests/semantic/clusters.json");
    let raw =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let fixture: Fixture = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("malformed fixture {}: {e}", path.display()));

    let embeddings: Vec<Embedding> = fixture.embeddings.into_iter().map(Embedding).collect();
    let got =
        matra::extraction::semantic_clusters(&embeddings, fixture.threshold, &fixture.model_hash)
            .unwrap();
    assert_eq!(got, fixture.expect, "fixture {}", fixture.name);
}

/// Reference-model conformance (i9 M6): exact-vector and exact-cluster
/// assertions against potion-base-8M, pinned by artifact digest in
/// `spec/tests/semantic/reference-model.json`. Ignored because the model
/// (~30 MB) is fetched on first run and the suite must not need a
/// network by default; run it with
/// `cargo test --features model2vec --test semantic_conformance -- --ignored`.
///
/// Since i10 M4 the model arrives through `Model2Vec::potion_base_8m`,
/// which downloads it into the resolved directory when it is absent and
/// verifies the same digest this fixture asserts. `MATRA_MODEL2VEC_DIR`
/// still names the directory when set; without it the directory is the
/// one `Config` resolves.
#[cfg(feature = "model2vec")]
mod reference_model {
    use std::fs;
    use std::path::PathBuf;

    use matra::domain::SemanticCluster;
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
        threshold: f32,
        clusters: Vec<SemanticCluster>,
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

        let texts: Vec<&str> = fixture.inputs.iter().map(String::as_str).collect();
        let embs = m.embed(&texts).expect("embed");
        let mut h = Sha256::new();
        for e in &embs {
            for v in &e.0 {
                h.update(v.to_le_bytes());
            }
        }
        assert_eq!(
            format!("{:x}", h.finalize()),
            fixture.vectors_sha256,
            "vector bytes drifted: the bit-determinism contract is broken"
        );

        for case in fixture.cases {
            let got = matra::extraction::semantic_clusters(&embs, case.threshold, m.model_hash())
                .expect("cluster");
            assert_eq!(got.clusters, case.clusters, "threshold {}", case.threshold);
        }
    }
}
