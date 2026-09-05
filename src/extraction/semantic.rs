//! Semantic-equivalence clustering over precomputed sentence embeddings.
//!
//! A pure function over domain values (rule 5): it never calls the embed
//! port. Whoever holds both the `Document` and an `Embedder` runs one,
//! then the other; the composition root owns that pairing. Everything
//! here is a total computation over the vectors it is handed.

use crate::domain::{
    Embedding, Error, Result, SemanticCluster, SemanticClusters, SemanticEdge, Sentence,
};

/// Cap on sentence count: the similarity matrix is O(n^2), same bound and
/// reason as TextRank's.
const MAX_SENTENCES: usize = 2000;

/// Cluster sentences whose embedding cosine similarity clears `threshold`.
///
/// Clusters are connected components of the above-threshold graph, with
/// the clearing edges reported alongside the members (see
/// [`SemanticClusters`] for what co-membership does and does not mean).
/// `model_hash` is the identity of the model that produced `embeddings`,
/// as the embed adapter reports it; it travels into the result so the
/// scores stay attributable to a geometry.
///
/// A zero-magnitude embedding (an empty sentence embeds to zero) has no
/// defined cosine with anything, so it gets no edges and lands in no
/// cluster: no claim, rather than a fabricated zero score.
///
/// # Errors
///
/// [`Error::InvalidInput`] when `sentences` and `embeddings` differ in
/// length, when the embeddings disagree on dimension, or when
/// `threshold` is not finite. [`Error::InputTooLarge`] (what =
/// `"semantic_clusters"`) past the O(n^2) cap.
pub fn semantic_clusters(
    sentences: &[Sentence],
    embeddings: &[Embedding],
    threshold: f32,
    model_hash: &str,
) -> Result<SemanticClusters> {
    if sentences.len() != embeddings.len() {
        return Err(Error::InvalidInput(format!(
            "sentences ({}) and embeddings ({}) must have equal length",
            sentences.len(),
            embeddings.len()
        )));
    }
    if !threshold.is_finite() {
        return Err(Error::InvalidInput(format!(
            "threshold must be finite, got {threshold}"
        )));
    }
    if sentences.len() > MAX_SENTENCES {
        return Err(Error::InputTooLarge {
            limit: MAX_SENTENCES,
            actual: sentences.len(),
            what: "semantic_clusters",
        });
    }
    let dim = embeddings.first().map_or(0, |e| e.0.len());
    if let Some((i, e)) = embeddings
        .iter()
        .enumerate()
        .find(|(_, e)| e.0.len() != dim)
    {
        return Err(Error::InvalidInput(format!(
            "embedding {i} has dimension {}, expected {dim}",
            e.0.len()
        )));
    }

    let n = embeddings.len();
    let norms: Vec<f32> = embeddings
        .iter()
        .map(|e| e.0.iter().map(|v| v * v).sum::<f32>().sqrt())
        .collect();

    // Above-threshold edges, and union-find over them. The visited
    // structure is the component id table, so the walk is cycle-safe by
    // construction rather than by depth ceiling.
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    let mut edges: Vec<SemanticEdge> = Vec::new();
    for a in 0..n {
        if norms[a] == 0.0 {
            continue;
        }
        for b in (a + 1)..n {
            if norms[b] == 0.0 {
                continue;
            }
            let dot: f32 = embeddings[a]
                .0
                .iter()
                .zip(&embeddings[b].0)
                .map(|(x, y)| x * y)
                .sum();
            let score = dot / (norms[a] * norms[b]);
            if score >= threshold {
                edges.push(SemanticEdge { a, b, score });
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                if ra != rb {
                    parent[rb] = ra;
                }
            }
        }
    }

    // Group members by component root; only sentences with an edge are
    // in a component of size > 1, so singletons fall out naturally.
    let mut clusters: Vec<SemanticCluster> = Vec::new();
    let mut root_to_cluster: Vec<Option<usize>> = vec![None; n];
    for i in 0..n {
        let root = find(&mut parent, i);
        if root == i {
            continue; // handled when a member maps to it, if any edge exists
        }
        let slot = match root_to_cluster[root] {
            Some(s) => s,
            None => {
                clusters.push(SemanticCluster {
                    members: vec![root],
                    edges: Vec::new(),
                });
                root_to_cluster[root] = Some(clusters.len() - 1);
                clusters.len() - 1
            }
        };
        clusters[slot].members.push(i);
    }
    for edge in edges {
        let root = find(&mut parent, edge.a);
        if let Some(slot) = root_to_cluster[root] {
            clusters[slot].edges.push(edge);
        }
    }
    for cluster in &mut clusters {
        cluster.members.sort_unstable();
        cluster.edges.sort_unstable_by_key(|e| (e.a, e.b));
    }
    clusters.sort_unstable_by_key(|c| c.members[0]);

    Ok(SemanticClusters {
        model_hash: model_hash.to_string(),
        threshold,
        clusters,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sentences(n: usize) -> Vec<Sentence> {
        // Structure is irrelevant to clustering; only the count is used.
        (0..n)
            .map(|i| Sentence::new(format!("s{i}"), Vec::new()))
            .collect()
    }

    fn e(v: &[f32]) -> Embedding {
        Embedding(v.to_vec())
    }

    #[test]
    fn chained_similarity_is_one_component_without_the_far_edge() {
        // a ~ b and b ~ c clear 0.9; a ~ c does not. One cluster of
        // three, exactly two edges, the far pair absent.
        let f = std::f32::consts::FRAC_1_SQRT_2;
        let emb = [
            e(&[1.0, 0.0]),
            e(&[0.9239, 0.3827]), // ~cos 22.5 deg from a
            e(&[f, f]),           // cos 45 deg from a, ~22.5 from b
        ];
        let out = semantic_clusters(&sentences(3), &emb, 0.92, "h").unwrap();
        assert_eq!(out.clusters.len(), 1);
        let c = &out.clusters[0];
        assert_eq!(c.members, vec![0, 1, 2]);
        let pairs: Vec<(usize, usize)> = c.edges.iter().map(|e| (e.a, e.b)).collect();
        assert_eq!(pairs, vec![(0, 1), (1, 2)]);
    }

    #[test]
    fn postconditions_hold() {
        let emb = [
            e(&[1.0, 0.0]),
            e(&[1.0, 0.0]),
            e(&[0.0, 1.0]),
            e(&[0.0, 1.0]),
            e(&[-1.0, 0.0]),
        ];
        let out = semantic_clusters(&sentences(5), &emb, 0.99, "h").unwrap();
        // Two clusters; index 4 is a singleton and appears nowhere.
        assert_eq!(out.clusters.len(), 2);
        let mut seen = std::collections::HashSet::new();
        for c in &out.clusters {
            for &m in &c.members {
                assert!(seen.insert(m), "member {m} in two clusters");
            }
            for edge in &c.edges {
                assert!(edge.score >= out.threshold);
                assert!(edge.a < edge.b);
                assert!(c.members.contains(&edge.a) && c.members.contains(&edge.b));
            }
            // membership = transitive closure of the reported edges:
            // every member touches at least one edge.
            for &m in &c.members {
                assert!(
                    c.edges.iter().any(|e| e.a == m || e.b == m),
                    "member {m} has no edge"
                );
            }
        }
        assert!(!seen.contains(&4), "singleton clustered");
    }

    #[test]
    fn empty_input_yields_empty_clusters_not_an_error() {
        let out = semantic_clusters(&[], &[], 0.8, "h").unwrap();
        assert!(out.clusters.is_empty());
        assert_eq!(out.model_hash, "h");
    }

    #[test]
    fn zero_norm_embeddings_get_no_edges_even_at_threshold_zero() {
        let emb = [e(&[0.0, 0.0]), e(&[0.0, 0.0]), e(&[1.0, 0.0])];
        let out = semantic_clusters(&sentences(3), &emb, 0.0, "h").unwrap();
        assert!(out.clusters.is_empty());
    }

    #[test]
    fn length_mismatch_is_invalid_input() {
        let emb = [e(&[1.0, 0.0])];
        match semantic_clusters(&sentences(2), &emb, 0.8, "h") {
            Err(Error::InvalidInput(msg)) => assert!(msg.contains("equal length")),
            other => panic!("expected InvalidInput, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn dimension_disagreement_is_invalid_input() {
        let emb = [e(&[1.0, 0.0]), e(&[1.0, 0.0, 0.0])];
        match semantic_clusters(&sentences(2), &emb, 0.8, "h") {
            Err(Error::InvalidInput(msg)) => assert!(msg.contains("dimension")),
            other => panic!("expected InvalidInput, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn non_finite_threshold_is_invalid_input() {
        match semantic_clusters(&[], &[], f32::NAN, "h") {
            Err(Error::InvalidInput(msg)) => assert!(msg.contains("finite")),
            other => panic!("expected InvalidInput, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn sentence_cap_is_enforced_with_its_own_label() {
        let n = MAX_SENTENCES + 1;
        let emb: Vec<Embedding> = (0..n).map(|_| e(&[1.0])).collect();
        match semantic_clusters(&sentences(n), &emb, 0.9, "h") {
            Err(Error::InputTooLarge { what, .. }) => assert_eq!(what, "semantic_clusters"),
            other => panic!("expected InputTooLarge, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn output_ordering_is_deterministic() {
        let emb = [
            e(&[0.0, 1.0]),
            e(&[1.0, 0.0]),
            e(&[0.0, 1.0]),
            e(&[1.0, 0.0]),
        ];
        let out = semantic_clusters(&sentences(4), &emb, 0.99, "h").unwrap();
        assert_eq!(out.clusters[0].members, vec![0, 2]);
        assert_eq!(out.clusters[1].members, vec![1, 3]);
    }
}
