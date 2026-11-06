//! The provider ports from outside the crate: a parser, a reader and a
//! source of a caller's own, plugged into an `Engine` through the public
//! surface alone, and the engine's checks on what each hands back.
//!
//! What a conforming implementation must pass is what these tests hold
//! the broken ones to. A parser conforms when, on every paragraph, each
//! sentence's ids run `1..=n` in order (`token_id_sequence`), every head
//! is `0` or a token of the sentence (`head_out_of_range`), exactly one
//! token is the root (`root_count`), the heads form no cycle
//! (`head_cycle`), and every span is a span of the paragraph
//! (`token_span_invalid`), follows the previous token's
//! (`token_span_order`), and covers the token's form
//! (`token_form_mismatch`). A reader conforms when every span is a span
//! of the analysed text (`block_span_invalid`), blocks are in document
//! order and no two spans overlap (`block_span_order`), every heading
//! is at level 1 or more (`heading_level`), and every paragraph's text
//! is the analysed text at its span (`paragraph_text_mismatch`). A
//! source conforms by returning the bytes of the path it was given, or
//! an error. An implementation that passes these checks over its own
//! inputs, run through `Engine::analyze`, holds the contract.
//!
//! Runs with no parser in the build (`cargo test --no-default-features`):
//! the traits and `Engine::with_parser` depend on nothing optional.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use matra::decompose::{Decomposer, Read, ReadBlock};
use matra::domain::{
    DetailValue, Error, Format, Identity, ParagraphKind, Result, Skipped, SkippedKind, Span, Token,
};
use matra::nlp::NlpProvider;
use matra::source::Source;
use matra::{Engine, Input};

/// A conforming parser: one sentence per paragraph, one token per
/// whitespace-separated word, the first word the root and every other
/// attached to it.
struct Words;

fn words(text: &str) -> Vec<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut at = 0;
    for word in text.split_whitespace() {
        let start = at + text[at..].find(word).expect("the word is in the text");
        at = start + word.len();
        let id = tokens.len() + 1;
        let head = usize::from(id > 1);
        tokens.push(
            Token::builder(
                id,
                word.to_string(),
                word.to_lowercase(),
                "X".to_string(),
                head,
                "dep".to_string(),
            )
            .span(Span::new(start, at))
            .build(),
        );
    }
    if tokens.is_empty() {
        Vec::new()
    } else {
        vec![tokens]
    }
}

impl NlpProvider for Words {
    fn parse(&self, text: &str) -> Result<Vec<Vec<Token>>> {
        Ok(words(text))
    }

    fn identity(&self) -> Identity {
        Identity::new("words", Some("v1".to_string()), None, None)
    }
}

/// A parser that breaks one rule of the contract on every paragraph
/// holding three words or more.
#[derive(Clone, Copy, Debug)]
enum Broken {
    IdsSkip,
    HeadPastTheEnd,
    TwoRoots,
    NoTokens,
    Cycle,
    SpanPastTheText,
    SpansOutOfOrder,
    FormElsewhere,
}

impl NlpProvider for Broken {
    fn parse(&self, text: &str) -> Result<Vec<Vec<Token>>> {
        let mut sentences = words(text);
        let tokens = &mut sentences[0];
        match self {
            Broken::IdsSkip => tokens[1].id = 3,
            Broken::HeadPastTheEnd => tokens[2].head = 9,
            Broken::TwoRoots => tokens[2].head = 0,
            Broken::NoTokens => tokens.clear(),
            Broken::Cycle => {
                tokens[1].head = 3;
                tokens[2].head = 2;
            }
            Broken::SpanPastTheText => tokens[2].span = Span::new(text.len() - 1, text.len() + 4),
            Broken::SpansOutOfOrder => {
                tokens[1].text = tokens[0].text.clone();
                tokens[1].span = tokens[0].span;
            }
            Broken::FormElsewhere => tokens[2].text = "fell.".to_string(),
        }
        Ok(sentences)
    }

    fn identity(&self) -> Identity {
        Identity::new("broken", None, None, None)
    }
}

/// The check a failure names, its `reason` detail, and its `cause`.
fn check_of(err: &Error) -> (Option<String>, String) {
    assert_eq!(err.kind(), "parse_failed", "{err}");
    let details = err.details();
    let reason = match details.get("reason") {
        Some(DetailValue::Text(reason)) => Some(reason.clone()),
        _ => None,
    };
    let Some(DetailValue::Text(cause)) = details.get("cause") else {
        panic!("a parse failure carries its cause: {err}");
    };
    (reason, cause.clone())
}

const PROSE: &str = "The river rose overnight.\n\nThe bridge held.";

#[test]
fn a_parser_of_your_own_runs_through_the_engine() {
    let doc = Engine::with_parser(Words)
        .analyze(Input::text(PROSE, Format::PlainText))
        .expect("a conforming parser analyses");
    assert_eq!(doc.provenance.parser.name, "words");
    assert_eq!(doc.provenance.parser.model.as_deref(), Some("v1"));
    assert_eq!(doc.metrics.total_sentences, 2);
    let first = doc.sentences().next().expect("a sentence");
    assert_eq!(first.text, "The river rose overnight.");
    assert_eq!(first.tree_depth, Some(1));
    assert_eq!(
        &PROSE[first.span.byte_start..first.span.byte_end],
        first.text
    );
    let last = doc.tokens().last().expect("a token");
    assert_eq!(&PROSE[last.span.byte_start..last.span.byte_end], "held.");
}

/// Token spans are relative to the paragraph the parser was given, and
/// the engine maps them into the analysed text, through markup too.
#[test]
fn spans_relative_to_the_paragraph_land_in_the_document() {
    let md = "# Title\n\nSome **bold** words.";
    let doc = Engine::with_parser(Words)
        .analyze(Input::text(md, Format::Markdown))
        .expect("analyse");
    let tokens: Vec<&str> = doc
        .tokens()
        .map(|t| &md[t.span.byte_start..t.span.byte_end])
        .collect();
    assert_eq!(tokens, ["Some", "bold", "words."]);
}

#[test]
fn every_broken_rule_is_caught_and_named() {
    for (parser, reason) in [
        (Broken::IdsSkip, "token_id_sequence"),
        (Broken::HeadPastTheEnd, "head_out_of_range"),
        (Broken::TwoRoots, "root_count"),
        (Broken::NoTokens, "root_count"),
        (Broken::Cycle, "head_cycle"),
        (Broken::SpanPastTheText, "token_span_invalid"),
        (Broken::SpansOutOfOrder, "token_span_order"),
        (Broken::FormElsewhere, "token_form_mismatch"),
    ] {
        let err = Engine::with_parser(parser)
            .analyze(Input::text("The river rose.", Format::PlainText))
            .expect_err("a broken parse fails the document");
        let (got, cause) = check_of(&err);
        assert_eq!(got.as_deref(), Some(reason), "{parser:?}: {err}");
        assert!(cause.contains("paragraph 1"), "{parser:?}: {cause}");
        assert!(matches!(err, Error::ParseFailed { .. }), "{parser:?}");
    }
}

/// A broken parse names the token, so whoever wrote the parser can find
/// it.
#[test]
fn a_broken_parse_names_the_token() {
    let err = Engine::with_parser(Broken::HeadPastTheEnd)
        .analyze(Input::text("The river rose.", Format::PlainText))
        .expect_err("refused");
    let (_, cause) = check_of(&err);
    assert!(cause.contains("token 3 (\"rose.\")"), "{cause}");
    assert!(cause.contains("head 9"), "{cause}");
}

/// A parser's own failure reaches the caller as it was raised, with no
/// check named.
#[test]
fn a_parsers_own_failure_passes_through() {
    struct Refuses;
    impl NlpProvider for Refuses {
        fn parse(&self, _text: &str) -> Result<Vec<Vec<Token>>> {
            Err(Error::ParseFailed {
                reason: None,
                cause: "the service is down".to_string(),
            })
        }
        fn identity(&self) -> Identity {
            Identity::new("refuses", None, None, None)
        }
    }
    let err = Engine::with_parser(Refuses)
        .analyze(Input::text("Text.", Format::PlainText))
        .expect_err("refused");
    assert_eq!(check_of(&err), (None, "the service is down".to_string()));
}

/// Every non-empty line is a paragraph: a reader for a format matra has
/// no name for.
struct Lines;

impl Decomposer for Lines {
    fn decompose(&self, input: &[u8]) -> Result<Read> {
        let text = std::str::from_utf8(input).map_err(|_| Error::UnsupportedFormat {
            format: None,
            feature: None,
        })?;
        let mut blocks = Vec::new();
        let mut at = 0;
        for line in text.split_inclusive('\n') {
            let own = line.trim_end();
            if !own.is_empty() {
                blocks.push(ReadBlock::paragraph(
                    ParagraphKind::Paragraph,
                    own,
                    Span::new(at, at + own.len()),
                ));
            }
            at += line.len();
        }
        Ok(Read::new(blocks, Vec::new()))
    }
}

#[test]
fn a_reader_of_your_own_reads_a_format_you_name() {
    let lines = Format::named("lines");
    let engine = Engine::with_parser(Words).with_reader(lines.clone(), Lines);
    let doc = engine
        .analyze(Input::text("One line.\nAnother line.\n", lines))
        .expect("analyse");
    assert_eq!(doc.paragraphs().count(), 2);
    assert_eq!(doc.provenance.format, Format::named("lines"));
    let json = serde_json::to_value(&doc).expect("serialize");
    assert_eq!(json["provenance"]["format"], "lines");
}

#[test]
fn a_format_with_no_reader_is_unsupported_by_name() {
    let err = Engine::with_parser(Words)
        .analyze(Input::text("x", Format::named("html")))
        .expect_err("no reader for html");
    assert_eq!(err.kind(), "unsupported_format");
    assert_eq!(
        err.details().get("format"),
        Some(&DetailValue::Text("html".to_string()))
    );
    assert!(err.to_string().contains("no reader is registered"), "{err}");
}

/// A PDF reader gets the bytes, not text, and names the text it
/// extracted as the analysed text.
#[test]
fn a_reader_for_a_binary_format_names_its_own_text() {
    struct FakePdf;
    impl Decomposer for FakePdf {
        fn decompose(&self, input: &[u8]) -> Result<Read> {
            assert!(input.starts_with(b"%PDF"), "the reader sees the bytes");
            let text = "Extracted prose.";
            let block = ReadBlock::paragraph(ParagraphKind::Paragraph, text, Span::new(0, 16));
            Ok(Read::new(vec![block], Vec::new()).with_text(text))
        }
    }
    let bytes = b"%PDF-1.7 \xff\xfe not text".to_vec();
    let doc = Engine::with_parser(Words)
        .with_reader(Format::Pdf, FakePdf)
        .analyze(Input::bytes(bytes, None))
        .expect("a registered PDF reader reads a PDF");
    assert_eq!(doc.provenance.format, Format::Pdf);
    assert_eq!(doc.provenance.source.bytes, 16);
    assert_eq!(doc.metrics.total_words, 2);
}

/// A reader that hands back one fixed read, to break its rules.
struct Fixed(Read);

impl Decomposer for Fixed {
    fn decompose(&self, _input: &[u8]) -> Result<Read> {
        Ok(self.0.clone())
    }
}

#[test]
fn every_broken_reader_rule_is_caught_and_named() {
    let text = "Alpha beta.\n\nGamma delta.";
    let first = || ReadBlock::paragraph(ParagraphKind::Paragraph, "Alpha beta.", Span::new(0, 11));
    let second =
        || ReadBlock::paragraph(ParagraphKind::Paragraph, "Gamma delta.", Span::new(13, 25));
    let cases = [
        (
            Read::new(
                vec![first(), ReadBlock::heading(1, "x", Span::new(20, 99))],
                vec![],
            ),
            "block_span_invalid",
        ),
        (
            Read::new(vec![second(), first()], vec![]),
            "block_span_order",
        ),
        (
            Read::new(
                vec![first()],
                vec![Skipped::new(SkippedKind::Code, Span::new(5, 15))],
            ),
            "block_span_order",
        ),
        (
            Read::new(
                vec![ReadBlock::heading(0, "Alpha", Span::new(0, 5))],
                vec![],
            ),
            "heading_level",
        ),
        (
            Read::new(
                vec![ReadBlock::paragraph(
                    ParagraphKind::Paragraph,
                    "Alpha beta!",
                    Span::new(0, 11),
                )],
                vec![],
            ),
            "paragraph_text_mismatch",
        ),
    ];
    for (read, reason) in cases {
        let err = Engine::with_parser(Words)
            .with_reader(Format::PlainText, Fixed(read))
            .analyze(Input::text(text, Format::PlainText))
            .expect_err("a broken read fails the document");
        let (got, _) = check_of(&err);
        assert_eq!(got.as_deref(), Some(reason), "{err}");
    }
}

/// Replacing a shipped reader replaces it.
#[test]
fn a_shipped_reader_can_be_replaced() {
    let doc = Engine::with_parser(Words)
        .with_reader(Format::Markdown, Lines)
        .analyze(Input::text("# Not a heading\nprose", Format::Markdown))
        .expect("analyse");
    assert_eq!(
        doc.paragraphs().count(),
        2,
        "read line by line, not as markdown"
    );
}

/// Documents held in memory, under paths.
struct Memory(HashMap<PathBuf, Vec<u8>>);

impl Source for Memory {
    fn read(&self, path: &Path) -> Result<Vec<u8>> {
        self.0.get(path).cloned().ok_or_else(|| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("no document at {}", path.display()),
            ))
        })
    }
}

#[test]
fn a_source_of_your_own_reads_the_paths_inputs_name() {
    let store = Memory(HashMap::from([(
        PathBuf::from("store/notes.md"),
        b"# Notes\n\nThe river rose.".to_vec(),
    )]));
    let engine = Engine::with_parser(Words).with_source(store);
    let doc = engine
        .analyze(Input::file("store/notes.md"))
        .expect("read from the store");
    assert_eq!(
        doc.provenance.format,
        Format::Markdown,
        "the extension still decides"
    );
    assert_eq!(
        doc.provenance.source.path.as_deref(),
        Some(Path::new("store/notes.md"))
    );
    assert_eq!(doc.sections[0].heading.as_deref(), Some("Notes"));

    let missing = engine
        .analyze(Input::file("store/absent.md"))
        .expect_err("not in the store");
    assert_eq!(missing.kind(), "io");
}

/// What a source returns is capped like any other input.
#[test]
fn a_source_cannot_pass_the_size_cap() {
    struct Huge;
    impl Source for Huge {
        fn read(&self, _path: &Path) -> Result<Vec<u8>> {
            Ok(vec![b'a'; matra::domain::limits::MAX_INPUT_BYTES + 1])
        }
    }
    let err = Engine::with_parser(Words)
        .with_source(Huge)
        .analyze(Input::file("anything.txt"))
        .expect_err("over the cap");
    assert_eq!(err.kind(), "input_too_large");
}

/// An engine with parts of your own still moves to a worker thread.
#[test]
fn an_engine_with_your_parts_is_send() {
    fn assert_send<T: Send>(_: &T) {}
    let engine = Engine::with_parser(Words)
        .with_reader(Format::named("lines"), Lines)
        .with_source(Memory(HashMap::new()));
    assert_send(&engine);
}
