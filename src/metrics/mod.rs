//! Metric suite. Each metric reads NLP output and enriches the [`Analysis`].
//!
//! Metrics are closures with a uniform signature — add one by writing a
//! function and including it in [`default_suite`]. Order matters:
//! [`attach_sentences`] must run first so downstream metrics can see the
//! per-paragraph sentence assignments.
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

use crate::domain::{Analysis, Sentence};

/// A metric reads NLP output and enriches the analysis.
/// `Box<dyn Fn>` allows closures with captured config/state.
pub type Metric = Box<dyn Fn(&mut Analysis, &[Sentence])>;

/// Default metric suite. Returns metrics in dependency order.
///
/// [`attach_sentences`] wires NLP sentences into paragraphs; downstream
/// metrics (readability, lexical, compression) rely on that wiring for
/// per-paragraph word counts. Document-level metrics run over the raw
/// sentence slice, independent of paragraph assignment.
pub fn default_suite() -> Vec<Metric> {
    vec![
        Box::new(attach_sentences),
        Box::new(readability::compute),
        Box::new(lexical::compute),
        Box::new(document::compute),
        Box::new(compression::compute),
    ]
}

/// Run every metric in the suite, in order.
pub fn run_suite(analysis: &mut Analysis, sentences: &[Sentence], suite: &[Metric]) {
    for metric in suite {
        metric(analysis, sentences);
    }
}

/// Assign NLP sentences to paragraphs by prefix match against the
/// original paragraph text. This is a pipeline *wiring* step, not a
/// metric — it produces no scalar. Runs first in the default suite.
pub fn attach_sentences(analysis: &mut Analysis, sentences: &[Sentence]) {
    let mut assigned = vec![false; sentences.len()];
    let mut assigned_count = 0usize;

    for para in analysis.paragraphs_mut() {
        if para.in_blockquote || assigned_count == sentences.len() {
            continue;
        }

        for (sent_idx, sent) in sentences.iter().enumerate() {
            if assigned[sent_idx] {
                continue;
            }
            let prefix_end = sent
                .text
                .char_indices()
                .nth(30)
                .map(|(i, _)| i)
                .unwrap_or(sent.text.len());
            let sent_prefix = &sent.text[..prefix_end];
            if !para.text.contains(sent_prefix) {
                continue;
            }

            para.sentences.push(sent.clone());
            assigned[sent_idx] = true;
            assigned_count += 1;
        }
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

    #[test]
    fn attach_sentences_detects_passive() {
        let para_text = "The system was built by the team";
        let sentences = make_sentences(vec![(
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

        let sections = vec![Section {
            heading: None,
            level: 0,
            paragraphs: vec![Paragraph::new(para_text.to_string(), false)],
        }];
        let mut analysis = Analysis::new(sections);
        attach_sentences(&mut analysis, &sentences);

        assert_eq!(analysis.total_sentences(), 1);
        assert!(
            analysis.sentences().next().unwrap().is_passive(),
            "should detect passive voice"
        );
    }

    #[test]
    fn attach_sentences_skips_blockquotes() {
        let sentences = make_sentences(vec![(
            "Some text here",
            vec![
                make_token("Some", "DET", "det", 2),
                make_token("text", "NOUN", "nsubj", 3),
                make_token("here", "ADV", "root", 0),
            ],
        )]);

        let sections = vec![Section {
            heading: None,
            level: 0,
            paragraphs: vec![Paragraph::new("Some text here".to_string(), true)],
        }];
        let mut analysis = Analysis::new(sections);
        attach_sentences(&mut analysis, &sentences);

        assert_eq!(
            analysis.total_sentences(),
            0,
            "blockquote paragraphs should be skipped"
        );
    }

    #[test]
    fn document_passive_ratio_via_suite() {
        let sentences = make_sentences(vec![
            (
                "The system was built",
                vec![
                    make_token("The", "DET", "det", 2),
                    make_token("system", "NOUN", "nsubj:pass", 3),
                    make_token("was", "AUX", "aux:pass", 3),
                    make_token("built", "VERB", "root", 0),
                ],
            ),
            (
                "The team shipped the product",
                vec![
                    make_token("The", "DET", "det", 2),
                    make_token("team", "NOUN", "nsubj", 3),
                    make_token("shipped", "VERB", "root", 0),
                    make_token("the", "DET", "det", 5),
                    make_token("product", "NOUN", "obj", 3),
                ],
            ),
        ]);

        let sections = vec![Section {
            heading: None,
            level: 0,
            paragraphs: vec![
                Paragraph::new("The system was built".to_string(), false),
                Paragraph::new("The team shipped the product".to_string(), false),
            ],
        }];
        let mut analysis = Analysis::new(sections);
        attach_sentences(&mut analysis, &sentences);

        assert_eq!(analysis.total_sentences(), 2);
        assert!(
            (analysis.passive_ratio() - 0.5).abs() < 0.01,
            "expected ~50% passive, got {}",
            analysis.passive_ratio()
        );
    }
}
