//! YAKE keyphrase extraction.
//!
//! Yet Another Keyword Extractor. Unsupervised statistical approach that
//! scores individual words by positional, frequency, and context features,
//! then builds n-gram candidates. Lower YAKE score = more relevant.
//! Output is normalized so higher score = more relevant (inverted).

use std::collections::HashMap;

use crate::domain::{Keyphrase, Sentence};
use crate::stopwords::is_stop_word;

/// Extract ranked keyphrases using YAKE.
///
/// Scores individual terms by position, frequency, and context diversity,
/// then combines into n-gram candidates (1-3 words). Returns top phrases
/// sorted by relevance (highest score first).
pub fn yake_keyphrases(sentences: &[Sentence], max: usize) -> Vec<Keyphrase> {
    if sentences.is_empty() || max == 0 {
        return Vec::new();
    }

    // Collect all content terms with position info.
    let mut term_positions: HashMap<String, Vec<usize>> = HashMap::new();
    let mut term_contexts: HashMap<String, Vec<String>> = HashMap::new();
    let mut position = 0;

    for sent in sentences {
        let content: Vec<&str> = sent
            .tokens
            .iter()
            .filter(|t| !t.is_punct)
            .map(|t| t.lemma.as_str())
            .collect();

        for (i, &lemma) in content.iter().enumerate() {
            let lower = lemma.to_lowercase();
            if is_stop_word(&lower) || lower.len() <= 1 {
                position += 1;
                continue;
            }

            term_positions
                .entry(lower.clone())
                .or_default()
                .push(position);

            // Context: adjacent terms.
            if i > 0 {
                let prev = content[i - 1].to_lowercase();
                if !is_stop_word(&prev) {
                    term_contexts.entry(lower.clone()).or_default().push(prev);
                }
            }
            if i + 1 < content.len() {
                let next = content[i + 1].to_lowercase();
                if !is_stop_word(&next) {
                    term_contexts.entry(lower).or_default().push(next);
                }
            }

            position += 1;
        }
    }

    if term_positions.is_empty() {
        return Vec::new();
    }

    let total_positions = position.max(1) as f64;

    // Score individual terms (lower = more relevant in YAKE).
    let term_scores: HashMap<&str, f64> = term_positions
        .iter()
        .map(|(term, positions)| {
            let tf = positions.len() as f64;

            // Positional feature: terms appearing earlier are weighted higher.
            let pos_mean = positions.iter().sum::<usize>() as f64 / tf;
            let pos_score = (pos_mean / total_positions).ln_1p();

            // Frequency feature.
            let freq_score = tf / total_positions;

            // Context diversity: number of unique neighbors.
            let context_diversity = term_contexts
                .get(term)
                .map(|ctx| {
                    let unique: std::collections::HashSet<&str> =
                        ctx.iter().map(|s| s.as_str()).collect();
                    unique.len() as f64 / ctx.len().max(1) as f64
                })
                .unwrap_or(0.0);

            // Combine features. Lower = more relevant.
            let score = (pos_score + context_diversity) / (freq_score + 1.0);
            (term.as_str(), score)
        })
        .collect();

    // Build n-gram candidates (1-3 words).
    let mut candidates: HashMap<String, f64> = HashMap::new();

    for sent in sentences {
        let content: Vec<String> = sent
            .tokens
            .iter()
            .filter(|t| !t.is_punct)
            .map(|t| t.lemma.to_lowercase())
            .filter(|l| !is_stop_word(l) && l.len() > 1)
            .collect();

        for window in 1..=3.min(content.len()) {
            for ngram in content.windows(window) {
                let phrase = ngram.join(" ");
                // N-gram score: product of individual term scores.
                let score: f64 = ngram
                    .iter()
                    .map(|w| term_scores.get(w.as_str()).copied().unwrap_or(f64::MAX))
                    .product();
                let entry = candidates.entry(phrase).or_insert(f64::MAX);
                if score < *entry {
                    *entry = score;
                }
            }
        }
    }

    // Invert scores so higher = more relevant, then sort.
    let mut ranked: Vec<Keyphrase> = candidates
        .into_iter()
        .filter(|(_, score)| *score > 0.0 && score.is_finite())
        .map(|(phrase, score)| Keyphrase {
            phrase,
            score: 1.0 / score,
        })
        .collect();

    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked.truncate(max);
    ranked
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
        assert!(yake_keyphrases(&[], 5).is_empty());
    }

    #[test]
    fn max_zero_returns_empty() {
        let sentences = vec![sent("hello", vec![tok(1, "hello", "NOUN")])];
        assert!(yake_keyphrases(&sentences, 0).is_empty());
    }

    #[test]
    fn extracts_keyphrases() {
        let sentences = vec![
            sent(
                "machine learning algorithms process data",
                vec![
                    tok(1, "machine", "NOUN"),
                    tok(2, "learning", "NOUN"),
                    tok(3, "algorithms", "NOUN"),
                    tok(4, "process", "VERB"),
                    tok(5, "data", "NOUN"),
                ],
            ),
            sent(
                "machine learning models train on data",
                vec![
                    tok(1, "machine", "NOUN"),
                    tok(2, "learning", "NOUN"),
                    tok(3, "models", "NOUN"),
                    tok(4, "train", "VERB"),
                    tok(5, "data", "NOUN"),
                ],
            ),
        ];

        let result = yake_keyphrases(&sentences, 5);
        assert!(!result.is_empty());
        // "machine learning" should appear as a high-scoring phrase.
        let phrases: Vec<&str> = result.iter().map(|k| k.phrase.as_str()).collect();
        assert!(
            phrases
                .iter()
                .any(|p| p.contains("machine") && p.contains("learning")),
            "expected 'machine learning' in {:?}",
            phrases
        );
    }

    #[test]
    fn scores_are_positive() {
        let sentences = vec![sent(
            "architecture determines quality",
            vec![
                tok(1, "architecture", "NOUN"),
                tok(2, "determines", "VERB"),
                tok(3, "quality", "NOUN"),
            ],
        )];

        let result = yake_keyphrases(&sentences, 5);
        for kp in &result {
            assert!(kp.score > 0.0, "score should be positive: {}", kp.score);
        }
    }

    #[test]
    fn repeated_terms_score_higher() {
        let sentences = vec![
            sent(
                "system design patterns",
                vec![
                    tok(1, "system", "NOUN"),
                    tok(2, "design", "NOUN"),
                    tok(3, "patterns", "NOUN"),
                ],
            ),
            sent(
                "system design principles",
                vec![
                    tok(1, "system", "NOUN"),
                    tok(2, "design", "NOUN"),
                    tok(3, "principles", "NOUN"),
                ],
            ),
            sent(
                "unrelated words here",
                vec![
                    tok(1, "unrelated", "ADJ"),
                    tok(2, "words", "NOUN"),
                    tok(3, "here", "ADV"),
                ],
            ),
        ];

        let result = yake_keyphrases(&sentences, 3);
        // "system design" appears twice, should rank high.
        assert!(!result.is_empty());
        let top = &result[0];
        assert!(
            top.phrase.contains("system") || top.phrase.contains("design"),
            "expected repeated terms to rank high, got: {}",
            top.phrase
        );
    }
}
