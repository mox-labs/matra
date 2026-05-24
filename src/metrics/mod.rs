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
//! - [`document`]     — vocabulary TTR + nominalization ratio over the whole doc

pub mod compression;
pub mod document;
pub mod lexical;
pub mod readability;

use crate::domain::{Document, Sentence};

/// A metric reads NLP output and enriches the analysis.
/// `Box<dyn Fn>` allows closures with captured config/state.
pub type Metric = Box<dyn Fn(&mut Document, &[Sentence])>;

/// Default metric suite. Returns metrics in dependency order.
///
/// Per-paragraph metrics (readability, lexical, compression) read from
/// `paragraph.sentences` which the composition root populates by parsing
/// each paragraph individually. Document-level metrics run over the
/// flat sentence slice passed to [`run_suite`].
pub fn default_suite() -> Vec<Metric> {
    vec![
        Box::new(readability::compute),
        Box::new(lexical::compute),
        Box::new(document::compute),
        Box::new(compression::compute),
    ]
}

/// Run every metric in the suite, in order.
pub fn run_suite(analysis: &mut Document, sentences: &[Sentence], suite: &[Metric]) {
    for metric in suite {
        metric(analysis, sentences);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Paragraph, Section, Token};

    fn make_token(text: &str, pos: &str, dep: &str, head: usize) -> Token {
        Token {
            id: 0,
            text: text.to_string(),
            lemma: text.to_lowercase(),
            pos: pos.to_string(),
            xpos: String::new(),
            feats: String::new(),
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
            .map(|(text, tokens)| Sentence {
                text: text.to_string(),
                tokens,
            })
            .collect()
    }

    /// Build an analysis where each paragraph is pre-populated with its
    /// own sentences (mimicking what the composition root does in the
    /// per-paragraph parse pipeline). Returns the analysis plus a flat
    /// sentence slice for document-level metrics.
    fn analysis_from_paragraphs(
        paragraphs: Vec<(Paragraph, Vec<Sentence>)>,
    ) -> (Document, Vec<Sentence>) {
        let mut all_sentences = Vec::new();
        let mut paras = Vec::new();
        for (mut para, sents) in paragraphs {
            if !para.in_blockquote {
                para.sentences = sents.clone();
                all_sentences.extend(sents);
            }
            paras.push(para);
        }
        let sections = vec![Section {
            heading: None,
            level: 0,
            paragraphs: paras,
        }];
        (Document::new(sections), all_sentences)
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

        let (mut analysis, sentences) =
            analysis_from_paragraphs(vec![(Paragraph::new(para_text.to_string(), false), sents)]);
        let suite = default_suite();
        run_suite(&mut analysis, &sentences, &suite);

        assert_eq!(analysis.total_sentences(), 1);
        assert!(
            analysis.sentences().next().unwrap().is_passive(),
            "should detect passive voice"
        );
    }

    #[test]
    fn blockquote_paragraphs_have_no_sentences() {
        // Blockquote paragraphs are skipped by the composition root, so
        // they reach the suite with an empty sentences vec.
        let (mut analysis, sentences) = analysis_from_paragraphs(vec![(
            Paragraph::new("Some text here".to_string(), true),
            vec![],
        )]);
        let suite = default_suite();
        run_suite(&mut analysis, &sentences, &suite);

        assert_eq!(
            analysis.total_sentences(),
            0,
            "blockquote paragraphs should have no sentences"
        );
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

        let (mut analysis, sentences) = analysis_from_paragraphs(vec![
            (
                Paragraph::new("The system was built".to_string(), false),
                s1,
            ),
            (
                Paragraph::new("The team shipped the product".to_string(), false),
                s2,
            ),
        ]);
        let suite = default_suite();
        run_suite(&mut analysis, &sentences, &suite);

        assert_eq!(analysis.total_sentences(), 2);
        assert!(
            (analysis.passive_ratio() - 0.5).abs() < 0.01,
            "expected ~50% passive, got {}",
            analysis.passive_ratio()
        );
    }
}
