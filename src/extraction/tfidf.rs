//! TF-IDF extractive summarization.
//!
//! Scores each sentence by the mean TF-IDF of its lemmatized terms.
//! Each sentence is treated as a "document" for IDF computation.
//! Returns top-N sentences in document order.

use std::collections::HashMap;

use crate::domain::{Sentence, ScoredSentence};
use crate::stopwords::is_stop_word;

/// Extract top-N sentences by TF-IDF score, returned in document order.
pub fn summarize(sentences: &[Sentence], n: usize) -> Vec<ScoredSentence> {
    if sentences.is_empty() || n == 0 {
        return Vec::new();
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

            // Term frequency within this sentence.
            let mut tf: HashMap<&str, usize> = HashMap::new();
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

    scored
        .into_iter()
        .map(|(idx, score)| ScoredSentence {
            text: sentences[idx].text.clone(),
            score,
            position: idx,
        })
        .collect()
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
        Sentence { text: text.to_string(), tokens }
    }

    #[test]
    fn empty_input_returns_empty() {
        assert!(summarize(&[], 3).is_empty());
    }

    #[test]
    fn n_zero_returns_empty() {
        let sentences = vec![sent("hello world", vec![
            tok(1, "hello", "INTJ"),
            tok(2, "world", "NOUN"),
        ])];
        assert!(summarize(&sentences, 0).is_empty());
    }

    #[test]
    fn returns_in_document_order() {
        let sentences = vec![
            sent("Rust handles memory safely", vec![
                tok(1, "Rust", "PROPN"),
                tok(2, "handles", "VERB"),
                tok(3, "memory", "NOUN"),
                tok(4, "safely", "ADV"),
            ]),
            sent("Python is popular", vec![
                tok(1, "Python", "PROPN"),
                tok(2, "is", "AUX"),
                tok(3, "popular", "ADJ"),
            ]),
            sent("Rust memory safety prevents bugs", vec![
                tok(1, "Rust", "PROPN"),
                tok(2, "memory", "NOUN"),
                tok(3, "safety", "NOUN"),
                tok(4, "prevents", "VERB"),
                tok(5, "bugs", "NOUN"),
            ]),
        ];

        let result = summarize(&sentences, 2);
        assert_eq!(result.len(), 2);
        assert!(result[0].position < result[1].position);
    }

    #[test]
    fn stop_words_excluded_from_scoring() {
        let sentences = vec![
            sent("it is the", vec![
                tok(1, "it", "PRON"),
                tok(2, "is", "AUX"),
                tok(3, "the", "DET"),
            ]),
            sent("architecture determines quality", vec![
                tok(1, "architecture", "NOUN"),
                tok(2, "determines", "VERB"),
                tok(3, "quality", "NOUN"),
            ]),
        ];

        let result = summarize(&sentences, 1);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].position, 1);
    }

    #[test]
    fn n_greater_than_sentences_returns_all() {
        let sentences = vec![
            sent("one", vec![tok(1, "one", "NUM")]),
            sent("two", vec![tok(1, "two", "NUM")]),
        ];

        let result = summarize(&sentences, 10);
        assert_eq!(result.len(), 2);
    }
}
