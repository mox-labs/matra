//! Lexical density metric — content words over total words, per paragraph.

use crate::domain::{Document, ParagraphKind};
use crate::stopwords::is_stop_word;

/// Populate `ParagraphMetrics::lexical_density` for every non-empty,
/// paragraph that is not a quote.
pub(crate) fn compute(analysis: &mut Document) {
    for para in analysis.paragraphs_mut() {
        if para.word_count() == 0 || para.kind == ParagraphKind::Quote {
            continue;
        }
        let words: Vec<&str> = para.text.split_whitespace().collect();
        let total = words.len() as f64;
        if total == 0.0 {
            continue;
        }
        let content = words
            .iter()
            .filter(|w| {
                let clean: String = w.chars().filter(|c| c.is_alphabetic()).collect();
                let lower = clean.to_lowercase();
                !lower.is_empty() && !is_stop_word(&lower)
            })
            .count() as f64;
        para.metrics.lexical_density = Some(content / total);
    }
}
