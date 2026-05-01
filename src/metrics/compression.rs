//! Compression-ratio metric — brotli-compressed size / original size.
//!
//! Lower ratio = more compressible = more repetitive prose. A rough
//! proxy for surface redundancy in LLM-generated text.

use std::io::Write;

use crate::domain::{Analysis, Sentence};

/// Brotli sliding-window size (log2 bytes). 18 = 256 KiB window.
///
/// Down from 22 (4 MiB) per Vector's HIGH finding: a 4 MiB window
/// per paragraph multiplies CPU pegging risk on adversarial input.
/// 18 is the safe-by-default ceiling for prose: long-range redundancy
/// across 256 KiB is more than enough for compression-as-redundancy-proxy
/// signal; beyond that the metric is measuring engine plumbing, not
/// linguistic structure.
const BROTLI_LGWIN: u32 = 18;

/// Brotli quality level (0–11). 6 is mid-range — balances time vs ratio
/// for our metric purpose. We are not optimizing storage; we are getting
/// a redundancy proxy. Higher quality only marginally tightens the ratio
/// on prose at significant CPU cost.
const BROTLI_QUALITY: u32 = 6;

/// Per-paragraph byte ceiling. Above this, the compression metric is
/// skipped (`compression_ratio = None`) to bound worst-case CPU on
/// adversarial input. 256 KiB matches the brotli window so a single
/// paragraph never triggers more than one window's worth of work.
const MAX_PARAGRAPH_BYTES: usize = 256 * 1024;

/// Populate `Paragraph::compression_ratio` for every paragraph with
/// more than 50 words that is not in a blockquote and is at or under
/// the per-paragraph byte cap.
pub fn compute(analysis: &mut Analysis, _sentences: &[Sentence]) {
    for para in analysis.paragraphs_mut() {
        if para.word_count() > 50 && !para.in_blockquote && para.text.len() <= MAX_PARAGRAPH_BYTES {
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
        let mut writer =
            brotli::CompressorWriter::new(&mut compressed, 4096, BROTLI_QUALITY, BROTLI_LGWIN);
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
    fn oversize_paragraph_is_skipped_quickly() {
        // 1 MiB single paragraph (4× the per-paragraph cap). Should be
        // skipped (compression_ratio = None) and the whole compute call
        // should return well under 100ms — we are measuring the gate,
        // not brotli.
        let big_text = "a ".repeat(MAX_PARAGRAPH_BYTES); // ~512 KiB after duplication, but we want > cap
        let big_text = big_text.repeat(2); // ~1 MiB, well over MAX_PARAGRAPH_BYTES
        assert!(big_text.len() > MAX_PARAGRAPH_BYTES);

        let sections = vec![Section {
            heading: None,
            level: 0,
            paragraphs: vec![Paragraph::new(big_text, false)],
        }];
        let mut analysis = Analysis::new(sections);

        // Attach enough content tokens so word_count clears the >50 threshold.
        for para in analysis.paragraphs_mut() {
            let tokens = (0..51).map(|j| content_token(&format!("w{j}"))).collect();
            para.sentences.push(Sentence {
                text: String::new(),
                tokens,
            });
        }

        let empty: Vec<Sentence> = vec![];
        let start = std::time::Instant::now();
        compute(&mut analysis, &empty);
        let elapsed = start.elapsed();

        let paras: Vec<_> = analysis.paragraphs().collect();
        assert!(
            paras[0].compression_ratio.is_none(),
            "oversize paragraph should be skipped"
        );
        assert!(
            elapsed < std::time::Duration::from_millis(100),
            "skipping oversize paragraph took {elapsed:?} (expected < 100ms)"
        );
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
