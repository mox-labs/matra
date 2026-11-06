//! Parser port: the boundary between matra and whatever parses a
//! paragraph into tokenized, tagged, dependency-parsed sentences.
//!
//! This module holds only the trait. What a parser builds, [`Token`] and
//! [`Identity`], lives in the domain; the UDPipe adapter lives in a
//! submodule behind the `udpipe` feature. A parser of your own (a
//! transformer, a WebAssembly build of another toolkit, a rule-based
//! tokenizer) implements [`NlpProvider`] and reaches the pipeline through
//! `Engine::with_parser`.
//!
//! [`Token`]: domain::Token
//! [`Identity`]: domain::Identity

#[cfg(feature = "udpipe")]
pub(crate) mod udpipe;

use crate::domain;

/// A parser: one paragraph of text in, its sentences out, each a list of
/// CoNLL-U tokens.
///
/// The engine calls [`parse`](NlpProvider::parse) once per paragraph and
/// builds every [`domain::Sentence`] itself from the tokens returned: the
/// sentence's text, its span, and the fields derived from its dependency
/// tree (passive, negations, modals, mood, Hearst pairs). So a parser
/// does the parser's job and nothing else, and every derived field means
/// the same thing whichever parser ran.
///
/// # What `parse` returns
///
/// One `Vec<Token>` per sentence, in text order, covering the sentences
/// of `text`, the paragraph the engine handed in. Within each sentence,
/// in [`domain::Token`] terms:
///
/// - `id`s run `1..=n` in order, with no gaps, where `n` is the
///   sentence's token count.
/// - Every `head` is `0` or the `id` of a token in the same sentence,
///   and exactly one token has `head == 0`: the root. A sentence has at
///   least one token.
/// - Following `head` from any token reaches the root: the head
///   references form no cycle.
/// - Every `span` locates the token's form in `text`, in UTF-8 bytes
///   relative to `text`, not to the document: `&text[span.byte_start..
///   span.byte_end]` is the form, except that a run of whitespace inside
///   the form may stand for any run of whitespace in the text. Spans
///   follow token order across the whole paragraph and do not overlap.
///   The engine maps them into the analysed text.
/// - `text` is the surface form. Set `is_punct` for punctuation (UPOS
///   `PUNCT`), since word counts skip it, and `misc` to `SpaceAfter=No`
///   where no space follows, since the sentence's text is rebuilt from
///   the forms. The other columns hold what the parser emitted, `_` or
///   empty where it emitted nothing.
///
/// Build each token with [`domain::Token::builder`]. An empty paragraph
/// may return no sentences.
///
/// The engine checks every rule above before a token reaches a
/// [`domain::Document`], and fails the document with
/// [`domain::Error::ParseFailed`] when one breaks, its `reason` naming
/// the check (`token_id_sequence`, `head_out_of_range`, `root_count`,
/// `head_cycle`, `token_span_invalid`, `token_span_order` or
/// `token_form_mismatch`) and its `cause` the sentence and token. A
/// conforming parser is one whose output passes those checks on every
/// paragraph: run it through `Engine::analyze` over your own texts and
/// it holds when no document fails with one of those reasons.
///
/// # Errors
///
/// A parser reports its own failure as one of these, never as a panic,
/// and the engine passes it through unchanged:
///
/// - [`domain::Error::ParseFailed`], with `reason` `None`, when it
///   cannot parse `text`: an error from the underlying toolkit, or a
///   panic it caught at its own boundary, as the UDPipe adapter catches
///   one at the C++ boundary.
/// - [`domain::Error::ModelInvalid`] when its model cannot be used.
/// - [`domain::Error::Io`] when something it reaches fails: a file, or
///   a remote service.
/// - [`domain::Error::InputTooLarge`] when `text` exceeds a cap of its
///   own, with a `what` naming that cap.
///
/// # Size and blocking
///
/// `parse` is blocking and may be slow on long text. No paragraph over
/// [`domain::limits::MAX_INPUT_BYTES`] reaches it through the engine; a
/// caller invoking `parse` directly bypasses that bound.
///
/// # Threads
///
/// `Send`, because an engine holds its parser and moves to a worker
/// thread. Not `Sync`: the engine is not shared between threads, so a
/// parser holding state its parse path mutates (as UDPipe's model does)
/// needs no lock.
pub trait NlpProvider: Send {
    /// Parse one paragraph into sentences of tokens, satisfying the
    /// contract above.
    fn parse(&self, text: &str) -> domain::Result<Vec<Vec<domain::Token>>>;

    /// Who this parser is: its name, and the model it runs with that
    /// model's digest and licence where it has one. The engine records
    /// it in every document's `provenance.parser`, so a reader sees which
    /// model produced a parse rather than discovering a change of model
    /// as a shift in the numbers. Two parsers that can parse the same
    /// text differently must not report the same identity.
    fn identity(&self) -> domain::Identity;
}
