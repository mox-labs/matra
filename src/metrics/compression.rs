//! Compression-ratio metric — brotli-compressed size / original size.
//!
//! Lower ratio = more compressible = more repetitive prose. A rough
//! proxy for surface redundancy in LLM-generated text.

use std::io::Write;

use crate::domain::{Analysis, Sentence};

/// Populate `Paragraph::compression_ratio` for every paragraph with
/// more than 50 words that is not in a blockquote.
pub fn compute(analysis: &mut Analysis, _sentences: &[Sentence]) {
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

    fn content_token(text: &str) -> Token {
        Token {
            id: 0,
            text: text.to_string(),
            lemma: text.to_lowercase(),
            pos: "NOUN".to_string(),
            xpos: String::new(),
            feats: String::new(),
            dep: "dep".to_string(),
            head: 0,
            deps: String::new(),
            misc: String::new(),
            is_punct: false,
        }
    }

    #[test]
    fn compression_ratio_bounds() {
        let ratio = compression_ratio("Some text to compress for testing purposes.")
            .expect("compression should succeed");
        assert!(ratio > 0.0 && ratio <= 2.0);
    }

    #[test]
    fn compression_word_count_boundary() {
        let text_50 = (0..50)
            .map(|i| format!("word{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let text_51 = (0..51)
            .map(|i| format!("word{i}"))
            .collect::<Vec<_>>()
            .join(" ");
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

        // Attach enough content tokens so word_count clears the thresholds.
        for (i, para) in analysis.paragraphs_mut().enumerate() {
            let tokens = (0..(50 + i))
                .map(|j| content_token(&format!("w{j}")))
                .collect();
            para.sentences.push(Sentence {
                text: String::new(),
                tokens,
            });
        }

        compute(&mut analysis, &empty);

        let paras: Vec<_> = analysis.paragraphs().collect();
        assert!(
            paras[0].compression_ratio.is_none(),
            "50 words should not get compression"
        );
        assert!(
            paras[1].compression_ratio.is_some(),
            "51 words should get compression"
        );
    }
}
