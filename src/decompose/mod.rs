//! Reading port. A reader turns one format's bytes into blocks, each
//! with its span in the analysed text, and lists what it skipped.
//!
//! This module holds the [`Decomposer`] trait and what a reader returns,
//! [`Read`] and its [`ReadBlock`]s, built through their constructors so
//! the block vocabulary can grow without breaking a reader. The readers
//! matra ships live in submodules (markdown.rs, plain.rs, blocks.rs) and
//! are registered by the composition root; a reader of your own reaches
//! the pipeline through `Engine::with_reader`.

pub(crate) mod blocks;
pub(crate) mod markdown;
pub(crate) mod plain;

use crate::domain::{self, Format, ParagraphKind, Skipped, Span};

/// A reader: one format's bytes in, the document's headings and
/// paragraphs out, each with where it lies in the analysed text.
///
/// The engine looks up the reader registered for the input's
/// [`Format`], hands it the input's bytes, builds the section tree from
/// the blocks it returns, and parses each paragraph that is not a quote.
///
/// # What `decompose` returns
///
/// A [`Read`]: the blocks in document order, and the constructs the
/// reader saw and did not analyse (a table, a code block) as
/// [`Skipped`] values in document order.
///
/// - The analysed text is `input` decoded as UTF-8, unless the reader
///   names another with [`Read::with_text`]: the text it extracted from a
///   binary format, or a text it assembled. Every span points into the
///   analysed text, in UTF-8 bytes.
/// - Every span lies within the analysed text, starts at or before its
///   end, and falls on character boundaries.
/// - Blocks are in document order, and no two spans among the blocks and
///   the skipped constructs overlap.
/// - A heading's level is 1 or more. Its text is what the heading says,
///   markers dropped; its span covers the heading, markers included.
/// - A paragraph's text is the analysed text at its span, byte for byte
///   ([`ReadBlock::paragraph`]). A reader that needs a paragraph's text
///   to differ from its source (markup dropped, an entity decoded) names
///   the extracted text as the analysed text instead.
///
/// The engine checks every rule above before a block reaches a
/// [`domain::Document`], and fails the document with
/// [`domain::Error::ParseFailed`] when one breaks, its `reason` naming
/// the check (`block_span_invalid`, `block_span_order`,
/// `heading_level` or `paragraph_text_mismatch`) and its `cause` the
/// block. A conforming reader is one whose output passes those checks on
/// every input it accepts.
///
/// # Errors
///
/// A reader of a text syntax is total: malformed markup is read as best
/// it can be. A failure is an `Err`, never a panic:
///
/// - [`domain::Error::UnsupportedFormat`] with `format` `None` when the
///   bytes are not text the reader reads (not UTF-8, for a text format).
/// - [`domain::Error::InvalidInput`] when the input does not have the
///   shape its format names (block input that is not a block array),
///   with a `reason` naming the check.
/// - [`domain::Error::ParseFailed`], with `reason` `None`, when the
///   reader cannot read input of its format (a PDF its extractor
///   cannot open).
///
/// A reader does no I/O of its own: the bytes are already read.
///
/// # Threads
///
/// `Send`, because an engine holds its readers and moves to a worker
/// thread.
pub trait Decomposer: Send {
    /// Read `input` into blocks and skipped constructs, satisfying the
    /// contract above.
    fn decompose(&self, input: &[u8]) -> domain::Result<Read>;
}

/// What a reader produces from one input: built with [`Read::new`], and
/// [`Read::with_text`] when the analysed text is not the input itself.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Read {
    /// The analysed text, when it is not the input itself. `None` means
    /// the input, decoded as UTF-8, is the analysed text.
    pub(crate) text: Option<String>,
    /// Headings and paragraphs, in document order.
    pub(crate) blocks: Vec<ReadBlock>,
    /// What the reader saw and did not analyse, in document order.
    pub(crate) skipped: Vec<Skipped>,
}

impl Read {
    /// The headings and paragraphs of an input, in document order, and
    /// the constructs the reader skipped, in document order. Every span
    /// points into the input decoded as UTF-8, unless
    /// [`Read::with_text`] names another analysed text.
    ///
    /// ```
    /// use matra::decompose::{Read, ReadBlock};
    /// use matra::domain::{ParagraphKind, Span};
    /// let read = Read::new(
    ///     vec![
    ///         ReadBlock::heading(1, "Notes", Span::new(0, 7)),
    ///         ReadBlock::paragraph(ParagraphKind::Paragraph, "The river rose.", Span::new(9, 24)),
    ///     ],
    ///     Vec::new(),
    /// );
    /// ```
    pub fn new(blocks: Vec<ReadBlock>, skipped: Vec<Skipped>) -> Read {
        Read {
            text: None,
            blocks,
            skipped,
        }
    }

    /// The same read, with `text` as the analysed text: what the reader
    /// extracted from a format that is not text, or assembled. Every span
    /// then points into `text`, and the document's provenance records its
    /// length and digest.
    #[must_use]
    pub fn with_text(mut self, text: impl Into<String>) -> Read {
        self.text = Some(text.into());
        self
    }
}

/// One heading or paragraph a reader emits: built with
/// [`ReadBlock::heading`] or [`ReadBlock::paragraph`], and opaque, so a
/// new kind of block is an additive change.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ReadBlock(pub(crate) BlockKind);

/// What a [`ReadBlock`] holds. Crate-private, so the vocabulary can grow
/// and a paragraph can carry the map of a text that is not a slice of
/// the analysed text, which only matra's own markdown reader builds.
#[derive(Debug, Clone)]
pub(crate) enum BlockKind {
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
    /// A heading at `level` (1 for a top-level heading) saying `text`,
    /// lying at `span` in the analysed text, markers included. A heading
    /// opens a section; its text is not parsed.
    pub fn heading(level: usize, text: impl Into<String>, span: Span) -> ReadBlock {
        ReadBlock(BlockKind::Heading {
            level,
            text: text.into(),
            span,
        })
    }

    /// A paragraph of `kind` whose text is `text`, the analysed text at
    /// `span` byte for byte. The parser reads `text`, and the engine maps
    /// each token's span from it into the analysed text. A
    /// [`ParagraphKind::Quote`] is kept and not parsed.
    pub fn paragraph(kind: ParagraphKind, text: impl Into<String>, span: Span) -> ReadBlock {
        let text = text.into();
        let map = TextMap::verbatim(span.byte_start, text.len());
        ReadBlock(BlockKind::Paragraph {
            kind,
            text,
            span,
            map,
        })
    }

    /// A paragraph whose text maps back to the analysed text through
    /// `map`, for a reader that drops markup from the text it parses.
    pub(crate) fn mapped(kind: ParagraphKind, text: String, span: Span, map: TextMap) -> ReadBlock {
        ReadBlock(BlockKind::Paragraph {
            kind,
            text,
            span,
            map,
        })
    }

    /// The block's span, to adjust in place.
    pub(crate) fn span_mut(&mut self) -> &mut Span {
        match &mut self.0 {
            BlockKind::Heading { span, .. } | BlockKind::Paragraph { span, .. } => span,
        }
    }

    /// The block's span.
    pub(crate) fn span(&self) -> Span {
        match &self.0 {
            BlockKind::Heading { span, .. } | BlockKind::Paragraph { span, .. } => *span,
        }
    }
}

/// The text of `input`, for a reader of a text format: bytes that are
/// not UTF-8 are not text it reads, and fail as an unsupported format
/// rather than as a read error, since nothing went wrong reading them.
pub(crate) fn text_of(input: &[u8]) -> domain::Result<&str> {
    std::str::from_utf8(input).map_err(|_| domain::Error::UnsupportedFormat {
        format: None,
        feature: None,
    })
}

/// Format-indexed table of readers. Lookup is the partial step; which
/// formats an engine reads is a question asked of a value, and a miss is
/// `Error::UnsupportedFormat`. Population happens in the composition
/// root, the only place that names the adapters, and through
/// `Engine::with_reader`.
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

    /// What is wrong with this map as the map of a paragraph whose text
    /// is `text` and which lies at `span` in `analysed`, or `None` when
    /// nothing is: every piece lies within `text` and comes from within
    /// `span`, and a verbatim piece's text is the analysed text it came
    /// from, byte for byte. A map that holds maps every token span inside
    /// the paragraph's span.
    pub(crate) fn mismatch(&self, text: &str, analysed: &str, span: Span) -> Option<String> {
        let inside = |s: Span| span.byte_start <= s.byte_start && s.byte_end <= span.byte_end;
        if !inside(Span::new(self.origin, self.origin)) {
            return Some(format!(
                "its empty text maps to byte {}, outside its span",
                self.origin
            ));
        }
        for piece in &self.pieces {
            let end = piece.text_start + piece.len;
            let Some(own) = text.get(piece.text_start..end) else {
                return Some(format!(
                    "bytes {}..{end} of its text are not a slice of the text",
                    piece.text_start
                ));
            };
            if piece.source.byte_start > piece.source.byte_end || !inside(piece.source) {
                return Some(format!(
                    "bytes {}..{end} of its text map to {}..{}, outside its span",
                    piece.text_start, piece.source.byte_start, piece.source.byte_end
                ));
            }
            let source = analysed.get(piece.source.byte_start..piece.source.byte_end);
            if piece.verbatim && source != Some(own) {
                return Some(format!(
                    "its text {own:?} at {}..{end} is not the analysed text at {}..{}",
                    piece.text_start, piece.source.byte_start, piece.source.byte_end
                ));
            }
        }
        None
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
        fn decompose(&self, _input: &[u8]) -> domain::Result<Read> {
            Ok(Read::new(
                vec![ReadBlock::heading(1, self.0, Span::default())],
                Vec::new(),
            ))
        }
    }

    fn marker_of(d: &dyn Decomposer) -> String {
        match d.decompose(b"").unwrap().blocks.into_iter().next() {
            Some(ReadBlock(BlockKind::Heading { text, .. })) => text,
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
