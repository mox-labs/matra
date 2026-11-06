//! Structure: how a reader's blocks become the section tree.
//!
//! The one decision this module hides is the shape of the tree: today a
//! flat list of sections, each opened by a heading, with whatever comes
//! before the first heading in a section of its own. A nested tree, or a
//! different rule for headings, changes this file and no other.

use crate::decompose::{BlockKind, ReadBlock, TextMap};
use crate::domain::{Paragraph, Section};

/// The sections `blocks` make, and the text map of every paragraph in
/// document order (the order `Document::paragraphs` walks).
///
/// A heading opens a new section at its level. Paragraphs before the
/// first heading go in a section with no heading at level 0, which exists
/// only when there is such a paragraph. A heading with no paragraph after
/// it is still a section.
pub(crate) fn sections(blocks: Vec<ReadBlock>) -> (Vec<Section>, Vec<TextMap>) {
    let mut sections = Vec::new();
    let mut maps = Vec::new();
    let mut current = Section::new(None, 0, None, Vec::new());
    for block in blocks {
        match block.0 {
            BlockKind::Heading { level, text, span } => {
                if current.heading.is_some() || !current.paragraphs.is_empty() {
                    sections.push(current);
                }
                current = Section::new(Some(text), level, Some(span), Vec::new());
            }
            BlockKind::Paragraph {
                kind,
                text,
                span,
                map,
            } => {
                current.paragraphs.push(Paragraph::new(text, kind, span));
                maps.push(map);
            }
        }
    }
    if current.heading.is_some() || !current.paragraphs.is_empty() {
        sections.push(current);
    }
    (sections, maps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ParagraphKind, Span};

    fn paragraph(text: &str) -> ReadBlock {
        ReadBlock::mapped(
            ParagraphKind::Paragraph,
            text.to_string(),
            Span::default(),
            TextMap::default(),
        )
    }

    fn heading(level: usize, text: &str) -> ReadBlock {
        ReadBlock::heading(level, text, Span::new(1, 2))
    }

    #[test]
    fn paragraphs_before_a_heading_get_a_section_of_their_own() {
        let (sections, maps) = sections(vec![
            paragraph("intro"),
            heading(1, "Title"),
            paragraph("body"),
        ]);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].heading, None);
        assert_eq!(sections[0].level, 0);
        assert_eq!(sections[0].span, None);
        assert_eq!(sections[1].heading.as_deref(), Some("Title"));
        assert_eq!(sections[1].span, Some(Span::new(1, 2)));
        assert_eq!(maps.len(), 2);
    }

    #[test]
    fn a_heading_alone_is_a_section() {
        let (sections, _) = sections(vec![heading(2, "A"), heading(2, "B")]);
        assert_eq!(sections.len(), 2);
        assert!(sections.iter().all(|s| s.paragraphs.is_empty()));
    }

    #[test]
    fn no_blocks_no_sections() {
        let (sections, maps) = sections(Vec::new());
        assert!(sections.is_empty() && maps.is_empty());
    }
}
