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
