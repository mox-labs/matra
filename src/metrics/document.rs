//! Document-level metrics — vocabulary TTR and nominalization ratio.
//!
//! These aggregate over every sentence attached to the document's
//! paragraphs ([`Document::sentences`]). Blockquote paragraphs carry no
//! sentences (the composition root skips them at parse time), so they
//! never contribute.

use crate::domain::Document;

const NOMINALIZATION_SUFFIXES: &[&str] = &["tion", "ment", "ness", "ity", "ence", "ance"];

/// Populate `Document::vocabulary_ttr` (type-token ratio over lemmas,
/// excluding punctuation), `Document::nominalization_ratio`
/// (share of NOUN tokens ending in a nominalizing suffix), and
/// `Document::passive_ratio` (share of sentences with a passive-voice
/// construction, materialized so the aggregate crosses FFI per
/// ADR-0008).
pub fn compute(analysis: &mut Document) {
    if analysis.total_sentences() > 0 {
        let ratio = analysis.passive_ratio();
        analysis.passive_ratio = Some(ratio);
    }

    let lemma_count = analysis.tokens().filter(|t| !t.is_punct).count();

    if lemma_count == 0 {
        return;
    }

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

    analysis.vocabulary_ttr = Some(unique_count as f64 / lemma_count as f64);
    analysis.nominalization_ratio = Some(nom_count as f64 / lemma_count as f64);
}
