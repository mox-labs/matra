//! Semantic-equivalence clustering over precomputed sentence embeddings.
//!
//! A pure function over domain values (rule 5): it never calls the embed
//! port. The composition root's `matra::clusters` embeds a document's
//! sentences, then calls [`cluster`] here; everything here is a total
//! computation over the vectors it is handed.

use crate::domain::{Cluster, Embedding, Error, Result, SimilarityEdge};

// The O(n^2) cap: literally TextRank's bound, not a copy of it, so the
// two cannot drift apart.
use super::textrank::MAX_SENTENCES;

/// Cluster texts whose embedding cosine similarity is at least
/// `threshold`, returning the clusters and every edge that cleared it.
///
/// Indices in the result are positions in `embeddings`; the caller knows
/// what each vector stands for (`matra::clusters` pairs them with a
/// document's sentences and validates the correspondence before calling
/// here).
///
/// Clusters are connected components of the above-threshold graph (see
/// [`crate::domain::Clusters`] for what co-membership does and does not
/// mean), ordered by their smallest member; members are ascending, and
/// edges are ordered by `a`, then `b`. Similarity is computed in the
/// vectors' own `f32` and widened to `f64` for the comparison and the
/// result.
///
/// A zero-magnitude embedding (an empty sentence embeds to zero) has no
/// defined cosine with anything, so it gets no edges and lands in no
/// cluster: no claim, rather than a fabricated zero score.
///
/// # Errors
///
/// [`Error::InvalidInput`] when the embeddings disagree on dimension,
/// contain a non-finite value, or when `threshold` is not finite.
/// [`Error::InputTooLarge`] (what = `"clusters"`) past the O(n^2) cap.
pub(crate) fn cluster(
    embeddings: &[Embedding],
    threshold: f64,
) -> Result<(Vec<Cluster>, Vec<SimilarityEdge>)> {
    if !threshold.is_finite() {
        return Err(Error::InvalidInput(format!(
            "threshold must be finite, got {threshold}"
        )));
    }
    if embeddings.len() > MAX_SENTENCES {
        return Err(Error::InputTooLarge {
            limit: MAX_SENTENCES,
            actual: embeddings.len(),
            what: "clusters",
        });
    }
    let dim = embeddings.first().map_or(0, |e| e.0.len());
    for (i, e) in embeddings.iter().enumerate() {
        if e.0.len() != dim {
            return Err(Error::InvalidInput(format!(
                "embedding {i} has dimension {}, expected {dim}",
                e.0.len()
            )));
        }
        // A zero vector is a real geometry (documented above); a NaN or
        // infinity is not a geometry at all, and letting it through
        // would silently park its sentence in the unclustered count.
        if !e.0.iter().all(|v| v.is_finite()) {
            return Err(Error::InvalidInput(format!(
                "embedding {i} contains a non-finite value"
            )));
        }
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
    // Generated in (a, b) order, which is the order the result carries.
    let mut edges: Vec<SimilarityEdge> = Vec::new();
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
            let score = f64::from(dot / (norms[a] * norms[b]));
            if score >= threshold {
                edges.push(SimilarityEdge::new(a, b, score));
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                if ra != rb {
                    parent[rb] = ra;
                }
            }
        }
    }

    // Group members by component root; only sentences with an edge are
    // in a component of size > 1, so singletons fall out naturally.
    let mut members: Vec<Vec<usize>> = Vec::new();
    let mut root_to_cluster: Vec<Option<usize>> = vec![None; n];
    for i in 0..n {
        let root = find(&mut parent, i);
        if root == i {
            continue; // handled when a member maps to it, if any edge exists
        }
        let slot = match root_to_cluster[root] {
            Some(s) => s,
            None => {
                members.push(vec![root]);
                root_to_cluster[root] = Some(members.len() - 1);
                members.len() - 1
            }
        };
        members[slot].push(i);
    }
    for cluster in &mut members {
        cluster.sort_unstable();
    }
    members.sort_unstable_by_key(|m| m[0]);

    Ok((members.into_iter().map(Cluster::new).collect(), edges))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(v: &[f32]) -> Embedding {
        Embedding(v.to_vec())
    }

    #[test]
    fn chained_similarity_is_one_component_without_the_far_edge() {
        // a ~ b and b ~ c clear 0.92; a ~ c does not. One cluster of
        // three, exactly two edges, the far pair absent.
        let f = std::f32::consts::FRAC_1_SQRT_2;
        let emb = [
            e(&[1.0, 0.0]),
            e(&[0.9239, 0.3827]), // ~cos 22.5 deg from a
            e(&[f, f]),           // cos 45 deg from a, ~22.5 from b
        ];
        let (clusters, edges) = cluster(&emb, 0.92).unwrap();
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].members, vec![0, 1, 2]);
        let pairs: Vec<(usize, usize)> = edges.iter().map(|e| (e.a, e.b)).collect();
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
        let threshold = 0.99;
        let (clusters, edges) = cluster(&emb, threshold).unwrap();
        // Two clusters; index 4 is a singleton and appears nowhere.
        assert_eq!(clusters.len(), 2);
        let mut seen = std::collections::HashSet::new();
        for c in &clusters {
            for &m in &c.members {
                assert!(seen.insert(m), "member {m} in two clusters");
            }
            let own: Vec<&SimilarityEdge> =
                edges.iter().filter(|e| c.members.contains(&e.a)).collect();
            for edge in &own {
                assert!(edge.score >= threshold);
                assert!(edge.a < edge.b);
                assert!(c.members.contains(&edge.b), "an edge leaves its cluster");
            }
            // membership = transitive closure of the reported edges:
            // every member is REACHABLE from members[0] over the
            // cluster's own edges, not merely incident to some edge.
            let mut reach = std::collections::HashSet::from([c.members[0]]);
            let mut grew = true;
            while grew {
                grew = false;
                for edge in &own {
                    if reach.contains(&edge.a) && reach.insert(edge.b) {
                        grew = true;
                    }
                    if reach.contains(&edge.b) && reach.insert(edge.a) {
                        grew = true;
                    }
                }
            }
            for &m in &c.members {
                assert!(
                    reach.contains(&m),
                    "member {m} unreachable over reported edges"
                );
            }
        }
        assert!(!seen.contains(&4), "singleton clustered");
        let order: Vec<(usize, usize)> = edges.iter().map(|e| (e.a, e.b)).collect();
        let mut sorted = order.clone();
        sorted.sort_unstable();
        assert_eq!(order, sorted, "edges are ordered by a, then b");
    }

    #[test]
    fn empty_input_yields_empty_clusters_not_an_error() {
        let (clusters, edges) = cluster(&[], 0.8).unwrap();
        assert!(clusters.is_empty());
        assert!(edges.is_empty());
    }

    #[test]
    fn zero_norm_embeddings_get_no_edges_even_at_threshold_zero() {
        let emb = [e(&[0.0, 0.0]), e(&[0.0, 0.0]), e(&[1.0, 0.0])];
        let (clusters, edges) = cluster(&emb, 0.0).unwrap();
        assert!(clusters.is_empty());
        assert!(edges.is_empty());
    }

    #[test]
    fn dimension_disagreement_is_invalid_input() {
        let emb = [e(&[1.0, 0.0]), e(&[1.0, 0.0, 0.0])];
        match cluster(&emb, 0.8) {
            Err(Error::InvalidInput(msg)) => assert!(msg.contains("dimension")),
            other => panic!("expected InvalidInput, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn non_finite_embedding_value_is_invalid_input() {
        let emb = [e(&[1.0, 0.0]), e(&[f32::NAN, 0.0])];
        match cluster(&emb, 0.8) {
            Err(Error::InvalidInput(msg)) => assert!(msg.contains("non-finite")),
            other => panic!("expected InvalidInput, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn non_finite_threshold_is_invalid_input() {
        match cluster(&[], f64::NAN) {
            Err(Error::InvalidInput(msg)) => assert!(msg.contains("finite")),
            other => panic!("expected InvalidInput, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn sentence_cap_is_enforced_with_its_own_label() {
        let n = MAX_SENTENCES + 1;
        let emb: Vec<Embedding> = (0..n).map(|_| e(&[1.0])).collect();
        match cluster(&emb, 0.9) {
            Err(Error::InputTooLarge { what, .. }) => assert_eq!(what, "clusters"),
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
        let (clusters, _) = cluster(&emb, 0.99).unwrap();
        assert_eq!(clusters[0].members, vec![0, 2]);
        assert_eq!(clusters[1].members, vec![1, 3]);
    }
}
