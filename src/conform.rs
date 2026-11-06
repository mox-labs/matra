//! Conformance: the engine's checks on what a reader and a parser hand
//! back, run once, before anything they return reaches a `Document`.
//!
//! The decision this module hides is what counts as well-formed provider
//! output. A reader and a parser are ports anyone can implement, so the
//! engine trusts neither: every rule their contracts state is checked
//! here, and a break fails the document with `Error::ParseFailed`, its
//! `reason` a stable name for the check and its `cause` the block or the
//! token. Downstream code (the section tree, the span mapping, the tree
//! walks, the metrics) can then read provider output without checking it
//! again. Both checks are single passes, linear in what they read.

use crate::decompose::{BlockKind, Read};
use crate::domain::{Error, Result, Span, Token, form_end};

/// The failure for a broken check: `reason` names it, `cause` says where.
fn broken(reason: &'static str, cause: String) -> Error {
    Error::ParseFailed {
        reason: Some(reason),
        cause,
    }
}

/// Whether `span` is a well-formed span of `text`: in order, inside it,
/// and on character boundaries.
fn fits(text: &str, span: Span) -> bool {
    span.byte_start <= span.byte_end && text.get(span.byte_start..span.byte_end).is_some()
}

/// Check what a reader returned against `text`, the analysed text.
///
/// | `reason` | Check |
/// |---|---|
/// | `block_span_invalid` | a block's or a skipped construct's span is reversed, runs past the analysed text, or splits a character |
/// | `block_span_order` | blocks out of document order, or two spans among the blocks and skipped constructs overlap |
/// | `heading_level` | a heading at level 0 |
/// | `paragraph_text_mismatch` | a paragraph's text is not the analysed text its span covers |
pub(crate) fn read(text: &str, read: &Read) -> Result<()> {
    // Each span with what it belongs to, the block (`true`) or skipped
    // construct and its 1-based number, named only if a check fails.
    let mut spans: Vec<(Span, bool, usize)> =
        Vec::with_capacity(read.blocks.len() + read.skipped.len());
    let mut previous: Option<Span> = None;
    for (index, block) in read.blocks.iter().enumerate() {
        let n = index + 1;
        let span = block.span();
        if !fits(text, span) {
            return Err(broken(
                "block_span_invalid",
                format!(
                    "block {n} has span {}..{}, which is not a span of the {}-byte analysed text",
                    span.byte_start,
                    span.byte_end,
                    text.len()
                ),
            ));
        }
        if let Some(prior) = previous
            && span.byte_start < prior.byte_start
        {
            return Err(broken(
                "block_span_order",
                format!(
                    "block {n} starts at byte {}, before block {index}, which starts at {}",
                    span.byte_start, prior.byte_start
                ),
            ));
        }
        previous = Some(span);
        match &block.0 {
            BlockKind::Heading { level, .. } => {
                if *level == 0 {
                    return Err(broken(
                        "heading_level",
                        format!(
                            "block {n} is a heading at level 0; a heading's level is 1 or more"
                        ),
                    ));
                }
            }
            BlockKind::Paragraph { text: own, map, .. } => {
                if let Some(wrong) = map.mismatch(own, text, span) {
                    return Err(broken(
                        "paragraph_text_mismatch",
                        format!("block {n}, a paragraph: {wrong}"),
                    ));
                }
            }
        }
        spans.push((span, true, n));
    }
    for (index, skipped) in read.skipped.iter().enumerate() {
        let span = skipped.span;
        if !fits(text, span) {
            return Err(broken(
                "block_span_invalid",
                format!(
                    "skipped construct {} has span {}..{}, which is not a span of the {}-byte analysed text",
                    index + 1,
                    span.byte_start,
                    span.byte_end,
                    text.len()
                ),
            ));
        }
        spans.push((span, false, index + 1));
    }
    spans.sort_by_key(|(span, _, _)| (span.byte_start, span.byte_end));
    let name = |is_block: bool, n: usize| {
        if is_block {
            format!("block {n}")
        } else {
            format!("skipped construct {n}")
        }
    };
    for pair in spans.windows(2) {
        let ((first, a_block, a), (second, b_block, b)) = (pair[0], pair[1]);
        if second.byte_start < first.byte_end {
            return Err(broken(
                "block_span_order",
                format!(
                    "{} ({}..{}) and {} ({}..{}) overlap",
                    name(a_block, a),
                    first.byte_start,
                    first.byte_end,
                    name(b_block, b),
                    second.byte_start,
                    second.byte_end
                ),
            ));
        }
    }
    Ok(())
}

/// Check what a parser returned for paragraph `paragraph` (1-based, in
/// document order), whose text is `text`.
///
/// | `reason` | Check |
/// |---|---|
/// | `token_id_sequence` | a sentence's ids are not `1..=n` in order |
/// | `head_out_of_range` | a head is neither `0` nor a token of the sentence |
/// | `root_count` | a sentence has no token whose head is `0`, or more than one |
/// | `head_cycle` | following heads from a token never reaches the root |
/// | `token_span_invalid` | a span is reversed, runs past the paragraph, or splits a character |
/// | `token_span_order` | a span starts before the previous token's span ends |
/// | `token_form_mismatch` | the paragraph text at a span is not the token's form |
pub(crate) fn parse(paragraph: usize, text: &str, sentences: &[Vec<Token>]) -> Result<()> {
    let mut cursor = 0;
    for (index, tokens) in sentences.iter().enumerate() {
        let at = |token: &Token| {
            format!(
                "paragraph {paragraph}, sentence {}, token {} ({:?})",
                index + 1,
                token.id,
                token.text
            )
        };
        let n = tokens.len();
        for (position, token) in tokens.iter().enumerate() {
            if token.id != position + 1 {
                return Err(broken(
                    "token_id_sequence",
                    format!(
                        "{}: id {} at position {}; ids run 1..={n} in order",
                        at(token),
                        token.id,
                        position + 1
                    ),
                ));
            }
            if token.head > n {
                return Err(broken(
                    "head_out_of_range",
                    format!(
                        "{}: head {} is not 0 or a token of the sentence (1..={n})",
                        at(token),
                        token.head
                    ),
                ));
            }
        }
        let roots = tokens.iter().filter(|t| t.head == 0).count();
        if roots != 1 {
            return Err(broken(
                "root_count",
                format!(
                    "paragraph {paragraph}, sentence {}: {roots} tokens have head 0; a sentence has exactly one root",
                    index + 1
                ),
            ));
        }
        if let Some(token) = on_a_cycle(tokens) {
            return Err(broken(
                "head_cycle",
                format!("{}: following its heads never reaches the root", at(token)),
            ));
        }
        for token in tokens {
            let span = token.span;
            if !fits(text, span) {
                return Err(broken(
                    "token_span_invalid",
                    format!(
                        "{}: span {}..{} is not a span of the {}-byte paragraph",
                        at(token),
                        span.byte_start,
                        span.byte_end,
                        text.len()
                    ),
                ));
            }
            if span.byte_start < cursor {
                return Err(broken(
                    "token_span_order",
                    format!(
                        "{}: span {}..{} starts before the previous token's ends, at {cursor}",
                        at(token),
                        span.byte_start,
                        span.byte_end
                    ),
                ));
            }
            if form_end(text, span.byte_start, &token.text) != Some(span.byte_end) {
                return Err(broken(
                    "token_form_mismatch",
                    format!(
                        "{}: the paragraph at {}..{} reads {:?}",
                        at(token),
                        span.byte_start,
                        span.byte_end,
                        &text[span.byte_start..span.byte_end]
                    ),
                ));
            }
            cursor = span.byte_end;
        }
    }
    Ok(())
}

/// A token whose heads never reach the root, if any. Ids are `1..=n` and
/// every head is in range, so heads index the slice directly. Each token
/// is settled once: a walk stops at a token already known to reach the
/// root, and meeting a token on its own walk is a cycle. A visited set,
/// not a depth ceiling, so a well-formed tree of any depth passes.
fn on_a_cycle(tokens: &[Token]) -> Option<&Token> {
    #[derive(Clone, Copy, PartialEq)]
    enum State {
        Unseen,
        OnWalk,
        Rooted,
    }
    let mut state = vec![State::Unseen; tokens.len()];
    let mut walk = Vec::new();
    for start in 0..tokens.len() {
        let mut current = start;
        loop {
            match state[current] {
                State::Rooted => break,
                State::OnWalk => return Some(&tokens[start]),
                State::Unseen => {}
            }
            state[current] = State::OnWalk;
            walk.push(current);
            match tokens[current].head {
                0 => break,
                head => current = head - 1,
            }
        }
        for settled in walk.drain(..) {
            state[settled] = State::Rooted;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decompose::ReadBlock;
    use crate::domain::{ParagraphKind, Skipped, SkippedKind};

    fn token(id: usize, head: usize, form: &str, start: usize) -> Token {
        Token::builder(
            id,
            form.to_string(),
            form.to_string(),
            "X".to_string(),
            head,
            "dep".to_string(),
        )
        .span(Span::new(start, start + form.len()))
        .build()
    }

    fn reason(result: Result<()>) -> Option<&'static str> {
        match result {
            Ok(()) => None,
            Err(Error::ParseFailed { reason, .. }) => reason,
            Err(other) => panic!("expected ParseFailed, got {other:?}"),
        }
    }

    /// "a b c": a chain c -> b -> a, a the root.
    fn chain() -> Vec<Token> {
        vec![
            token(1, 0, "a", 0),
            token(2, 1, "b", 2),
            token(3, 2, "c", 4),
        ]
    }

    #[test]
    fn a_well_formed_parse_passes() {
        assert_eq!(reason(parse(1, "a b c", &[chain()])), None);
        assert_eq!(reason(parse(1, "", &[])), None);
    }

    #[test]
    fn each_parse_check_names_itself() {
        let mut ids = chain();
        ids[1].id = 3;
        let mut head = chain();
        head[2].head = 4;
        let mut two_roots = chain();
        two_roots[2].head = 0;
        let mut cycle = chain();
        cycle[1].head = 3;
        let mut past = chain();
        past[2].span = Span::new(4, 9);
        let mut order = chain();
        order[2].span = Span::new(1, 2);
        let mut form = chain();
        form[2].text = "z".to_string();
        for (tokens, expected) in [
            (ids, "token_id_sequence"),
            (head, "head_out_of_range"),
            (two_roots, "root_count"),
            (Vec::new(), "root_count"),
            (cycle, "head_cycle"),
            (past, "token_span_invalid"),
            (order, "token_span_order"),
            (form, "token_form_mismatch"),
        ] {
            assert_eq!(reason(parse(1, "a b c", &[tokens])), Some(expected));
        }
    }

    /// A self-loop is a cycle of one, and a cycle away from the root is
    /// found even when the root's own chain is fine.
    #[test]
    fn a_cycle_anywhere_is_found() {
        let mut selfloop = chain();
        selfloop[2].head = 3;
        assert_eq!(reason(parse(1, "a b c", &[selfloop])), Some("head_cycle"));
        let text = "a b c d";
        let tokens = vec![
            token(1, 0, "a", 0),
            token(2, 1, "b", 2),
            token(3, 4, "c", 4),
            token(4, 3, "d", 6),
        ];
        assert_eq!(reason(parse(1, text, &[tokens])), Some("head_cycle"));
    }

    /// No depth ceiling: a chain ten thousand tokens deep is a tree.
    #[test]
    fn a_deep_tree_is_not_a_cycle() {
        let n = 10_000;
        let text = "w ".repeat(n);
        let tokens: Vec<Token> = (1..=n).map(|i| token(i, i - 1, "w", 2 * (i - 1))).collect();
        assert_eq!(reason(parse(1, &text, &[tokens])), None);
    }

    /// Spans run in order across sentences, not only within one.
    #[test]
    fn spans_are_ordered_across_sentences() {
        let first = vec![token(1, 0, "b", 2)];
        let second = vec![token(1, 0, "a", 0)];
        assert_eq!(
            reason(parse(1, "a b", &[first, second])),
            Some("token_span_order")
        );
    }

    /// Whitespace inside a form matches any run of whitespace, the rule
    /// the UDPipe adapter places spans by; a span splitting a character
    /// is refused rather than sliced.
    #[test]
    fn spans_follow_the_form_rule() {
        let across = vec![token(1, 0, "a b", 0)];
        let mut wide = across.clone();
        wide[0].span = Span::new(0, 4);
        assert_eq!(reason(parse(1, "a\n b", &[wide])), None);
        let mut split = vec![token(1, 0, "é", 0)];
        split[0].span = Span::new(0, 1);
        assert_eq!(reason(parse(1, "é", &[split])), Some("token_span_invalid"));
    }

    fn para(text: &str, start: usize) -> ReadBlock {
        ReadBlock::paragraph(
            ParagraphKind::Paragraph,
            text,
            Span::new(start, start + text.len()),
        )
    }

    #[test]
    fn a_well_formed_read_passes() {
        let text = "# A\n\nb c\n\n| t |";
        let read = Read::new(
            vec![ReadBlock::heading(1, "A", Span::new(0, 3)), para("b c", 5)],
            vec![Skipped::new(SkippedKind::Table, Span::new(10, 15))],
        );
        assert_eq!(reason(super::read(text, &read)), None);
    }

    #[test]
    fn each_read_check_names_itself() {
        let text = "one two";
        let cases = [
            (Read::new(vec![para("two", 4)], vec![]), None),
            (
                Read::new(vec![ReadBlock::heading(1, "x", Span::new(3, 2))], vec![]),
                Some("block_span_invalid"),
            ),
            (
                Read::new(
                    vec![para("two", 4)],
                    vec![Skipped::new(SkippedKind::Code, Span::new(0, 99))],
                ),
                Some("block_span_invalid"),
            ),
            (
                Read::new(vec![para("two", 4), para("one", 0)], vec![]),
                Some("block_span_order"),
            ),
            (
                Read::new(
                    vec![para("one", 0)],
                    vec![Skipped::new(SkippedKind::Code, Span::new(2, 5))],
                ),
                Some("block_span_order"),
            ),
            (
                Read::new(vec![ReadBlock::heading(0, "x", Span::new(0, 3))], vec![]),
                Some("heading_level"),
            ),
            (
                Read::new(
                    vec![ReadBlock::paragraph(
                        ParagraphKind::Paragraph,
                        "eno",
                        Span::new(0, 3),
                    )],
                    vec![],
                ),
                Some("paragraph_text_mismatch"),
            ),
        ];
        for (read, expected) in cases {
            assert_eq!(reason(super::read(text, &read)), expected);
        }
    }
}
