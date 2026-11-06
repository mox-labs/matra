//! Plain text reader. Paragraphs are separated by blank lines.

use crate::domain::{self, ParagraphKind, Span};

use super::{Decomposer, Read, ReadBlock, TextMap};

/// Reads plain text into paragraphs: runs of lines separated by lines
/// that hold only whitespace, so `\n` and `\r\n` line endings read the
/// same. Each paragraph's text is the run trimmed of surrounding
/// whitespace, a verbatim slice of the input, and its span is where that
/// slice lies. Plain text has no headings and skips nothing.
pub(crate) struct PlainTextDecomposer;

impl Decomposer for PlainTextDecomposer {
    fn decompose(&self, input: &str) -> domain::Result<Read> {
        Ok(Read {
            text: None,
            blocks: paragraphs(input),
            skipped: Vec::new(),
        })
    }
}

fn paragraphs(input: &str) -> Vec<ReadBlock> {
    let mut blocks = Vec::new();
    let mut run: Option<(usize, usize)> = None;
    let mut offset = 0;
    for line in input.split_inclusive('\n') {
        let end = offset + line.len();
        if line.trim().is_empty() {
            if let Some((start, stop)) = run.take() {
                blocks.push(paragraph(input, start, stop));
            }
        } else {
            run = Some((run.map_or(offset, |(start, _)| start), end));
        }
        offset = end;
    }
    if let Some((start, stop)) = run {
        blocks.push(paragraph(input, start, stop));
    }
    blocks
}

/// The paragraph whose lines run from `start` to `stop`, trimmed.
fn paragraph(input: &str, start: usize, stop: usize) -> ReadBlock {
    let raw = &input[start..stop];
    let lead = raw.len() - raw.trim_start().len();
    let text = raw.trim();
    let begin = start + lead;
    ReadBlock::Paragraph {
        kind: ParagraphKind::Paragraph,
        text: text.to_string(),
        span: Span::new(begin, begin + text.len()),
        map: TextMap::verbatim(begin, text.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts_and_spans(input: &str) -> Vec<(String, Span)> {
        paragraphs(input)
            .into_iter()
            .map(|b| match b {
                ReadBlock::Paragraph { text, span, .. } => (text, span),
                ReadBlock::Heading { .. } => panic!("plain text has no headings"),
            })
            .collect()
    }

    #[test]
    fn splits_on_blank_lines() {
        let got = texts_and_spans("First paragraph.\n\nSecond paragraph.");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0], ("First paragraph.".to_string(), Span::new(0, 16)));
        assert_eq!(got[1], ("Second paragraph.".to_string(), Span::new(18, 35)));
    }

    #[test]
    fn empty_text_returns_empty() {
        assert!(paragraphs("").is_empty());
        assert!(paragraphs("   \n\n  ").is_empty());
    }

    #[test]
    fn single_paragraph() {
        assert_eq!(paragraphs("Just one paragraph.").len(), 1);
    }

    /// A `\r\n` file reads as the same paragraphs a `\n` file does, and
    /// each paragraph is the verbatim slice its span names.
    #[test]
    fn crlf_line_endings_separate_paragraphs() {
        let input = "One line.\r\nSame paragraph.\r\n\r\nNext one.\r\n";
        let got = texts_and_spans(input);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].0, "One line.\r\nSame paragraph.");
        assert_eq!(got[1].0, "Next one.");
        for (text, span) in got {
            assert_eq!(&input[span.byte_start..span.byte_end], text);
        }
    }

    #[test]
    fn a_whitespace_only_line_is_a_blank_line() {
        let got = texts_and_spans("  Indented start.\n \t \nAfter.");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0], ("Indented start.".to_string(), Span::new(2, 17)));
    }
}
