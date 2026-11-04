//! RAKE keyphrase extraction with POS filtering.
//!
//! Rapid Automatic Keyword Extraction. Splits text at stop words,
//! filters candidates to NOUN/ADJ runs (using POS tags from NLP provider),
//! scores by co-occurrence degree/frequency ratio.

use std::collections::HashMap;

use crate::domain::{Error, Keyphrase, Result, Sentence, Span};
use crate::stopwords::is_stop_word;

/// RAKE's cap, on tokens rather than sentences, and its cost model:
/// [`limits::MAX_RAKE_TOKENS`](crate::domain::limits::MAX_RAKE_TOKENS).
use crate::domain::limits::MAX_RAKE_TOKENS as MAX_TOKENS;

/// One candidate phrase where it occurs: its words (lowercased lemmas)
/// and the span from its first token's start to its last token's end.
struct Candidate {
    words: Vec<String>,
    span: Span,
}

/// At most `n` keyphrases ranked by RAKE with POS filtering, each with
/// every place it occurs.
///
/// Only NOUN, ADJ, and PROPN tokens contribute to candidate phrases.
/// Stop words and punctuation act as phrase boundaries. Scoring uses
/// the standard RAKE degree/frequency ratio from the co-occurrence matrix.
///
/// Returns [`Error::InputTooLarge`] when the total token count across all
/// input sentences exceeds the per-extractor `MAX_TOKENS` cap.
pub(super) fn keyphrases(sentences: &[&Sentence], n: usize) -> Result<Vec<Keyphrase>> {
    if n == 0 {
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

    for candidate in &candidates {
        let phrase_len = candidate.words.len();
        for word in &candidate.words {
            *freq.entry(word.as_str()).or_insert(0) += 1;
            // Degree = number of co-occurrences (including self).
            *degree.entry(word.as_str()).or_insert(0) += phrase_len;
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

    // Score each distinct phrase as the sum of its word scores, keeping
    // the phrases in the order they first occur and every occurrence.
    let mut phrases: Vec<Keyphrase> = Vec::new();
    let mut slot: HashMap<String, usize> = HashMap::new();
    for candidate in &candidates {
        let key = candidate.words.join(" ");
        let score: f64 = candidate
            .words
            .iter()
            .map(|w| word_score.get(w.as_str()).copied().unwrap_or(0.0))
            .sum();
        match slot.get(&key) {
            Some(&at) => {
                // Keep the highest score if the same phrase appears
                // multiple times.
                let phrase = &mut phrases[at];
                if score > phrase.score {
                    phrase.score = score;
                }
                phrase.occurrences.push(candidate.span);
            }
            None => {
                slot.insert(key.clone(), phrases.len());
                phrases.push(Keyphrase {
                    phrase: key,
                    score,
                    occurrences: vec![candidate.span],
                });
            }
        }
    }

    Ok(super::rank_phrases(phrases, n))
}

/// Extract candidate phrases: runs of NOUN/ADJ tokens between stop word
/// boundaries, in document order.
fn extract_candidates(sentences: &[&Sentence]) -> Vec<Candidate> {
    let mut candidates = Vec::new();

    for sentence in sentences {
        let mut words: Vec<String> = Vec::new();
        let mut span = Span::default();

        for token in &sentence.tokens {
            let lower = token.lemma.to_lowercase();
            let is_boundary = token.is_punct || is_stop_word(&lower);
            let is_content = token.pos == "NOUN" || token.pos == "ADJ" || token.pos == "PROPN";

            if is_boundary || !is_content {
                if !words.is_empty() {
                    candidates.push(Candidate {
                        words: std::mem::take(&mut words),
                        span,
                    });
                }
            } else {
                if words.is_empty() {
                    span.start = token.span.start;
                }
                span.end = token.span.end;
                words.push(lower);
            }
        }

        // Flush remaining candidate at sentence end.
        if !words.is_empty() {
            candidates.push(Candidate { words, span });
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

    fn run(sentences: &[Sentence], n: usize) -> Result<Vec<Keyphrase>> {
        keyphrases(&sentences.iter().collect::<Vec<_>>(), n)
    }

    /// Tokens of `text`, one per space-separated word with its span, all
    /// nouns except the stop word `of`.
    fn spanned(text: &str) -> Sentence {
        let mut at = 0;
        let tokens = text
            .split(' ')
            .enumerate()
            .map(|(i, word)| {
                let mut token = tok(i + 1, word, if word == "of" { "ADP" } else { "NOUN" });
                token.span = Span::new(at, at + word.len());
                at += word.len() + 1;
                token
            })
            .collect();
        sent(text, tokens)
    }

    /// Every occurrence of a phrase is reported, in document order, as
    /// the span from its first word to its last.
    #[test]
    fn a_phrase_reports_every_occurrence() {
        let text = "data pipeline of data pipeline";
        let result = run(&[spanned(text)], 5).unwrap();
        let found = result
            .iter()
            .find(|k| k.phrase == "data pipeline")
            .expect("the phrase is found");
        assert_eq!(found.occurrences, [Span::new(0, 13), Span::new(17, 30)]);
        for span in &found.occurrences {
            assert_eq!(&text[span.start..span.end], "data pipeline");
        }
    }

    /// Equal scores resolve in the order the phrases first occur, the
    /// same on every run.
    #[test]
    fn ties_keep_first_occurrence_order() {
        let text = "zebra of apple of mango";
        let phrases: Vec<String> = run(&[spanned(text)], 5)
            .unwrap()
            .into_iter()
            .map(|k| k.phrase)
            .collect();
        assert_eq!(phrases, ["zebra", "apple", "mango"]);
    }

    #[test]
    fn empty_input_returns_empty() {
        assert!(keyphrases(&[], 5).unwrap().is_empty());
    }

    #[test]
    fn max_zero_returns_empty() {
        let sentences = vec![sent("hello", vec![tok(1, "hello", "NOUN")])];
        assert!(run(&sentences, 0).unwrap().is_empty());
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

        let result = run(&sentences, 5).unwrap();
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

        let result = run(&sentences, 5).unwrap();
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
        match run(&sentences, 5) {
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
        let sentence = sent(
            "system is fast",
            vec![
                tok(1, "system", "NOUN"),
                tok(2, "is", "AUX"),
                tok(3, "fast", "ADJ"),
            ],
        );

        let candidates = extract_candidates(&[&sentence]);
        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn verbs_break_candidates() {
        let sentence = sent(
            "system processes data",
            vec![
                tok(1, "system", "NOUN"),
                tok(2, "processes", "VERB"),
                tok(3, "data", "NOUN"),
            ],
        );

        let candidates = extract_candidates(&[&sentence]);
        assert_eq!(candidates.len(), 2);
    }
}
