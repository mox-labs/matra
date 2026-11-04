//! YAKE keyphrase extraction.
//!
//! Yet Another Keyword Extractor. Unsupervised statistical approach that
//! scores individual words by positional, frequency, and context features,
//! then builds n-gram candidates. Lower YAKE score = more relevant.
//! Output is normalized so higher score = more relevant (inverted).

use std::collections::HashMap;

use crate::domain::{Error, Keyphrase, Result, Sentence, Span};
use crate::stopwords::is_stop_word;

/// YAKE accumulates per-term position vectors, context-word sets, and
/// 1-/2-/3-gram candidates keyed by phrase strings. The dominant cost is
/// the n-gram windowing over content tokens: at 200k tokens the n-gram
/// candidate map holds <= ~600k entries (3-grams strictly bounded by token
/// count) at ~64 bytes each = ~40 MiB resident; per-term context sets are
/// bounded by mean window size (~5 surrounding words). Cap is on tokens
/// (not sentences) per Knuth's correction — same reasoning as RAKE.
const MAX_TOKENS: usize = 200_000;

/// At most `n` keyphrases ranked by YAKE, each with every place it
/// occurs.
///
/// Scores individual terms by position, frequency, and context diversity,
/// then combines into n-gram candidates (1-3 words). Returns top phrases
/// sorted by relevance (highest score first).
///
/// Returns [`Error::InputTooLarge`] when the total token count across all
/// input sentences exceeds the per-extractor `MAX_TOKENS` cap.
pub(super) fn keyphrases(sentences: &[&Sentence], n: usize) -> Result<Vec<Keyphrase>> {
    if sentences.is_empty() || n == 0 {
        return Ok(Vec::new());
    }
    let total_tokens: usize = sentences.iter().map(|s| s.tokens.len()).sum();
    if total_tokens > MAX_TOKENS {
        return Err(Error::InputTooLarge {
            limit: MAX_TOKENS,
            actual: total_tokens,
            what: "yake",
        });
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
        return Ok(Vec::new());
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

    // Build n-gram candidates (1-3 words), in the order they are first
    // met, with every window each one occurs in.
    let mut candidates: Vec<(String, f64, Vec<Span>)> = Vec::new();
    let mut slot: HashMap<String, usize> = HashMap::new();

    for sent in sentences {
        let content: Vec<(String, Span)> = sent
            .tokens
            .iter()
            .filter(|t| !t.is_punct)
            .map(|t| (t.lemma.to_lowercase(), t.span))
            .filter(|(l, _)| !is_stop_word(l) && l.len() > 1)
            .collect();

        for window in 1..=3.min(content.len()) {
            for ngram in content.windows(window) {
                let phrase = ngram
                    .iter()
                    .map(|(lemma, _)| lemma.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                // N-gram score: product of individual term scores.
                let score: f64 = ngram
                    .iter()
                    .map(|(w, _)| term_scores.get(w.as_str()).copied().unwrap_or(f64::MAX))
                    .product();
                // A window spans its first word's start to its last
                // word's end, stop words between them included.
                let span = Span::new(ngram[0].1.start, ngram[ngram.len() - 1].1.end);
                match slot.get(&phrase) {
                    Some(&at) => {
                        let entry = &mut candidates[at];
                        if score < entry.1 {
                            entry.1 = score;
                        }
                        entry.2.push(span);
                    }
                    None => {
                        slot.insert(phrase.clone(), candidates.len());
                        candidates.push((phrase, score.min(f64::MAX), vec![span]));
                    }
                }
            }
        }
    }

    // Invert scores so higher = more relevant, then rank.
    let phrases: Vec<Keyphrase> = candidates
        .into_iter()
        .filter(|(_, score, _)| *score > 0.0 && score.is_finite())
        .map(|(phrase, score, occurrences)| Keyphrase {
            phrase,
            score: 1.0 / score,
            occurrences,
        })
        .collect();
    Ok(super::rank_phrases(phrases, n))
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

    /// Every occurrence of a phrase is reported, in document order, and a
    /// window that passes over a stop word spans it.
    #[test]
    fn a_phrase_reports_every_occurrence() {
        let text = "red data of red data";
        let mut at = 0;
        let tokens = text
            .split(' ')
            .enumerate()
            .map(|(i, word)| {
                let mut token = tok(i + 1, word, "NOUN");
                token.span = Span::new(at, at + word.len());
                at += word.len() + 1;
                token
            })
            .collect();
        let result = run(&[sent(text, tokens)], 20).unwrap();
        let occurrences = |phrase: &str| {
            result
                .iter()
                .find(|k| k.phrase == phrase)
                .map(|k| k.occurrences.clone())
                .unwrap_or_default()
        };
        assert_eq!(
            occurrences("red data"),
            [Span::new(0, 8), Span::new(12, 20)]
        );
        assert_eq!(occurrences("data red"), [Span::new(4, 15)]);
        assert_eq!(&text[4..15], "data of red");
    }

    #[test]
    fn empty_returns_empty() {
        assert!(keyphrases(&[], 5).unwrap().is_empty());
    }

    #[test]
    fn max_zero_returns_empty() {
        let sentences = vec![sent("hello", vec![tok(1, "hello", "NOUN")])];
        assert!(run(&sentences, 0).unwrap().is_empty());
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

        let result = run(&sentences, 5).unwrap();
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

        let result = run(&sentences, 5).unwrap();
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

        let result = run(&sentences, 3).unwrap();
        // "system design" appears twice, should rank high.
        assert!(!result.is_empty());
        let top = &result[0];
        assert!(
            top.phrase.contains("system") || top.phrase.contains("design"),
            "expected repeated terms to rank high, got: {}",
            top.phrase
        );
    }

    #[test]
    fn rejects_oversized_input() {
        // One sentence with MAX_TOKENS + 1 tokens, exceeds cap.
        let tokens: Vec<crate::domain::Token> = (0..MAX_TOKENS + 1)
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
                assert_eq!(what, "yake");
            }
            other => panic!("expected InputTooLarge, got {other:?}"),
        }
    }
}
