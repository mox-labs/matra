# RECOVERED-FROM-READ source=[claude-project-path]/[session-id]/subagents/[agent-transcript].jsonl timestamp=2026-04-09T13:02:50.563Z original_path=[path]/src/decompose/plain.rs
//! Plain text decomposer adapter. Splits on blank lines into paragraphs.

use crate::domain::{Paragraph, Section};

use super::Decomposer;

/// Decomposes plain text into a single section with paragraphs
/// split on blank lines.
pub struct PlainTextDecomposer;

impl Decomposer for PlainTextDecomposer {
    fn decompose(&self, text: &str) -> Vec<Section> {
        let paragraphs: Vec<Paragraph> = text
            .split("\n\n")
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(|p| Paragraph::new(p.to_string(), false))
            .collect();

        if paragraphs.is_empty() {
            return Vec::new();
        }

        vec![Section {
            heading: None,
            level: 0,
            paragraphs,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_blank_lines() {
        let d = PlainTextDecomposer;
        let sections = d.decompose("First paragraph.\n\nSecond paragraph.");
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].paragraphs.len(), 2);
    }

    #[test]
    fn empty_text_returns_empty() {
        let d = PlainTextDecomposer;
        assert!(d.decompose("").is_empty());
        assert!(d.decompose("   \n\n  ").is_empty());
    }

    #[test]
    fn single_paragraph() {
        let d = PlainTextDecomposer;
        let sections = d.decompose("Just one paragraph.");
        assert_eq!(sections[0].paragraphs.len(), 1);
    }
}

[result-id: r17]