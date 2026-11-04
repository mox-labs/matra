//! Metric suite. Each metric reads NLP output and enriches the [`Document`].
//!
//! Metrics are closures with a uniform signature — add one by writing a
//! function and including it in [`default_suite`]. Sentence-to-paragraph
//! wiring is done by the composition root (one parse call per paragraph,
//! sentences attached directly), so the metrics here see paragraphs
//! already populated with their sentences.
//!
//! Submodules own the individual metrics:
//! - [`readability`]  — Flesch-Kincaid grade per paragraph
//! - [`lexical`]      — lexical density per paragraph
//! - [`compression`]  — brotli compression ratio per paragraph
//! - [`document`]     — the document metric record: counts, sentence length, TTR, nominalization, passive ratio

pub(crate) mod compression;
pub(crate) mod document;
pub(crate) mod lexical;
pub(crate) mod readability;

use crate::domain::Document;

/// A metric reads NLP output and enriches the analysis.
/// `Box<dyn Fn>` allows closures with captured config/state.
///
/// One parameter, one sentence set: every metric reads the sentences
/// attached to the document's paragraphs ([`Document::sentences`]).
/// The old two-parameter shape carried the sentence set twice (a flat
/// slice plus the paragraph attachment) with nothing enforcing
/// agreement, which is how `analyze_from` shipped half-populated
/// documents.
pub(crate) type Metric = Box<dyn Fn(&mut Document)>;

/// Default metric suite.
///
/// The suite is a list, not a chain. No metric reads a slot another metric
/// writes, so the order here is not load-bearing and a caller may drop,
/// reorder, or add entries freely.
///
/// Per-paragraph metrics (readability, lexical, compression) read from
/// `paragraph.sentences`, which the composition root populates by parsing
/// each paragraph individually. Document-level metrics aggregate over
/// the same attachment via [`Document::sentences`].
pub(crate) fn default_suite() -> Vec<Metric> {
    vec![
        Box::new(readability::compute),
        Box::new(lexical::compute),
        Box::new(document::compute),
        Box::new(compression::compute),
    ]
}

/// Run every metric in the suite, in order.
pub(crate) fn run_suite(analysis: &mut Document, suite: &[Metric]) {
    for metric in suite {
        metric(analysis);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Paragraph, ParagraphKind, Section, Sentence, Span, Token};

    fn make_token(text: &str, pos: &str, dep: &str, head: usize) -> Token {
        Token {
            id: 0,
            span: Default::default(),
            text: text.to_string(),
            lemma: text.to_lowercase(),
            pos: pos.to_string(),
            xpos: String::new(),
            features: std::collections::BTreeMap::new(),
            dep: dep.to_string(),
            head,
            deps: String::new(),
            misc: String::new(),
            is_punct: pos == "PUNCT",
        }
    }

    fn make_sentences(input: Vec<(&str, Vec<Token>)>) -> Vec<Sentence> {
        input
            .into_iter()
            .map(|(text, tokens)| Sentence::new(text.to_string(), tokens))
            .collect()
    }

    /// Build an analysis where each paragraph is pre-populated with its
    /// own sentences (mimicking what the composition root does in the
    /// per-paragraph parse pipeline).
    fn analysis_from_paragraphs(paragraphs: Vec<(Paragraph, Vec<Sentence>)>) -> Document {
        let mut paras = Vec::new();
        for (mut para, sents) in paragraphs {
            if para.kind != ParagraphKind::Quote {
                para.sentences = sents;
            }
            paras.push(para);
        }
        let sections = vec![Section {
            heading: None,
            level: 0,
            span: None,
            paragraphs: paras,
        }];
        Document::new(crate::domain::Provenance::for_test(), sections, Vec::new())
    }

    #[test]
    fn passive_voice_propagates_through_suite() {
        let para_text = "The system was built by the team";
        let sents = make_sentences(vec![(
            para_text,
            vec![
                make_token("The", "DET", "det", 2),
                make_token("system", "NOUN", "nsubj:pass", 4),
                make_token("was", "AUX", "aux:pass", 4),
                make_token("built", "VERB", "root", 0),
                make_token("by", "ADP", "case", 6),
                make_token("the", "DET", "det", 6),
                make_token("team", "NOUN", "obl", 4),
            ],
        )]);

        let mut analysis = analysis_from_paragraphs(vec![(
            Paragraph::new(
                para_text.to_string(),
                ParagraphKind::Paragraph,
                Span::default(),
            ),
            sents,
        )]);
        let suite = default_suite();
        run_suite(&mut analysis, &suite);

        assert_eq!(analysis.metrics.total_sentences, 1);
        assert!(
            analysis.sentences().next().unwrap().passive,
            "should detect passive voice"
        );
    }

    #[test]
    fn blockquote_paragraphs_have_no_sentences() {
        // Blockquote paragraphs are skipped by the composition root, so
        // they reach the suite with an empty sentences vec.
        let mut analysis = analysis_from_paragraphs(vec![(
            Paragraph::new(
                "Some text here".to_string(),
                ParagraphKind::Quote,
                Span::default(),
            ),
            vec![],
        )]);
        let suite = default_suite();
        run_suite(&mut analysis, &suite);

        assert_eq!(
            analysis.metrics.total_sentences, 0,
            "blockquote paragraphs should have no sentences"
        );
        assert_eq!(analysis.metrics.paragraph_count, 1);
        assert_eq!(analysis.metrics.mean_sentence_length, None);
        assert_eq!(analysis.metrics.passive_ratio, None);
    }

    #[test]
    fn document_passive_ratio_via_suite() {
        let s1 = make_sentences(vec![(
            "The system was built",
            vec![
                make_token("The", "DET", "det", 2),
                make_token("system", "NOUN", "nsubj:pass", 3),
                make_token("was", "AUX", "aux:pass", 3),
                make_token("built", "VERB", "root", 0),
            ],
        )]);
        let s2 = make_sentences(vec![(
            "The team shipped the product",
            vec![
                make_token("The", "DET", "det", 2),
                make_token("team", "NOUN", "nsubj", 3),
                make_token("shipped", "VERB", "root", 0),
                make_token("the", "DET", "det", 5),
                make_token("product", "NOUN", "obj", 3),
            ],
        )]);

        let mut analysis = analysis_from_paragraphs(vec![
            (
                Paragraph::new(
                    "The system was built".to_string(),
                    ParagraphKind::Paragraph,
                    Span::default(),
                ),
                s1,
            ),
            (
                Paragraph::new(
                    "The team shipped the product".to_string(),
                    ParagraphKind::Paragraph,
                    Span::default(),
                ),
                s2,
            ),
        ]);
        let suite = default_suite();
        run_suite(&mut analysis, &suite);

        assert_eq!(analysis.metrics.total_sentences, 2);
        assert_eq!(analysis.metrics.total_words, 9);
        assert_eq!(analysis.metrics.paragraph_count, 2);
        assert_eq!(analysis.metrics.mean_sentence_length, Some(4.5));
        let passive = analysis.metrics.passive_ratio.expect("two sentences");
        assert!(
            (passive - 0.5).abs() < 0.01,
            "expected ~50% passive, got {passive}"
        );
    }
}
