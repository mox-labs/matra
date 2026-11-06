//! Block input reader: headings and paragraphs the caller supplies, as
//! [`Block`] values or as the JSON array `Format::Blocks` names.

use crate::domain::{self, Block, BlockContent, Error, ParagraphKind, Span};

use super::{Decomposer, Read, ReadBlock};

/// What separates two blocks' texts in the analysed text.
const SEPARATOR: &str = "\n\n";

/// Reads the JSON form of block input: an array of objects, each a
/// heading (`{"kind": "heading", "level": 1, "text": "..."}`) or a
/// paragraph (`{"kind": "paragraph", "text": "..."}`). The schema is
/// `spec/schema/v2/blocks.schema.json`. A `kind` it does not know, a
/// field it does not know and malformed JSON are refused with
/// [`Error::InvalidInput`], naming what was wrong.
pub(crate) struct BlocksDecomposer;

impl Decomposer for BlocksDecomposer {
    fn decompose(&self, input: &[u8]) -> domain::Result<Read> {
        assemble(&parse(input)?)
    }
}

/// One block as JSON. Separate from [`Block`], which is opaque, so the
/// wire form is defined here and nowhere else.
#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum WireBlock {
    Heading { level: u8, text: String },
    Paragraph { text: String },
}

/// Parse the JSON array into blocks.
fn parse(input: &[u8]) -> domain::Result<Vec<Block>> {
    let wire: Vec<WireBlock> = serde_json::from_slice(input).map_err(|e| Error::InvalidInput {
        reason: "blocks_malformed",
        message: format!("block input is not a block array: {e}"),
    })?;
    Ok(wire
        .into_iter()
        .map(|block| match block {
            WireBlock::Heading { level, text } => Block::heading(level, text),
            WireBlock::Paragraph { text } => Block::paragraph(text),
        })
        .collect())
}

/// Join the blocks' texts by a blank line, in order, into the analysed
/// text, and give each block the span of its own text in it. A heading
/// whose level is 0 is refused, naming its position.
pub(crate) fn assemble(blocks: &[Block]) -> domain::Result<Read> {
    let mut text = String::new();
    let mut out = Vec::with_capacity(blocks.len());
    for (index, block) in blocks.iter().enumerate() {
        if index > 0 {
            text.push_str(SEPARATOR);
        }
        let start = text.len();
        match &block.0 {
            BlockContent::Heading { level, text: own } => {
                if *level == 0 {
                    return Err(Error::InvalidInput {
                        reason: "heading_level_zero",
                        message: format!(
                            "block {index} is a heading at level 0; a heading's level is 1 or more"
                        ),
                    });
                }
                text.push_str(own);
                out.push(ReadBlock::heading(
                    usize::from(*level),
                    own.clone(),
                    Span::new(start, text.len()),
                ));
            }
            BlockContent::Paragraph { text: own } => {
                text.push_str(own);
                out.push(ReadBlock::paragraph(
                    ParagraphKind::Paragraph,
                    own.clone(),
                    Span::new(start, text.len()),
                ));
            }
        }
    }
    Ok(Read::new(out, Vec::new()).with_text(text))
}

#[cfg(test)]
mod tests {
    use super::super::BlockKind;
    use super::*;

    #[test]
    fn blocks_join_by_a_blank_line_and_span_their_own_text() {
        let read = assemble(&[
            Block::heading(1, "Field notes"),
            Block::paragraph("The river rose."),
        ])
        .unwrap();
        let text = read.text.unwrap();
        assert_eq!(text, "Field notes\n\nThe river rose.");
        match &read.blocks[1].0 {
            BlockKind::Paragraph { span, .. } => {
                assert_eq!(&text[span.byte_start..span.byte_end], "The river rose.");
            }
            BlockKind::Heading { .. } => panic!("second block is a paragraph"),
        }
    }

    #[test]
    fn json_reads_the_same_blocks() {
        let json = r#"[{"kind":"heading","level":2,"text":"A"},{"kind":"paragraph","text":"B"}]"#;
        let read = BlocksDecomposer.decompose(json.as_bytes()).unwrap();
        assert_eq!(read.text.as_deref(), Some("A\n\nB"));
        assert!(matches!(
            read.blocks[0].0,
            BlockKind::Heading { level: 2, .. }
        ));
    }

    #[test]
    fn an_unknown_kind_is_refused_by_name() {
        let err = BlocksDecomposer
            .decompose(br#"[{"kind":"table","text":"x"}]"#)
            .expect_err("refused");
        assert!(matches!(err, Error::InvalidInput { .. }));
        assert!(err.to_string().contains("table"), "{err}");
    }

    #[test]
    fn an_unknown_field_is_refused() {
        let err = BlocksDecomposer
            .decompose(br#"[{"kind":"paragraph","text":"x","style":"bold"}]"#)
            .expect_err("refused");
        assert!(err.to_string().contains("style"), "{err}");
    }

    #[test]
    fn a_level_zero_heading_is_refused() {
        let err = assemble(&[Block::heading(0, "x")]).expect_err("refused");
        assert!(matches!(err, Error::InvalidInput { .. }));
    }

    #[test]
    fn no_blocks_is_an_empty_text() {
        let read = assemble(&[]).unwrap();
        assert_eq!(read.text.as_deref(), Some(""));
        assert!(read.blocks.is_empty());
    }
}
