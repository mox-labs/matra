//! RAKE keyphrase extraction with POS filtering.
//!
//! Rapid Automatic Keyword Extraction. Splits text at stop words,
//! filters candidates to NOUN/ADJ runs (using POS tags from NLP provider),
//! scores by co-occurrence degree/frequency ratio.

use std::collections::HashMap;

use crate::domain::{Error, Keyphrase, Result, Sentence};
use crate::stopwords::is_stop_word;

/// RAKE builds a co-occurrence map keyed on phrase strings. Worst-case
/// unique-phrase cardinality is bounded by total token count times mean
/// candidate-phrase length k (typically ~2–3 for NOUN/ADJ/PROPN runs).
/// At 200k tokens the candidate map holds <= ~50k entries at ~64 bytes
/// each = ~3 MiB resident; degree/frequency walks are O(unique tokens).
/// Cap is on tokens (not sentences) per Knuth's correction (chat-log
/// corpora can have 50k one-token sentences and still fit comfortably;
/// a sentence-cap defeats the actual cost model).
const MAX_TOKENS: usize = 200_000;

/// Extract ranked keyphrases using RAKE with POS filtering.
///
/// Only NOUN, ADJ, and PROPN tokens contribute to candidate phrases.
/// Stop words and punctuation act as phrase boundaries. Scoring uses
/// the standard RAKE degree/frequency ratio from the co-occurrence matrix.
///
/// Returns [`Error::InputTooLarge`] when the total token count across all
/// input sentences exceeds the per-extractor `MAX_TOKENS` cap.
pub fn keyphrases(sentences: &[Sentence], max_phrases: usize) -> Result<Vec<Keyphrase>> {
    if max_phrases == 0 {
        return Ok(Vec::new());
    }
    let total_tokens: usize = sentences.iter().map(|s| s.tokens.len()).sum();
    if total_tokens > MAX_TOKENS {
        return Err(Error::InputTooLarge {
            limit: MAX_TOKENS,
            actual: total_tokens,
            what: "rake",
        });
    }

    let candidates = extract_candidates(sentences);
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    // Build word co-occurrence matrix from candidate phrases.
    let mut freq: HashMap<&str, usize> = HashMap::new();
    let mut degree: HashMap<&str, usize> = HashMap::new();

    for phrase in &candidates {
        let words: Vec<&str> = phrase.iter().map(|s| s.as_str()).collect();
        let phrase_len = words.len();
        for &word in &words {
            *freq.entry(word).or_insert(0) += 1;
            // Degree = number of co-occurrences (including self).
            *degree.entry(word).or_insert(0) += phrase_len;
        }
    }

    // Score each word: degree / frequency.
    let word_score: HashMap<&str, f64> = freq
        .keys()
        .map(|&word| {
            let d = degree.get(word).copied().unwrap_or(0) as f64;
            let f = freq.get(word).copied().unwrap_or(1) as f64;
            (word, d / f)
        })
        .collect();

    // Score each candidate phrase: sum of word scores.
    let mut phrase_scores: HashMap<String, f64> = HashMap::new();
    for phrase in &candidates {
        let key = phrase.join(" ");
        let score: f64 = phrase
            .iter()
            .map(|w| word_score.get(w.as_str()).copied().unwrap_or(0.0))
            .sum();
        // Keep the highest score if the same phrase appears multiple times.
        let entry = phrase_scores.entry(key).or_insert(0.0);
        if score > *entry {
            *entry = score;
        }
    }

    // Sort by score descending, take top N.
    let mut ranked: Vec<Keyphrase> = phrase_scores
        .into_iter()
        .map(|(phrase, score)| Keyphrase { phrase, score })
        .collect();
    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked.truncate(max_phrases);
    Ok(ranked)
}

/// Extract candidate phrases: runs of NOUN/ADJ tokens between stop word boundaries.
fn extract_candidates(sentences: &[Sentence]) -> Vec<Vec<String>> {
    let mut candidates = Vec::new();

    for sentence in sentences {
        let mut current: Vec<String> = Vec::new();

        for token in &sentence.tokens {
            let lower = token.lemma.to_lowercase();
            let is_boundary = token.is_punct || is_stop_word(&lower);
            let is_content = token.pos == "NOUN" || token.pos == "ADJ" || token.pos == "PROPN";

            if is_boundary || !is_content {
                if !current.is_empty() {
                    candidates.push(std::mem::take(&mut current));
                }
            } else {
                current.push(lower);
            }
        }

        // Flush remaining candidate at sentence end.
        if !current.is_empty() {
            candidates.push(current);
        }
    }

    candidates
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
    fn empty_input_returns_empty() {
        assert!(keyphrases(&[], 5).unwrap().is_empty());
    }

    #[test]
    fn max_zero_returns_empty() {
        let sentences = vec![sent("hello", vec![tok(1, "hello", "NOUN")])];
        assert!(keyphrases(&sentences, 0).unwrap().is_empty());
    }

    #[test]
    fn extracts_noun_adj_phrases() {
        let sentences = vec![sent(
            "The quick brown fox jumps over the lazy dog",
            vec![
                tok(1, "The", "DET"),
                tok(2, "quick", "ADJ"),
                tok(3, "brown", "ADJ"),
                tok(4, "fox", "NOUN"),
                tok(5, "jumps", "VERB"),
                tok(6, "over", "ADP"),
                tok(7, "the", "DET"),
                tok(8, "lazy", "ADJ"),
                tok(9, "dog", "NOUN"),
            ],
        )];

        let result = keyphrases(&sentences, 5).unwrap();
        assert!(!result.is_empty());

        let phrases: Vec<&str> = result.iter().map(|k| k.phrase.as_str()).collect();
        assert!(phrases.contains(&"quick brown fox"));
        assert!(phrases.contains(&"lazy dog"));
    }

    #[test]
    fn multi_word_phrases_score_higher() {
        let sentences = vec![sent(
            "machine learning algorithms process natural language",
            vec![
                tok(1, "machine", "NOUN"),
                tok(2, "learning", "NOUN"),
                tok(3, "algorithms", "NOUN"),
                tok(4, "process", "VERB"),
                tok(5, "natural", "ADJ"),
                tok(6, "language", "NOUN"),
            ],
        )];

        let result = keyphrases(&sentences, 5).unwrap();
        assert!(result.len() >= 2);
        assert!(result[0].score >= result[1].score);
    }

    #[test]
    fn rejects_oversized_input() {
        // One sentence with MAX_TOKENS + 1 tokens (cheap to construct, exceeds cap).
        let tokens: Vec<Token> = (0..MAX_TOKENS + 1)
            .map(|i| tok(i + 1, "x", "NOUN"))
            .collect();
        let sentences = vec![sent("x".repeat(MAX_TOKENS + 1).as_str(), tokens)];
        match keyphrases(&sentences, 5) {
            Err(Error::InputTooLarge {
                limit,
                actual,
                what,
            }) => {
                assert_eq!(limit, MAX_TOKENS);
                assert_eq!(actual, MAX_TOKENS + 1);
                assert_eq!(what, "rake");
            }
            other => panic!("expected InputTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn stop_words_split_candidates() {
        let sentences = vec![sent(
            "system is fast",
            vec![
                tok(1, "system", "NOUN"),
                tok(2, "is", "AUX"),
                tok(3, "fast", "ADJ"),
            ],
        )];

        let candidates = extract_candidates(&sentences);
        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn verbs_break_candidates() {
        let sentences = vec![sent(
            "system processes data",
            vec![
                tok(1, "system", "NOUN"),
                tok(2, "processes", "VERB"),
                tok(3, "data", "NOUN"),
            ],
        )];

        let candidates = extract_candidates(&sentences);
        assert_eq!(candidates.len(), 2);
    }
}
