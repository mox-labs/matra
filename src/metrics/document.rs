//! Document-level metrics — vocabulary TTR and nominalization ratio.
//!
//! These aggregate over the raw sentence slice (independent of paragraph
//! attachment) so `Document::vocabulary_ttr` and `Document::nominalization_ratio`
//! reflect the whole text, including blockquotes if the caller included them.

use crate::domain::{Document, Sentence};

const NOMINALIZATION_SUFFIXES: &[&str] = &["tion", "ment", "ness", "ity", "ence", "ance"];

/// Populate `Document::vocabulary_ttr` (type-token ratio over lemmas,
/// excluding punctuation) and `Document::nominalization_ratio`
/// (share of NOUN tokens ending in a nominalizing suffix).
pub fn compute(analysis: &mut Document, sentences: &[Sentence]) {
    let lemmas: Vec<&str> = sentences
        .iter()
        .flat_map(|s| s.tokens.iter())
        .filter(|t| !t.is_punct)
        .map(|t| t.lemma.as_str())
        .collect();

    if !lemmas.is_empty() {
        let unique: std::collections::HashSet<&str> = lemmas.iter().copied().collect();
        analysis.vocabulary_ttr = Some(unique.len() as f64 / lemmas.len() as f64);
    }

    let nom_count = sentences
        .iter()
        .flat_map(|s| s.tokens.iter())
        .filter(|t| {
            t.pos == "NOUN"
                && NOMINALIZATION_SUFFIXES
                    .iter()
                    .any(|suf| t.text.to_lowercase().ends_with(suf))
        })
        .count();
    if !lemmas.is_empty() {
        analysis.nominalization_ratio = Some(nom_count as f64 / lemmas.len() as f64);
    }
}
