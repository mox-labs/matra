//! TextRank extractive summarization.
//!
//! Builds a similarity graph over sentences using TF overlap,
//! then runs iterative PageRank-style scoring. Returns top-N
//! sentences in document order.
//!
//! Memory is O(n^2) in sentence count due to the dense similarity
//! matrix. [`MAX_SENTENCES`] caps the input to keep worst-case memory
//! bounded; inputs above the cap return [`Error::InputTooLarge`].

use std::collections::HashMap;

use crate::domain::{Error, Result, ScoredSentence, Sentence};
use crate::stopwords::is_stop_word;

/// Maximum input size for [`textrank_summarize`]. At 2000 sentences the
/// similarity matrix is ~32 MB of f64; the ceiling for unattended use.
pub(crate) const MAX_SENTENCES: usize = 2000;

/// Extract top-N sentences by TextRank score, returned in document order.
///
/// Similarity between sentences is computed as the count of shared
/// content lemmas divided by the log of their lengths (to avoid
/// favoring long sentences).
///
/// Returns [`Error::InputTooLarge`] when `sentences.len() > MAX_SENTENCES`.
pub fn textrank_summarize(sentences: &[Sentence], n: usize) -> Result<Vec<ScoredSentence>> {
    if sentences.is_empty() || n == 0 {
        return Ok(Vec::new());
    }
    if sentences.len() > MAX_SENTENCES {
        return Err(Error::InputTooLarge {
            limit: MAX_SENTENCES,
            actual: sentences.len(),
            what: "textrank",
        });
    }

    let term_sets: Vec<HashMap<&str, usize>> = sentences
        .iter()
        .map(|s| {
            let mut counts = HashMap::new();
            for t in &s.tokens {
                if !t.is_punct {
                    let lower_lemma = t.lemma.as_str();
                    if !is_stop_word(&lower_lemma.to_lowercase()) {
                        *counts.entry(lower_lemma).or_insert(0) += 1;
                    }
                }
            }
            counts
        })
        .collect();

    let len = sentences.len();

    // Build similarity matrix.
    let mut similarity = vec![vec![0.0f64; len]; len];
    for i in 0..len {
        for j in (i + 1)..len {
            let sim = sentence_similarity(&term_sets[i], &term_sets[j]);
            similarity[i][j] = sim;
            similarity[j][i] = sim;
        }
    }

    // Run PageRank iteration.
    let damping = 0.85;
    let max_iter = 50;
    let convergence = 1e-6;
    let mut scores = vec![1.0 / len as f64; len];

    for _ in 0..max_iter {
        let mut new_scores = vec![0.0f64; len];
        let mut max_delta = 0.0f64;

        // Precompute row sums once per iteration: O(n^2) instead of O(n^3).
        let out_sums: Vec<f64> = (0..len).map(|j| similarity[j].iter().sum()).collect();

        for i in 0..len {
            let mut sum = 0.0;
            for j in 0..len {
                if i == j {
                    continue;
                }
                if out_sums[j] > 0.0 {
                    sum += similarity[j][i] / out_sums[j] * scores[j];
                }
            }
            new_scores[i] = (1.0 - damping) / len as f64 + damping * sum;
            max_delta = max_delta.max((new_scores[i] - scores[i]).abs());
        }

        scores = new_scores;
        if max_delta < convergence {
            break;
        }
    }

    // Select top-N by score, return in document order.
    let mut indexed: Vec<(usize, f64)> = scores.into_iter().enumerate().collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    indexed.truncate(n);
    indexed.sort_by_key(|&(idx, _)| idx);

    Ok(indexed
        .into_iter()
        .map(|(idx, score)| ScoredSentence {
            text: sentences[idx].text.clone(),
            score,
            position: idx,
        })
        .collect())
}

/// Similarity between two sentences: shared terms / log normalization.
fn sentence_similarity(a: &HashMap<&str, usize>, b: &HashMap<&str, usize>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let shared: usize = a
        .keys()
        .filter(|k| b.contains_key(*k))
        .map(|k| a[k].min(b[k]))
        .sum();

    if shared == 0 {
        return 0.0;
    }

    let norm = (a.len() as f64).ln_1p() + (b.len() as f64).ln_1p();
    if norm == 0.0 {
        return 0.0;
    }

    shared as f64 / norm
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Token;

    fn tok(id: usize, text: &str, pos: &str) -> Token {
        Token {
            id,
            text: text.to_string(),
            lemma: text.to_lowercase(),
            pos: pos.to_string(),
            xpos: String::new(),
            feats: String::new(),
            dep: String::new(),
            head: 0,
            deps: String::new(),
            misc: String::new(),
            is_punct: pos == "PUNCT",
        }
    }

    fn sent(text: &str, tokens: Vec<Token>) -> Sentence {
        Sentence {
            text: text.to_string(),
            tokens,
        }
    }

    #[test]
    fn empty_returns_empty() {
        assert!(textrank_summarize(&[], 3).unwrap().is_empty());
    }

    #[test]
    fn n_zero_returns_empty() {
        let sentences = vec![sent("hello", vec![tok(1, "hello", "INTJ")])];
        assert!(textrank_summarize(&sentences, 0).unwrap().is_empty());
    }

    #[test]
    fn rejects_oversized_input() {
        let toks = vec![tok(1, "word", "NOUN")];
        let sentences: Vec<Sentence> = (0..MAX_SENTENCES + 1)
            .map(|_| sent("word", toks.clone()))
            .collect();
        match textrank_summarize(&sentences, 3) {
            Err(crate::domain::Error::InputTooLarge {
                limit,
                actual,
                what,
            }) => {
                assert_eq!(limit, MAX_SENTENCES);
                assert_eq!(actual, MAX_SENTENCES + 1);
                assert_eq!(what, "textrank");
            }
            other => panic!("expected InputTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn returns_in_document_order() {
        let sentences = vec![
            sent(
                "Rust handles memory safely",
                vec![
                    tok(1, "Rust", "PROPN"),
                    tok(2, "handles", "VERB"),
                    tok(3, "memory", "NOUN"),
                    tok(4, "safely", "ADV"),
                ],
            ),
            sent(
                "Python is popular",
                vec![
                    tok(1, "Python", "PROPN"),
                    tok(2, "is", "AUX"),
                    tok(3, "popular", "ADJ"),
                ],
            ),
            sent(
                "Rust memory safety prevents bugs",
                vec![
                    tok(1, "Rust", "PROPN"),
                    tok(2, "memory", "NOUN"),
                    tok(3, "safety", "NOUN"),
                    tok(4, "prevents", "VERB"),
                    tok(5, "bugs", "NOUN"),
                ],
            ),
        ];

        let result = textrank_summarize(&sentences, 2).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].position < result[1].position);
    }

    #[test]
    fn similar_sentences_score_higher() {
        // Sentences sharing terms should get boosted by TextRank.
        let sentences = vec![
            sent(
                "machine learning algorithms",
                vec![
                    tok(1, "machine", "NOUN"),
                    tok(2, "learning", "NOUN"),
                    tok(3, "algorithms", "NOUN"),
                ],
            ),
            sent(
                "unrelated topic here",
                vec![
                    tok(1, "unrelated", "ADJ"),
                    tok(2, "topic", "NOUN"),
                    tok(3, "here", "ADV"),
                ],
            ),
            sent(
                "machine learning models",
                vec![
                    tok(1, "machine", "NOUN"),
                    tok(2, "learning", "NOUN"),
                    tok(3, "models", "NOUN"),
                ],
            ),
        ];

        let result = textrank_summarize(&sentences, 2).unwrap();
        // The two ML sentences should be selected (higher mutual reinforcement).
        let positions: Vec<usize> = result.iter().map(|s| s.position).collect();
        assert!(positions.contains(&0));
        assert!(positions.contains(&2));
    }

    #[test]
    fn single_sentence() {
        let sentences = vec![sent(
            "only one",
            vec![tok(1, "only", "ADV"), tok(2, "one", "NUM")],
        )];
        let result = textrank_summarize(&sentences, 5).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].position, 0);
    }
}
