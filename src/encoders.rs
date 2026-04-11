# RECOVERED-FROM-READ source=[claude-project-path]/[session-id]/subagents/[agent-transcript].jsonl timestamp=2026-04-09T13:02:33.372Z original_path=[path]/src/encoders.rs
//! Encoder pipeline. Each encoder reads from NLP output and enriches the Analysis.
//! Encoders run in order. Adding one = write a function (or closure), add to pipeline.

use std::io::Write;

use crate::domain::{Analysis, Sentence};

/// An encoder takes NLP output and enriches the analysis.
/// `Box<dyn Fn>` allows closures with captured config/state.
pub type Encoder = Box<dyn Fn(&mut Analysis, &[Sentence])>;

/// Default pipeline. Returns encoders in dependency order.
///
/// Order is significant: `encode_sentences` assigns NLP sentences to
/// paragraphs, which downstream encoders (readability, lexical, compression)
/// rely on for word counts. `encode_document` computes vocabulary and
/// nominalization from the raw NLP sentences.
pub fn default_pipeline() -> Vec<Encoder> {
    vec![
        Box::new(encode_sentences),
        Box::new(encode_readability),
        Box::new(encode_lexical),
        Box::new(encode_document),
        Box::new(encode_compression),
    ]
}

/// Run all encoders in order.
pub fn run_pipeline(analysis: &mut Analysis, sentences: &[Sentence], pipeline: &[Encoder]) {
    for encoder in pipeline {
        encoder(analysis, sentences);
    }
}

// ---------------------------------------------------------------------------
// Sentence encoder
// ---------------------------------------------------------------------------

fn encode_sentences(analysis: &mut Analysis, sentences: &[Sentence]) {
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

// ---------------------------------------------------------------------------
// Readability encoder (pure math)
// ---------------------------------------------------------------------------

fn encode_readability(analysis: &mut Analysis, _sentences: &[Sentence]) {
    for para in analysis.paragraphs_mut() {
        if para.word_count() > 10 && !para.in_blockquote {
            para.readability_grade = Some(flesch_kincaid_grade(&para.text));
        }
    }
}

fn syllable_count(word: &str) -> usize {
    let word = word.to_lowercase();
    if word.len() <= 3 {
        return 1;
    }
    let vowels = b"aeiouy";
    let mut count = 0;
    let mut prev_vowel = false;
    for &b in word.as_bytes() {
        let is_vowel = vowels.contains(&b);
        if is_vowel && !prev_vowel {
            count += 1;
        }
        prev_vowel = is_vowel;
    }
    if word.ends_with('e') && count > 1 {
        count -= 1;
    }
    count.max(1)
}

/// Flesch-Kincaid grade level. Uses raw whitespace splitting per the
/// formula's original specification, not NLP token counts.
pub fn flesch_kincaid_grade(text: &str) -> f64 {
    let words: Vec<&str> = text.split_whitespace().collect();
    let n = words.len() as f64;
    if n == 0.0 {
        return 0.0;
    }
    let sents = text.chars().filter(|c| matches!(c, '.' | '!' | '?')).count().max(1) as f64;
    let syllables: f64 = words.iter().map(|w| {
        let clean: String = w.chars().filter(|c| c.is_alphabetic()).collect();
        if clean.is_empty() { 0.0 } else { syllable_count(&clean) as f64 }
    }).sum();
    0.39 * (n / sents) + 11.8 * (syllables / n) - 15.59
}

// ---------------------------------------------------------------------------
// Lexical encoder
// ---------------------------------------------------------------------------

use crate::stopwords::is_stop_word;

fn encode_lexical(analysis: &mut Analysis, _sentences: &[Sentence]) {
    for para in analysis.paragraphs_mut() {
        if para.word_count() == 0 || para.in_blockquote {
            continue;
        }
        let words: Vec<&str> = para.text.split_whitespace().collect();
        let total = words.len() as f64;
        if total == 0.0 {
            continue;
        }
        let content = words.iter().filter(|w| {
            let clean: String = w.chars().filter(|c| c.is_alphabetic()).collect();
            let lower = clean.to_lowercase();
            !lower.is_empty() && !is_stop_word(&lower)
        }).count() as f64;
        para.lexical_density = Some(content / total);
    }
}

// ---------------------------------------------------------------------------
// Document encoder (aggregates)
// ---------------------------------------------------------------------------

fn encode_document(analysis: &mut Analysis, sentences: &[Sentence]) {
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

    let nom_suffixes = ["tion", "ment", "ness", "ity", "ence", "ance"];
    let nom_count = sentences
        .iter()
        .flat_map(|s| s.tokens.iter())
        .filter(|t| {
            t.pos == "NOUN"
                && nom_suffixes
                    .iter()
                    .any(|suf| t.text.to_lowercase().ends_with(suf))
        })
        .count();
    let content_count = lemmas.len();
    if content_count > 0 {
        analysis.nominalization_ratio = Some(nom_count as f64 / content_count as f64);
    }
}

// ---------------------------------------------------------------------------
// Compression encoder (AI detection via brotli ratio)
// ---------------------------------------------------------------------------

fn encode_compression(analysis: &mut Analysis, _sentences: &[Sentence]) {
    for para in analysis.paragraphs_mut() {
        if para.word_count() > 50 && !para.in_blockquote {
            para.compression_ratio = compression_ratio(&para.text);
        }
    }
}

fn compression_ratio(text: &str) -> Option<f64> {
    let original = text.as_bytes();
    if original.is_empty() {
        return Some(1.0);
    }
    let mut compressed = Vec::new();
    {
        let mut writer = brotli::CompressorWriter::new(&mut compressed, 4096, 6, 22);
        writer.write_all(original).ok()?;
    }
    Some(compressed.len() as f64 / original.len() as f64)
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
    fn test_syllable_count() {
        assert_eq!(syllable_count("the"), 1);
        assert_eq!(syllable_count("beautiful"), 3);
    }

    #[test]
    fn test_flesch_kincaid() {
        let grade = flesch_kincaid_grade("The cat sat on the mat. The dog chased the cat.");
        assert!(grade > -5.0 && grade < 20.0, "FK grade was {grade}");
    }

    #[test]
    fn test_compression_ratio_bounds() {
        let ratio = compression_ratio("Some text to compress for testing purposes.");
        let ratio = ratio.expect("compression should succeed");
        assert!(ratio > 0.0 && ratio <= 2.0);
    }

    #[test]
    fn test_encode_sentences_detects_passive() {
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
        encode_sentences(&mut analysis, &sentences);

        assert_eq!(analysis.total_sentences(), 1);
        assert!(
            analysis.sentences().next().unwrap().is_passive(),
            "should detect passive voice"
        );
    }

    #[test]
    fn test_encode_document_passive_ratio() {
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
        encode_sentences(&mut analysis, &sentences);

        assert_eq!(analysis.total_sentences(), 2);
        assert!(
            (analysis.passive_ratio() - 0.5).abs() < 0.01,
            "expected ~50% passive, got {}",
            analysis.passive_ratio()
        );
    }

    #[test]
    fn test_encode_compression_boundary() {
        let text_50 = (0..50).map(|i| format!("word{i}")).collect::<Vec<_>>().join(" ");
        let text_51 = (0..51).map(|i| format!("word{i}")).collect::<Vec<_>>().join(" ");

        let empty: Vec<Sentence> = vec![];

        let sections = vec![Section {
            heading: None,
            level: 0,
            paragraphs: vec![
                Paragraph::new(text_50, false),
                Paragraph::new(text_51, false),
            ],
        }];
        let mut analysis = Analysis::new(sections);

        // Push dummy Sentences with enough non-punct tokens to reach word counts.
        for (i, para) in analysis.paragraphs_mut().enumerate() {
            let tokens = (0..(50 + i))
                .map(|j| make_token(&format!("w{j}"), "NOUN", "dep", 0))
                .collect();
            para.sentences.push(Sentence {
                text: String::new(),
                tokens,
            });
        }

        encode_compression(&mut analysis, &empty);

        let paras: Vec<_> = analysis.paragraphs().collect();
        assert!(paras[0].compression_ratio.is_none(), "50 words should not get compression");
        assert!(paras[1].compression_ratio.is_some(), "51 words should get compression");
    }

    #[test]
    fn test_encode_sentences_skips_blockquotes() {
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
            paragraphs: vec![
                Paragraph::new("Some text here".to_string(), true),
            ],
        }];
        let mut analysis = Analysis::new(sections);
        encode_sentences(&mut analysis, &sentences);

        assert_eq!(analysis.total_sentences(), 0, "blockquote paragraphs should be skipped");
    }
}

[result-id: r1]