//! Markdown reader. CommonMark, read by pulldown-cmark, whose offset
//! iterator gives every event the byte range of the source it came from.

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::domain::{self, ParagraphKind, Skipped, SkippedKind, Span};

use super::{Decomposer, Read, ReadBlock, TextMap};

/// Reads markdown into headings and paragraphs with their spans, and
/// records what it does not analyse.
///
/// - **Read**: ATX and setext headings (a `#` with no space after it,
///   as in `#hashtag`, is text, not a heading), paragraphs, list items
///   (tight or loose) and block quotes. A paragraph's text is its inline
///   text with the markup removed: emphasis, strikethrough, link and
///   image syntax (the link text and the image's alt text stay), the
///   backticks of a code span, inline HTML, task list markers. Escapes and
///   entities are decoded and soft line breaks become `\n`.
/// - **Skipped**, each recorded in `skipped` with its span: tables, fenced
///   and indented code, front matter (YAML `---` or TOML `+++` at the very
///   start), HTML blocks, thematic breaks and link reference definitions.
///   Nothing is dropped silently.
/// - **Spans**: a heading's or paragraph's span covers its source with
///   trailing whitespace excluded, and stretches over the container
///   markup around it (the `>` and list markers on its first line, and a
///   `>` or bare list marker on a line of its own after it), so every byte
///   of the input that is not whitespace lies in exactly one heading,
///   paragraph or skipped span.
///
/// A heading is a heading wherever it appears: `## References` opens a
/// section like any other.
pub(crate) struct MarkdownDecomposer;

impl Decomposer for MarkdownDecomposer {
    fn decompose(&self, input: &str) -> domain::Result<Read> {
        Ok(read(input))
    }
}

/// The CommonMark extensions the reader turns on: the ones whose markup
/// would otherwise reach the parser as text (tables, strikethrough, task
/// list markers, front matter). Smart punctuation stays off, since it
/// would rewrite the text the parser reads.
fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
}

fn read(input: &str) -> Read {
    let mut events = Parser::new_ext(input, options()).into_offset_iter();
    let mut reader = Reader::new(input);
    for (event, range) in events.by_ref() {
        reader.event(event, range);
    }
    for (_, definition) in events.reference_definitions().iter() {
        reader.skip(SkippedKind::LinkDefinition, definition.span.clone());
    }
    reader.recover_duplicate_definitions();
    reader.finish()
}

/// The link reference definitions in `input[from..to]`, read on their
/// own, as ranges of `input`.
fn definitions_in(input: &str, from: usize, to: usize) -> Vec<Range<usize>> {
    let events = Parser::new_ext(&input[from..to], options()).into_offset_iter();
    events
        .reference_definitions()
        .iter()
        .map(|(_, d)| d.span.start + from..d.span.end + from)
        .collect()
}

/// What kind of block is open and collecting inline text.
enum Open {
    Heading {
        level: usize,
        start: usize,
        end: usize,
    },
    /// A `Paragraph` tag, or the text of a tight list item, which
    /// CommonMark gives no paragraph tag: `explicit` is false then, and
    /// `end` grows with each inline event.
    Paragraph {
        kind: ParagraphKind,
        start: usize,
        end: usize,
        explicit: bool,
    },
}

struct Reader<'a> {
    input: &'a str,
    blocks: Vec<ReadBlock>,
    skipped: Vec<Skipped>,
    quote_depth: usize,
    item_depth: usize,
    /// Nesting depth inside a construct being skipped whole; its events
    /// are ignored until it closes.
    skip_depth: usize,
    /// For each open block quote and list item, how many blocks and
    /// skipped constructs existed when it opened, so one that closes
    /// holding none is still read (an empty `-` item, a bare `>`).
    containers: Vec<usize>,
    /// Containers that closed holding nothing, as the empty paragraphs
    /// they become unless a link reference definition (known only once
    /// the input is parsed) turns out to lie inside one.
    empty: Vec<(ParagraphKind, Span)>,
    open: Option<Open>,
    text: String,
    map: TextMap,
}

impl<'a> Reader<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            blocks: Vec::new(),
            skipped: Vec::new(),
            quote_depth: 0,
            item_depth: 0,
            skip_depth: 0,
            containers: Vec::new(),
            empty: Vec::new(),
            open: None,
            text: String::new(),
            map: TextMap::default(),
        }
    }

    fn event(&mut self, event: Event<'_>, range: Range<usize>) {
        if self.skip_depth > 0 {
            match event {
                Event::Start(_) => self.skip_depth += 1,
                Event::End(_) => self.skip_depth -= 1,
                _ => {}
            }
            return;
        }
        match event {
            Event::Start(Tag::Table(_)) => self.skip_block(SkippedKind::Table, range),
            Event::Start(Tag::CodeBlock(_)) => self.skip_block(SkippedKind::Code, range),
            Event::Start(Tag::HtmlBlock) => self.skip_block(SkippedKind::Html, range),
            Event::Start(Tag::MetadataBlock(_)) => {
                self.skip_block(SkippedKind::FrontMatter, range);
            }
            Event::Rule => {
                self.close_item_text();
                self.skip(SkippedKind::ThematicBreak, range);
            }
            Event::Start(Tag::BlockQuote(_)) => {
                self.open_container();
                self.quote_depth += 1;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                self.close_container(range);
                self.quote_depth = self.quote_depth.saturating_sub(1);
            }
            Event::Start(Tag::Item) => {
                self.open_container();
                self.item_depth += 1;
            }
            Event::End(TagEnd::Item) => {
                self.close_container(range);
                self.item_depth = self.item_depth.saturating_sub(1);
            }
            Event::Start(Tag::List(_)) | Event::End(TagEnd::List(_)) => self.close_item_text(),
            Event::Start(Tag::Paragraph) => {
                self.close_item_text();
                self.open = Some(Open::Paragraph {
                    kind: self.kind(),
                    start: range.start,
                    end: range.end,
                    explicit: true,
                });
            }
            Event::Start(Tag::Heading { level, .. }) => {
                self.close_item_text();
                self.open = Some(Open::Heading {
                    level: level as usize,
                    start: range.start,
                    end: range.end,
                });
            }
            Event::End(TagEnd::Paragraph | TagEnd::Heading(_)) => self.close(),
            Event::Text(text) => {
                self.inline(&range);
                let source = &self.input[range.clone()];
                self.push_text(&text, &range, source == &*text);
            }
            Event::Code(code) => {
                self.inline(&range);
                match code_content(&self.input[range.clone()], &code) {
                    Some(offset) => {
                        let start = range.start + offset;
                        self.push_text(&code, &(start..start + code.len()), true);
                    }
                    None => self.push_text(&code, &range, false),
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                self.inline(&range);
                self.push_text("\n", &range, false);
            }
            // Inline markup with no text of its own: emphasis, links,
            // images (their text arrives as Text events), inline HTML,
            // task list markers. Each extends an open item's text.
            _ => self.inline(&range),
        }
    }

    fn open_container(&mut self) {
        self.close_item_text();
        let seen = self.seen();
        self.containers.push(seen);
    }

    /// How many blocks, skipped constructs and empty containers have
    /// been read so far.
    fn seen(&self) -> usize {
        self.blocks.len() + self.skipped.len() + self.empty.len()
    }

    /// Close a block quote or list item. One that held nothing (a list
    /// marker with no text, a `>` alone) is read as an empty paragraph of
    /// its kind, so its markup lies in a span like any other.
    fn close_container(&mut self, range: Range<usize>) {
        self.close_item_text();
        let before = self.containers.pop().unwrap_or(0);
        if self.seen() == before {
            let span = self.trimmed(range.start, range.end);
            self.empty.push((self.kind(), span));
        }
    }

    /// The kind a paragraph opened here takes: inside a block quote at
    /// any depth it is a quote, else inside a list item a list item.
    fn kind(&self) -> ParagraphKind {
        if self.quote_depth > 0 {
            ParagraphKind::Quote
        } else if self.item_depth > 0 {
            ParagraphKind::ListItem
        } else {
            ParagraphKind::Paragraph
        }
    }

    /// An inline event at `range`: opens a tight list item's text when
    /// nothing is open, and stretches the open block over the event. A
    /// task list marker is reported inside its item's paragraph but lies
    /// before the paragraph's own range, so a start can move back too.
    /// The backslash of an escape belongs to no event, so a tight item's
    /// text that opens with an escaped character starts at the backslash.
    fn inline(&mut self, range: &Range<usize>) {
        match &mut self.open {
            None => {
                let escaped = self.input[..range.start].ends_with('\\');
                self.open = Some(Open::Paragraph {
                    kind: self.kind(),
                    start: range.start - usize::from(escaped),
                    end: range.end,
                    explicit: false,
                });
            }
            Some(Open::Paragraph {
                start,
                end,
                explicit,
                ..
            }) => {
                *start = (*start).min(range.start);
                if !*explicit {
                    *end = (*end).max(range.end);
                }
            }
            Some(Open::Heading { start, .. }) => *start = (*start).min(range.start),
        }
    }

    fn push_text(&mut self, text: &str, source: &Range<usize>, verbatim: bool) {
        let at = self.text.len();
        self.text.push_str(text);
        self.map.push(
            at,
            text.len(),
            Span::new(source.start, source.end),
            verbatim,
        );
    }

    /// Close a tight list item's text before a block boundary. An
    /// explicit paragraph or heading closes on its own end tag.
    fn close_item_text(&mut self) {
        if matches!(
            self.open,
            Some(Open::Paragraph {
                explicit: false,
                ..
            })
        ) {
            self.close();
        }
    }

    /// Finish the open block: trim its text, trim its span's trailing
    /// whitespace, and emit it.
    fn close(&mut self) {
        let Some(open) = self.open.take() else {
            return;
        };
        let text = std::mem::take(&mut self.text);
        let map = std::mem::take(&mut self.map);
        let lead = text.len() - text.trim_start().len();
        let trimmed = text.trim();
        match open {
            Open::Heading { level, start, end } => self.blocks.push(ReadBlock::Heading {
                level,
                text: trimmed.to_string(),
                span: self.trimmed(start, end),
            }),
            Open::Paragraph {
                kind, start, end, ..
            } => {
                let span = self.trimmed(start, end);
                let map = if trimmed.is_empty() {
                    TextMap::at(span.byte_start)
                } else {
                    map.window(lead, lead + trimmed.len())
                };
                self.blocks.push(ReadBlock::Paragraph {
                    kind,
                    text: trimmed.to_string(),
                    span,
                    map,
                });
            }
        }
    }

    /// `start..end` with trailing whitespace excluded.
    fn trimmed(&self, start: usize, end: usize) -> Span {
        let kept = self.input[start..end].trim_end().len();
        Span::new(start, start + kept)
    }

    /// Record a construct skipped whole and ignore its events.
    fn skip_block(&mut self, kind: SkippedKind, range: Range<usize>) {
        self.close_item_text();
        self.skip(kind, range);
        self.skip_depth = 1;
    }

    fn skip(&mut self, kind: SkippedKind, range: Range<usize>) {
        let span = self.trimmed(range.start, range.end);
        self.skipped.push(Skipped::new(kind, span));
    }

    /// Record the link reference definitions the parser consumed without
    /// a trace. pulldown-cmark keeps one definition per label, so a
    /// second `[x]: ...` is read and reported nowhere. Each stretch of
    /// the input no span covers that holds more than whitespace and
    /// container markup is read again on its own, and the definitions
    /// found there are recorded; that repeats until none is found, since
    /// a stretch can hold a third definition of the same label.
    fn recover_duplicate_definitions(&mut self) {
        loop {
            let mut covered: Vec<Span> = self
                .blocks
                .iter_mut()
                .map(|b| *b.span_mut())
                .chain(self.skipped.iter().map(|s| s.span))
                .collect();
            covered.sort_by_key(|s| s.byte_start);
            let mut found = Vec::new();
            let mut cursor = 0;
            for span in covered
                .iter()
                .copied()
                .chain([Span::new(self.input.len(), self.input.len())])
            {
                if span.byte_start > cursor {
                    let gap = &self.input[cursor..span.byte_start];
                    if !gap
                        .chars()
                        .all(|c| c.is_whitespace() || is_container_markup(c))
                    {
                        found.extend(definitions_in(self.input, cursor, span.byte_start));
                    }
                }
                cursor = cursor.max(span.byte_end);
            }
            if found.is_empty() {
                return;
            }
            for range in found {
                self.skip(SkippedKind::LinkDefinition, range);
            }
        }
    }

    fn finish(mut self) -> Read {
        self.close();
        self.skipped.sort_by_key(|s| s.span.byte_start);
        for (kind, span) in std::mem::take(&mut self.empty) {
            let holds_a_definition = self
                .skipped
                .iter()
                .any(|s| s.span.byte_start < span.byte_end && span.byte_start < s.span.byte_end);
            if holds_a_definition {
                continue;
            }
            let at = self
                .blocks
                .partition_point(|b| block_start(b) <= span.byte_start);
            self.blocks.insert(
                at,
                ReadBlock::Paragraph {
                    kind,
                    text: String::new(),
                    span,
                    map: TextMap::at(span.byte_start),
                },
            );
        }
        absorb_container_markup(self.input, &mut self.blocks, &mut self.skipped);
        Read {
            text: None,
            blocks: self.blocks,
            skipped: self.skipped,
        }
    }
}

fn block_start(block: &ReadBlock) -> usize {
    match block {
        ReadBlock::Heading { span, .. } | ReadBlock::Paragraph { span, .. } => span.byte_start,
    }
}

/// Where a code span's content starts inside its source (`` `code` ``),
/// when the content is the source byte for byte: the backtick run
/// removed, and the one space CommonMark strips from each end when both
/// are present. `None` when the content differs (a line ending inside a
/// code span becomes a space).
fn code_content(source: &str, code: &str) -> Option<usize> {
    let ticks = source.len() - source.trim_start_matches('`').len();
    let inner = source.get(ticks..source.len().checked_sub(ticks)?)?;
    if inner == code {
        return Some(ticks);
    }
    let padded = inner.strip_prefix(' ')?.strip_suffix(' ')?;
    (padded == code).then_some(ticks + 1)
}

/// The characters of container markup: block quote markers and list
/// markers (bullets, and the digits and delimiter of an ordered marker).
fn is_container_markup(c: char) -> bool {
    matches!(c, '>' | '-' | '+' | '*' | '.' | ')') || c.is_ascii_digit()
}

/// Stretch spans over the container markup between them, so that every
/// byte of the input that is not whitespace lies in a span.
///
/// The markup on the line a block starts on (`> - ` before a list item's
/// text inside a quote) joins that block. Markup on lines of its own (a
/// `>` holding a quote's paragraphs apart, a list marker with no text)
/// joins the block before it, or the block after it when nothing comes
/// before. A gap holding anything other than whitespace and container
/// markup is left alone, so the accounting law's test finds it rather
/// than a span quietly swallowing it.
fn absorb_container_markup(input: &str, blocks: &mut [ReadBlock], skipped: &mut [Skipped]) {
    // Every span, in input order, with where it lives.
    let mut order: Vec<(usize, bool, usize)> = blocks
        .iter_mut()
        .enumerate()
        .map(|(i, b)| (b.span_mut().byte_start, true, i))
        .chain(
            skipped
                .iter()
                .enumerate()
                .map(|(i, s)| (s.span.byte_start, false, i)),
        )
        .collect();
    order.sort_unstable();

    let mut cursor = 0;
    let mut previous: Option<(bool, usize)> = None;
    for &(_, is_block, index) in &order {
        let span = *span_at(blocks, skipped, is_block, index);
        if span.byte_start > cursor {
            let gap = &input[cursor..span.byte_start];
            if gap
                .chars()
                .all(|c| c.is_whitespace() || is_container_markup(c))
            {
                // The part of the gap on the block's own first line.
                let line_start = gap.rfind('\n').map_or(0, |i| i + 1);
                let (before, on_line) = gap.split_at(line_start);
                let mut new_start = span.byte_start;
                if let Some(first) = first_visible(on_line) {
                    new_start = cursor + line_start + first;
                }
                if let Some(last) = last_visible(before) {
                    match previous {
                        Some((was_block, at)) => {
                            let prev = span_at(blocks, skipped, was_block, at);
                            prev.byte_end = prev.byte_end.max(cursor + last);
                        }
                        None => {
                            new_start = cursor + first_visible(before).unwrap_or(0);
                        }
                    }
                }
                span_at(blocks, skipped, is_block, index).byte_start = new_start;
            }
        }
        cursor = cursor.max(span_at(blocks, skipped, is_block, index).byte_end);
        previous = Some((is_block, index));
    }

    // Markup after the last span joins it.
    if let Some((is_block, index)) = previous
        && cursor < input.len()
    {
        let gap = &input[cursor..];
        if gap
            .chars()
            .all(|c| c.is_whitespace() || is_container_markup(c))
            && let Some(last) = last_visible(gap)
        {
            span_at(blocks, skipped, is_block, index).byte_end = cursor + last;
        }
    }
}

fn span_at<'s>(
    blocks: &'s mut [ReadBlock],
    skipped: &'s mut [Skipped],
    is_block: bool,
    index: usize,
) -> &'s mut Span {
    if is_block {
        blocks[index].span_mut()
    } else {
        &mut skipped[index].span
    }
}

/// The byte offset of the first character that is not whitespace.
fn first_visible(s: &str) -> Option<usize> {
    s.char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map(|(i, _)| i)
}

/// The byte offset just past the last character that is not whitespace.
fn last_visible(s: &str) -> Option<usize> {
    s.char_indices()
        .rev()
        .find(|(_, c)| !c.is_whitespace())
        .map(|(i, c)| i + c.len_utf8())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Shape {
        headings: Vec<(usize, String, Span)>,
        paragraphs: Vec<(ParagraphKind, String, Span)>,
        skipped: Vec<(SkippedKind, Span)>,
    }

    fn shape(input: &str) -> Shape {
        let read = read(input);
        let mut headings = Vec::new();
        let mut paragraphs = Vec::new();
        for block in read.blocks {
            match block {
                ReadBlock::Heading { level, text, span } => headings.push((level, text, span)),
                ReadBlock::Paragraph {
                    kind, text, span, ..
                } => paragraphs.push((kind, text, span)),
            }
        }
        Shape {
            headings,
            paragraphs,
            skipped: read.skipped.iter().map(|s| (s.kind, s.span)).collect(),
        }
    }

    fn slice(input: &str, span: Span) -> &str {
        &input[span.byte_start..span.byte_end]
    }

    #[test]
    fn headings_and_paragraphs_carry_their_spans() {
        let md = "## Intro\n\nFirst paragraph.\n\nSecond paragraph.\n\n## Body\n\nBody text.";
        let s = shape(md);
        assert_eq!(s.headings.len(), 2);
        assert_eq!(s.headings[0].1, "Intro");
        assert_eq!(slice(md, s.headings[0].2), "## Intro");
        assert_eq!(s.paragraphs.len(), 3);
        assert_eq!(slice(md, s.paragraphs[1].2), "Second paragraph.");
    }

    #[test]
    fn front_matter_is_skipped_with_its_span() {
        let md = "---\ntitle: Test\n---\n\n## Intro\n\nText.";
        let s = shape(md);
        assert_eq!(s.skipped.len(), 1);
        assert_eq!(s.skipped[0].0, SkippedKind::FrontMatter);
        assert_eq!(slice(md, s.skipped[0].1), "---\ntitle: Test\n---");
    }

    /// Regression: the line scanner dropped every line starting with
    /// `|`, so a table vanished without a trace.
    #[test]
    fn a_table_is_skipped_not_dropped() {
        let md = "Before.\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\nAfter.";
        let s = shape(md);
        assert_eq!(s.paragraphs.len(), 2);
        assert_eq!(s.skipped.len(), 1);
        assert_eq!(s.skipped[0].0, SkippedKind::Table);
        assert_eq!(slice(md, s.skipped[0].1), "| a | b |\n|---|---|\n| 1 | 2 |");
    }

    /// Regression: a `## References` line ended the document, dropping
    /// everything after it, not only the references.
    #[test]
    fn references_is_an_ordinary_heading() {
        let md = "Body.\n\n## References\n\nA cited work.\n\n## Appendix\n\nMore text.";
        let s = shape(md);
        let titles: Vec<&str> = s.headings.iter().map(|h| h.1.as_str()).collect();
        assert_eq!(titles, ["References", "Appendix"]);
        assert_eq!(s.paragraphs.len(), 3);
        assert_eq!(s.paragraphs[2].1, "More text.");
    }

    /// Regression: any line starting with `#` opened a section, so a
    /// hashtag became a heading.
    #[test]
    fn a_hashtag_is_text_not_a_heading() {
        let s = shape("#hashtag starts this line.\n\n# A heading");
        assert_eq!(s.headings.len(), 1);
        assert_eq!(s.headings[0].1, "A heading");
        assert_eq!(s.paragraphs[0].1, "#hashtag starts this line.");
    }

    /// Regression: lines were split with `str::lines` and rejoined with
    /// `\n`, and spans now count bytes of the input, so `\r\n` must not
    /// shift them.
    #[test]
    fn crlf_input_keeps_exact_spans() {
        let md = "# Title\r\n\r\nOne line\r\nand the next.\r\n\r\nSecond.\r\n";
        let s = shape(md);
        assert_eq!(slice(md, s.headings[0].2), "# Title");
        assert_eq!(slice(md, s.paragraphs[0].2), "One line\r\nand the next.");
        assert_eq!(s.paragraphs[0].1, "One line\nand the next.");
        assert_eq!(slice(md, s.paragraphs[1].2), "Second.");
    }

    /// Regression: blockquote text lost its `>` markers by string
    /// surgery. The text the parser reads is still the quote's words, and
    /// the span now covers the markers, so nothing is lost.
    #[test]
    fn a_block_quote_keeps_its_markers_in_its_span() {
        let md = "Normal text.\n\n> This is a quote\n> over two lines.";
        let s = shape(md);
        assert_eq!(s.paragraphs[0].0, ParagraphKind::Paragraph);
        assert_eq!(s.paragraphs[1].0, ParagraphKind::Quote);
        assert_eq!(s.paragraphs[1].1, "This is a quote\nover two lines.");
        assert_eq!(
            slice(md, s.paragraphs[1].2),
            "> This is a quote\n> over two lines."
        );
    }

    #[test]
    fn code_blocks_are_skipped() {
        let md = "## Code\n\nBefore.\n\n```rust\nfn main() {}\n```\n\n    indented\n\nAfter.";
        let s = shape(md);
        assert_eq!(s.paragraphs.len(), 2);
        let kinds: Vec<SkippedKind> = s.skipped.iter().map(|k| k.0).collect();
        assert_eq!(kinds, [SkippedKind::Code, SkippedKind::Code]);
        assert_eq!(slice(md, s.skipped[0].1), "```rust\nfn main() {}\n```");
    }

    #[test]
    fn list_items_are_read_tight_or_loose() {
        let md = "- tight one\n- tight two\n\n1. loose one\n\n2. loose two";
        let s = shape(md);
        let texts: Vec<&str> = s.paragraphs.iter().map(|p| p.1.as_str()).collect();
        assert_eq!(texts, ["tight one", "tight two", "loose one", "loose two"]);
        assert!(s.paragraphs.iter().all(|p| p.0 == ParagraphKind::ListItem));
        assert_eq!(slice(md, s.paragraphs[0].2), "- tight one");
        assert_eq!(slice(md, s.paragraphs[3].2), "2. loose two");
    }

    #[test]
    fn inline_markup_is_stripped_from_the_text() {
        let md = "Some **bold**, *em*, `code`, [a link](https://x.test), ![alt](i.png) and ~~gone~~ &amp; \\*.";
        let s = shape(md);
        assert_eq!(
            s.paragraphs[0].1,
            "Some bold, em, code, a link, alt and gone & *."
        );
        assert_eq!(slice(md, s.paragraphs[0].2), md);
    }

    #[test]
    fn a_text_offset_maps_back_through_the_markup() {
        let md = "Plain **bold** end";
        let read = read(md);
        let ReadBlock::Paragraph { text, map, .. } = &read.blocks[0] else {
            panic!("a paragraph");
        };
        let at = text.find("bold").unwrap();
        let span = map.map(Span::new(at, at + 4));
        assert_eq!(slice(md, span), "bold");
        let code = "Call `run()` now";
        let read = super::read(code);
        let ReadBlock::Paragraph { text, map, .. } = &read.blocks[0] else {
            panic!("a paragraph");
        };
        let at = text.find("run()").unwrap();
        assert_eq!(slice(code, map.map(Span::new(at, at + 5))), "run()");
    }

    #[test]
    fn html_rules_and_link_definitions_are_skipped() {
        let md = "<div>\nraw\n</div>\n\nText [x].\n\n---\n\n[x]: https://x.test";
        let s = shape(md);
        let kinds: Vec<SkippedKind> = s.skipped.iter().map(|k| k.0).collect();
        assert_eq!(
            kinds,
            [
                SkippedKind::Html,
                SkippedKind::ThematicBreak,
                SkippedKind::LinkDefinition
            ]
        );
        assert_eq!(slice(md, s.skipped[2].1), "[x]: https://x.test");
    }

    #[test]
    fn a_marker_only_line_joins_the_block_before_it() {
        let md = "> one\n>\n> two";
        let s = shape(md);
        assert_eq!(slice(md, s.paragraphs[0].2), "> one\n>");
        assert_eq!(slice(md, s.paragraphs[1].2), "> two");
    }

    #[test]
    fn a_list_inside_a_quote_is_a_quote() {
        let s = shape("> - quoted item");
        assert_eq!(s.paragraphs[0].0, ParagraphKind::Quote);
    }

    #[test]
    fn empty_input_reads_nothing() {
        let s = shape("");
        assert!(s.headings.is_empty() && s.paragraphs.is_empty() && s.skipped.is_empty());
    }
}
