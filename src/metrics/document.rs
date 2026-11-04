//! Document-level metrics: the counts, the sentence-length statistics,
//! vocabulary TTR, nominalization ratio and passive ratio.
//!
//! These aggregate over every sentence attached to the document's
//! paragraphs ([`Document::sentences`]). Blockquote paragraphs carry no
//! sentences (the composition root skips them at parse time), so they
//! never contribute to anything but `paragraph_count`.

use crate::domain::{Document, DocumentMetrics};

const NOMINALIZATION_SUFFIXES: &[&str] = &["tion", "ment", "ness", "ity", "ence", "ance"];

/// Fill [`Document::metrics`], the [`DocumentMetrics`] record.
///
/// The counts are always filled. Each ratio gates on its own condition
/// and stays `None` when unmet: the sentence-length mean and the
/// passive ratio need a sentence, the sample standard deviation needs
/// two, and the type-token and nominalization ratios need a
/// non-punctuation token. Materialized so the aggregates cross FFI as
/// data (RFC-0008).
pub(crate) fn compute(analysis: &mut Document) {
    let lengths: Vec<usize> = analysis.sentences().map(|s| s.word_count).collect();
    let total_sentences = lengths.len();
    let total_words: usize = lengths.iter().sum();
    let paragraph_count = analysis.paragraphs().count();

    let mean_sentence_length =
        (total_sentences > 0).then(|| total_words as f64 / total_sentences as f64);
    let sentence_length_std = sample_std(&lengths);
    let passive_ratio = (total_sentences > 0).then(|| {
        analysis.sentences().filter(|s| s.passive).count() as f64 / total_sentences as f64
    });

    let lemma_count = analysis.tokens().filter(|t| !t.is_punct).count();
    let (vocabulary_ttr, nominalization_ratio) = if lemma_count == 0 {
        (None, None)
    } else {
        let unique_count = analysis
            .tokens()
            .filter(|t| !t.is_punct)
            .map(|t| t.lemma.as_str())
            .collect::<std::collections::HashSet<&str>>()
            .len();
        let nom_count = analysis
            .tokens()
            .filter(|t| {
                t.pos == "NOUN"
                    && NOMINALIZATION_SUFFIXES
                        .iter()
                        .any(|suf| t.text.to_lowercase().ends_with(suf))
            })
            .count();
        (
            Some(unique_count as f64 / lemma_count as f64),
            Some(nom_count as f64 / lemma_count as f64),
        )
    };

    analysis.metrics = DocumentMetrics {
        total_words,
        total_sentences,
        paragraph_count,
        mean_sentence_length,
        sentence_length_std,
        vocabulary_ttr,
        nominalization_ratio,
        passive_ratio,
    };
}

/// Sample standard deviation of `values`, or `None` for fewer than two,
/// where it is undefined.
fn sample_std(values: &[usize]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<usize>() as f64 / n;
    let var = values
        .iter()
        .map(|&v| (v as f64 - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0);
    Some(var.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_std_is_undefined_below_two_values() {
        assert_eq!(sample_std(&[]), None);
        assert_eq!(sample_std(&[7]), None);
    }

    #[test]
    fn sample_std_matches_the_textbook_value() {
        // 2, 4, 4, 4, 5, 5, 7, 9: mean 5, sum of squares 32, n - 1 = 7.
        let got = sample_std(&[2, 4, 4, 4, 5, 5, 7, 9]).expect("defined");
        assert!((got - (32.0f64 / 7.0).sqrt()).abs() < 1e-12, "{got}");
    }
}
