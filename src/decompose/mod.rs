//! Reading port. A reader turns one format's text into blocks, each with
//! its span in the analysed text, and lists what it skipped.
//!
//! This module holds the [`Decomposer`] trait, the block vocabulary
//! readers emit ([`ReadBlock`]), and the map from a block's text back to
//! the analysed text ([`TextMap`]). Adapters live in submodules:
//! markdown.rs, plain.rs, blocks.rs. All of it is crate-private: a caller
//! hands in an `Input`, the format decides which reader runs, and the
//! block vocabulary can grow without touching a published signature.

pub(crate) mod blocks;
pub(crate) mod markdown;
pub(crate) mod plain;

use crate::domain::{self, Format, ParagraphKind, Skipped, Span};

/// What a reader produces from one input.
pub(crate) struct Read {
    /// The analysed text, when it is not the input itself. Block input is
    /// the one case today: its analysed text is the blocks' texts joined
    /// by a blank line. `None` means the input is the analysed text, and
    /// every span points into it.
    pub(crate) text: Option<String>,
    /// Headings and paragraphs, in document order.
    pub(crate) blocks: Vec<ReadBlock>,
    /// What the reader saw and did not analyse, in document order.
    pub(crate) skipped: Vec<Skipped>,
}

/// One block a reader emits. The richer vocabulary stays here, private,
/// so a new kind is a change to the readers and the structure step only.
pub(crate) enum ReadBlock {
    /// A heading, which opens a section.
    Heading {
        /// 1 or more.
        level: usize,
        /// The heading's text, normalized.
        text: String,
        /// The heading in the analysed text, markers included.
        span: Span,
    },
    /// A paragraph of prose, a quote or a list item's text.
    Paragraph {
        /// The kind of block it was read from.
        kind: ParagraphKind,
        /// The text the parser reads.
        text: String,
        /// The paragraph in the analysed text.
        span: Span,
        /// How offsets in `text` map back to the analysed text.
        map: TextMap,
    },
}

impl ReadBlock {
    /// The block's span, to adjust in place.
    pub(crate) fn span_mut(&mut self) -> &mut Span {
        match self {
            ReadBlock::Heading { span, .. } | ReadBlock::Paragraph { span, .. } => span,
        }
    }
}

/// Reads one format. Postcondition: every span the reader returns lies
/// within the analysed text, and spans of distinct blocks and skipped
/// constructs do not overlap.
pub(crate) trait Decomposer: Send {
    /// Read `input` into blocks and skipped constructs. A reader of a
    /// text syntax is total: malformed markup is read as best it can be.
    /// A reader of a structured format (block input as JSON) fails with
    /// [`domain::Error::InvalidInput`] on input that does not parse.
    fn decompose(&self, input: &str) -> domain::Result<Read>;
}

/// Format-indexed table of readers. Lookup is the partial step; which
/// formats a build reads is a question asked of a value, and a miss is
/// `Error::UnsupportedFormat`. Population happens in the composition
/// root, the only place that names the adapters.
///
/// Backed by a `Vec` keyed on [`Format`] equality. The table holds a
/// handful of entries, so a linear lookup is not a cost worth a `Hash`
/// bound on a `#[non_exhaustive]` enum.
#[derive(Default)]
pub(crate) struct Decomposers {
    entries: Vec<(Format, Box<dyn Decomposer>)>,
}

impl Decomposers {
    /// An empty table: every format is unsupported.
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Register `decomposer` for `format`, replacing any existing entry
    /// for that format.
    #[must_use]
    pub(crate) fn with(mut self, format: Format, decomposer: Box<dyn Decomposer>) -> Self {
        if let Some(slot) = self.entries.iter_mut().find(|(f, _)| *f == format) {
            slot.1 = decomposer;
        } else {
            self.entries.push((format, decomposer));
        }
        self
    }

    /// The reader registered for `format`, if any.
    pub(crate) fn get(&self, format: &Format) -> Option<&dyn Decomposer> {
        self.entries
            .iter()
            .find(|(f, _)| f == format)
            .map(|(_, d)| d.as_ref())
    }
}

/// How a block's text maps back to the analysed text.
///
/// The text a reader hands the parser is not always a slice of the
/// analysed text: a markdown paragraph drops its inline markup and
/// decodes entities. The map records, for each piece of the text, where
/// it came from. A verbatim piece maps byte for byte; an opaque piece (a
/// decoded entity, a line break that became `\n`) maps as a whole, so an
/// offset inside it maps to its start or its end. A span mapped through
/// it is exact for verbatim text and widens to whole pieces otherwise,
/// never to a position the text did not come from.
#[derive(Debug, Clone, Default)]
pub(crate) struct TextMap {
    /// Where an empty text maps: the block's own start.
    origin: usize,
    /// Pieces in text order, contiguous and covering the whole text.
    pieces: Vec<Piece>,
}

#[derive(Debug, Clone, Copy)]
struct Piece {
    /// Where the piece starts in the text.
    text_start: usize,
    /// Its length in the text.
    len: usize,
    /// Where it came from in the analysed text.
    source: Span,
    /// Whether the text equals the source byte for byte.
    verbatim: bool,
}

impl TextMap {
    /// The map of a text that is the slice of the analysed text starting
    /// at `start`, `len` bytes long.
    pub(crate) fn verbatim(start: usize, len: usize) -> Self {
        let mut map = Self {
            origin: start,
            pieces: Vec::new(),
        };
        map.push(0, len, Span::new(start, start + len), true);
        map
    }

    /// An empty map whose empty text sits at `origin`.
    pub(crate) fn at(origin: usize) -> Self {
        Self {
            origin,
            pieces: Vec::new(),
        }
    }

    /// Append the piece of text at `text_start`, `len` bytes long, that
    /// came from `source`. Empty pieces are not recorded.
    pub(crate) fn push(&mut self, text_start: usize, len: usize, source: Span, verbatim: bool) {
        if len == 0 {
            return;
        }
        self.pieces.push(Piece {
            text_start,
            len,
            source,
            verbatim,
        });
    }

    /// The map of the window `[from, to)` of the text, re-based so the
    /// window starts at 0. Pieces outside it are dropped, and pieces cut
    /// by it keep their verbatim offsets.
    pub(crate) fn window(&self, from: usize, to: usize) -> Self {
        let mut out = Self::at(self.origin);
        for piece in &self.pieces {
            let start = piece.text_start.max(from);
            let end = (piece.text_start + piece.len).min(to);
            if start >= end {
                continue;
            }
            let source = if piece.verbatim {
                let offset = piece.source.byte_start + (start - piece.text_start);
                Span::new(offset, offset + (end - start))
            } else {
                piece.source
            };
            out.push(start - from, end - start, source, piece.verbatim);
        }
        if let Some(first) = out.pieces.first() {
            out.origin = first.source.byte_start;
        }
        out
    }

    /// The span of the analysed text that `local`, a span of the text,
    /// came from.
    pub(crate) fn map(&self, local: Span) -> Span {
        let start = self.map_start(local.byte_start);
        if local.byte_end <= local.byte_start {
            return Span::new(start, start);
        }
        let end = self.map_end(local.byte_end).max(start);
        Span::new(start, end)
    }

    /// Where the byte at `at` of the text came from.
    fn map_start(&self, at: usize) -> usize {
        for piece in &self.pieces {
            if at < piece.text_start + piece.len {
                if at < piece.text_start {
                    return piece.source.byte_start;
                }
                return if piece.verbatim {
                    piece.source.byte_start + (at - piece.text_start)
                } else {
                    piece.source.byte_start
                };
            }
        }
        self.pieces
            .last()
            .map_or(self.origin, |p| p.source.byte_end)
    }

    /// Where the text up to `at` (exclusive) ends in the analysed text.
    fn map_end(&self, at: usize) -> usize {
        let mut end = self.origin;
        for piece in &self.pieces {
            if piece.text_start >= at {
                break;
            }
            end = if piece.verbatim && at < piece.text_start + piece.len {
                piece.source.byte_start + (at - piece.text_start)
            } else {
                piece.source.byte_end
            };
        }
        end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Marker(&'static str);
    impl Decomposer for Marker {
        fn decompose(&self, _text: &str) -> domain::Result<Read> {
            Ok(Read {
                text: None,
                blocks: vec![ReadBlock::Heading {
                    level: 1,
                    text: self.0.to_string(),
                    span: Span::default(),
                }],
                skipped: Vec::new(),
            })
        }
    }

    fn marker_of(d: &dyn Decomposer) -> String {
        match d.decompose("").unwrap().blocks.into_iter().next() {
            Some(ReadBlock::Heading { text, .. }) => text,
            _ => String::new(),
        }
    }

    #[test]
    fn empty_table_supports_nothing() {
        let table = Decomposers::new();
        assert!(table.get(&Format::Markdown).is_none());
        assert!(table.get(&Format::PlainText).is_none());
    }

    #[test]
    fn lookup_finds_the_registered_decomposer() {
        let table = Decomposers::new()
            .with(Format::Markdown, Box::new(Marker("md")))
            .with(Format::PlainText, Box::new(Marker("plain")));
        assert_eq!(marker_of(table.get(&Format::Markdown).unwrap()), "md");
        assert_eq!(marker_of(table.get(&Format::PlainText).unwrap()), "plain");
        assert!(table.get(&Format::Pdf).is_none());
    }

    #[test]
    fn with_replaces_on_duplicate_key() {
        let table = Decomposers::new()
            .with(Format::Markdown, Box::new(Marker("first")))
            .with(Format::Markdown, Box::new(Marker("second")));
        assert_eq!(marker_of(table.get(&Format::Markdown).unwrap()), "second");
    }

    #[test]
    fn a_verbatim_map_shifts_by_its_start() {
        let map = TextMap::verbatim(10, 5);
        assert_eq!(map.map(Span::new(1, 4)), Span::new(11, 14));
        assert_eq!(map.map(Span::new(0, 5)), Span::new(10, 15));
    }

    /// `a **b**` read as `a b`: the `a ` piece is verbatim at 0, `b` at
    /// 4. A token spanning both covers the markup between them.
    #[test]
    fn a_map_skips_markup_between_pieces() {
        let mut map = TextMap::at(0);
        map.push(0, 2, Span::new(0, 2), true);
        map.push(2, 1, Span::new(4, 5), true);
        assert_eq!(map.map(Span::new(2, 3)), Span::new(4, 5));
        assert_eq!(map.map(Span::new(0, 3)), Span::new(0, 5));
    }

    /// `&amp;` read as `&`: an opaque piece maps as a whole.
    #[test]
    fn an_opaque_piece_maps_whole() {
        let mut map = TextMap::at(0);
        map.push(0, 2, Span::new(0, 2), true);
        map.push(2, 1, Span::new(2, 7), false);
        assert_eq!(map.map(Span::new(2, 3)), Span::new(2, 7));
        assert_eq!(map.map(Span::new(0, 1)), Span::new(0, 1));
    }

    #[test]
    fn a_window_rebases_and_keeps_offsets() {
        let map = TextMap::verbatim(100, 10).window(2, 8);
        assert_eq!(map.map(Span::new(0, 6)), Span::new(102, 108));
    }

    #[test]
    fn an_empty_map_maps_to_its_origin() {
        let map = TextMap::at(42);
        assert_eq!(map.map(Span::new(0, 0)), Span::new(42, 42));
    }
}
