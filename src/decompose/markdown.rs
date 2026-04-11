//! Markdown decomposer adapter. Extracts sections, paragraphs, blockquotes.

use crate::domain::{Paragraph, Section};

use super::Decomposer;

/// Decomposes markdown text into sections with paragraph awareness.
///
/// Handles YAML frontmatter, code blocks, blockquotes, tables,
/// and reference sections.
pub struct MarkdownDecomposer;

impl Decomposer for MarkdownDecomposer {
    fn decompose(&self, text: &str) -> Vec<Section> {
        parse(text)
    }
}

/// Parse markdown into sections with paragraph awareness.
pub fn parse(text: &str) -> Vec<Section> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut sections: Vec<Section> = Vec::new();
    let mut current = Section {
        heading: None,
        level: 0,
        paragraphs: Vec::new(),
    };
    let mut para_lines: Vec<&str> = Vec::new();
    let mut in_frontmatter = false;
    let mut in_code_block = false;
    let mut in_blockquote = false;

    let flush = |para_lines: &mut Vec<&str>, in_bq: &mut bool, current: &mut Section| {
        if !para_lines.is_empty() {
            let text = para_lines.join("\n").trim().to_string();
            if !text.is_empty() {
                current.paragraphs.push(Paragraph::new(text, *in_bq));
            }
            para_lines.clear();
            *in_bq = false;
        }
    };

    for (i, line) in lines.iter().enumerate() {
        let stripped = line.trim();

        if i == 0 && stripped == "---" {
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter {
            if stripped == "---" {
                in_frontmatter = false;
            }
            continue;
        }
        if stripped.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block || stripped.starts_with('|') {
            continue;
        }
        if stripped == "## References" || stripped == "*References*" {
            flush(&mut para_lines, &mut in_blockquote, &mut current);
            break;
        }
        if stripped.starts_with('#') {
            flush(&mut para_lines, &mut in_blockquote, &mut current);
            if current.heading.is_some() || !current.paragraphs.is_empty() {
                sections.push(current);
            }
            let level = stripped.bytes().take_while(|&b| b == b'#').count();
            let heading = stripped[level..].trim().to_string();
            current = Section {
                heading: Some(heading),
                level,
                paragraphs: Vec::new(),
            };
            continue;
        }
        if stripped.starts_with('>') {
            in_blockquote = true;
            para_lines.push(stripped.trim_start_matches('>').trim());
            continue;
        }
        if stripped.is_empty() {
            flush(&mut para_lines, &mut in_blockquote, &mut current);
            continue;
        }
        para_lines.push(line);
    }

    flush(&mut para_lines, &mut in_blockquote, &mut current);
    if current.heading.is_some() || !current.paragraphs.is_empty() {
        sections.push(current);
    }

    sections
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_markdown() {
        let md = "---\ntitle: Test\n---\n\n## Intro\n\nFirst paragraph.\n\nSecond paragraph.\n\n## Body\n\nBody text.";
        let sections = parse(md);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].heading.as_deref(), Some("Intro"));
        assert_eq!(sections[0].paragraphs.len(), 2);
        assert_eq!(sections[1].heading.as_deref(), Some("Body"));
        assert_eq!(sections[1].paragraphs.len(), 1);
    }

    #[test]
    fn test_blockquote_detection() {
        let md = "## Section\n\nNormal text.\n\n> This is a quote.";
        let sections = parse(md);
        assert!(!sections[0].paragraphs[0].in_blockquote);
        assert!(sections[0].paragraphs[1].in_blockquote);
    }

    #[test]
    fn test_skips_code_blocks() {
        let md = "## Code\n\nBefore.\n\n```rust\nfn main() {}\n```\n\nAfter.";
        let sections = parse(md);
        assert_eq!(sections[0].paragraphs.len(), 2);
        assert!(sections[0].paragraphs[0].text.contains("Before"));
        assert!(sections[0].paragraphs[1].text.contains("After"));
    }

    #[test]
    fn test_decomposer_trait() {
        let d = MarkdownDecomposer;
        let sections = d.decompose("## Hello\n\nWorld.");
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].heading.as_deref(), Some("Hello"));
    }
}
