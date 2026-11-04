//! TF-IDF extractive summarization.
//!
//! Scores each sentence by the mean TF-IDF of its lemmatized terms.
//! Each sentence is treated as a "document" for IDF computation.
//! Returns top-N sentences in document order.

use std::collections::{BTreeMap, HashMap};

use crate::domain::{Error, Result, Sentence};
use crate::stopwords::is_stop_word;

/// The cap, and why it is not TextRank's, is
/// [`limits::MAX_TFIDF_SENTENCES`](crate::domain::limits::MAX_TFIDF_SENTENCES).
use crate::domain::limits::MAX_TFIDF_SENTENCES as MAX_SENTENCES;

/// The top `n` sentences by TF-IDF score, as `(index, score)` pairs in
/// document order.
///
/// Returns [`Error::InputTooLarge`] when `sentences.len() > MAX_SENTENCES`.
pub(super) fn rank(sentences: &[&Sentence], n: usize) -> Result<Vec<(usize, f64)>> {
    if sentences.is_empty() || n == 0 {
        return Ok(Vec::new());
    }
    if sentences.len() > MAX_SENTENCES {
        return Err(Error::InputTooLarge {
            limit: MAX_SENTENCES,
            actual: sentences.len(),
            what: "tfidf",
        });
    }

    let total_sentences = sentences.len();

    // Collect lemmatized terms per sentence (lowercased, no punct, no stop words).
    let sentence_terms: Vec<Vec<&str>> = sentences
        .iter()
        .map(|sent| {
            sent.tokens
                .iter()
                .filter(|t| !t.is_punct && !is_stop_word(&t.lemma.to_lowercase()))
                .map(|t| t.lemma.as_str())
                .collect()
        })
        .collect();

    // Document frequency: how many sentences contain each term.
    let mut df: HashMap<&str, usize> = HashMap::new();
    for terms in &sentence_terms {
        let mut seen: HashMap<&str, bool> = HashMap::new();
        for &term in terms {
            if seen.insert(term, true).is_none() {
                *df.entry(term).or_insert(0) += 1;
            }
        }
    }

    // Score each sentence.
    let mut scored: Vec<(usize, f64)> = sentence_terms
        .iter()
        .enumerate()
        .map(|(idx, terms)| {
            if terms.is_empty() {
                return (idx, 0.0);
            }

            // Term frequency within this sentence. Ordered, because the
            // score is a float sum over it: summed in a hash map's order,
            // it moved in its last bit from one call to the next.
            let mut tf: BTreeMap<&str, usize> = BTreeMap::new();
            for &term in terms {
                *tf.entry(term).or_insert(0) += 1;
            }

            let len = terms.len() as f64;
            let score: f64 = tf
                .iter()
                .map(|(&term, &count)| {
                    let term_freq = count as f64 / len;
                    let doc_freq = df.get(term).copied().unwrap_or(1) as f64;
                    let idf = (total_sentences as f64 / doc_freq).ln();
                    term_freq * idf
                })
                .sum::<f64>()
                / tf.len() as f64;

            (idx, score)
        })
        .collect();

    // Select top-N by score.
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(n);

    // Re-sort by document position.
    scored.sort_by_key(|&(idx, _)| idx);

    Ok(scored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Token;

    fn tok(id: usize, text: &str, pos: &str) -> Token {
        Token {
            id,
            span: Default::default(),
            text: text.to_string(),
            lemma: text.to_lowercase(),
            pos: pos.to_string(),
            xpos: String::new(),
            features: std::collections::BTreeMap::new(),
            dep: String::new(),
            head: 0,
            deps: String::new(),
            misc: String::new(),
            is_punct: pos == "PUNCT",
        }
    }

    fn sent(text: &str, tokens: Vec<Token>) -> Sentence {
        Sentence::new(text.to_string(), tokens)
    }

    fn summarize(sentences: &[Sentence], n: usize) -> Result<Vec<(usize, f64)>> {
        rank(&sentences.iter().collect::<Vec<_>>(), n)
    }

    /// Regression: the score summed in a hash map's iteration order, which
    /// differs between maps, so the same document scored differently in
    /// its last bit from one call to the next (the parity lane found it).
    #[test]
    fn the_same_sentences_score_the_same_every_time() {
        // Many terms per sentence, some repeated, so the sum has enough
        // terms of different sizes for its order to reach the last bit.
        let sentences: Vec<Sentence> = (0..30)
            .map(|i| {
                let tokens = (0..25)
                    .map(|j| tok(j + 1, &format!("term{}", (i * 7 + j * j * 3) % 41), "NOUN"))
                    .collect();
                sent(&format!("sentence {i}"), tokens)
            })
            .collect();
        let first = summarize(&sentences, 30).unwrap();
        for _ in 0..64 {
            let again = summarize(&sentences, 30).unwrap();
            for (a, b) in first.iter().zip(&again) {
                assert_eq!(a.0, b.0);
                assert_eq!(a.1.to_bits(), b.1.to_bits(), "sentence {}", a.0);
            }
        }
    }

    #[test]
    fn empty_input_returns_empty() {
        assert!(summarize(&[], 3).unwrap().is_empty());
    }

    #[test]
    fn n_zero_returns_empty() {
        let sentences = vec![sent(
            "hello world",
            vec![tok(1, "hello", "INTJ"), tok(2, "world", "NOUN")],
        )];
        assert!(summarize(&sentences, 0).unwrap().is_empty());
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

        let result = summarize(&sentences, 2).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].0 < result[1].0);
    }

    #[test]
    fn stop_words_excluded_from_scoring() {
        let sentences = vec![
            sent(
                "it is the",
                vec![
                    tok(1, "it", "PRON"),
                    tok(2, "is", "AUX"),
                    tok(3, "the", "DET"),
                ],
            ),
            sent(
                "architecture determines quality",
                vec![
                    tok(1, "architecture", "NOUN"),
                    tok(2, "determines", "VERB"),
                    tok(3, "quality", "NOUN"),
                ],
            ),
        ];

        let result = summarize(&sentences, 1).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, 1);
    }

    #[test]
    fn n_greater_than_sentences_returns_all() {
        let sentences = vec![
            sent("one", vec![tok(1, "one", "NUM")]),
            sent("two", vec![tok(1, "two", "NUM")]),
        ];

        let result = summarize(&sentences, 10).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn rejects_oversized_input() {
        let sentence = sent("a", vec![tok(1, "a", "NOUN")]);
        let sentences: Vec<Sentence> = (0..MAX_SENTENCES + 1).map(|_| sentence.clone()).collect();
        match summarize(&sentences, 5) {
            Err(Error::InputTooLarge {
                limit,
                actual,
                what,
            }) => {
                assert_eq!(limit, MAX_SENTENCES);
                assert_eq!(actual, MAX_SENTENCES + 1);
                assert_eq!(what, "tfidf");
            }
            other => panic!("expected InputTooLarge, got {other:?}"),
        }
    }
}
