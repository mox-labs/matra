# RFC-0018: Source spans and provenance

- Feature Name: `source_spans`
- Start Date: 2026-09-26
- RFC PR: (this pull request)
- Tracking EP: (assigned when the EP opens)
- Status: accepted (the file reaches `main` only by merging, which is what accepts it)

## Summary

Every paragraph, sentence and token matra returns carries a `SourceSpan`: where it sits in
the text the caller passed in, in bytes and in characters. Every document carries a
`Provenance` stamp that names the matra version, the output schema version, and the parser
and model that produced it. `NlpProvider` gains `identity()`, the same method `Embedder`
already has. A token's span is either exact (slicing the source at it gives back the
token's form) or absent; it is never approximate. A sentence's or paragraph's span covers
the source from its first placed token to its last.

## Motivation

A caller who wants to point back at the source cannot do it today. `Token` has `text` and
`id`, `Sentence` has `text`, and neither says where in the input it came from. The
Markdown decomposer rebuilds each paragraph by trimming and joining lines
(`src/decompose/markdown.rs`), so a paragraph's text is often not a substring of the
source at all, and a caller searching for it can find the wrong occurrence or none.

Three kinds of caller need positions:

- **Highlighting and review tools** that underline a finding in the original file. Today
  they re-find the sentence text by string search, which fails on repeated sentences and
  on anything the decomposer normalised.
- **Anything that grounds a claim in a quotation.** A claim a reader can check is one
  whose quotation can be re-derived from the source at a stated position. Without
  positions the check is "the reference exists", not "the reference says this". A
  pipeline that summarises documents and then verifies each summary sentence against its
  source needs exactly this.
- **Rules**, which RFC-0006 reserves (`Rule`, `Predicate`, `Finding`). Its `Finding`
  contract makes provenance mandatory ("Frame-1. `source_span()` returns a valid byte
  range"). There is nothing to point at until the parse carries spans.

Provenance has the same shape of need. A result without the parser's identity cannot be
compared with a later one: when the model changes, every downstream number can move, and
nothing in the output says which model produced which number. `Embedder::identity()`
already solved this for semantic scores (RFC-0010). The parse is the one input every
other result depends on, and it has no identity at all.

## Guide-level explanation

### Spans

```rust
let doc = engine.analyze_one(RawDocument::new(text.clone(), None, Format::Markdown))?;
for sentence in doc.analysis.sentences() {
    for token in &sentence.tokens {
        if let Some(span) = &token.span {
            assert_eq!(&text[span.byte_start..span.byte_end], token.text);
        }
    }
    if let Some(span) = &sentence.span {
        println!("sentence at {}..{}", span.char_start, span.char_end);
    }
}
```

```python
doc = matra.Matra().analyze(text)
for s in doc["sections"][0]["paragraphs"][0]["sentences"]:
    for t in s["tokens"]:
        span = t["span"]
        if span is not None:
            assert text[span["char_start"]:span["char_end"]] == t["text"]
```

A `SourceSpan` has four fields: `byte_start`, `byte_end`, `char_start`, `char_end`. Both
pairs are half-open ranges over the exact string the caller passed in. Rust slices by
bytes; Python slices by characters (code points); both slice without conversion.

Tokens carry the exactness guarantee. A sentence or paragraph span is the range from its
first placed token's start to its last placed token's end, so it points at the right
place in the file; its slice can differ from `Sentence.text` where the decomposer removed
something between lines (indentation, a `>` marker) or the parser normalised whitespace.

A token's `span` is `None` when matra cannot place it exactly. Cases:

- the parser changed the form (UDPipe splits a contraction into syntactic words whose
  forms are not substrings, for example);
- the decomposer rewrote the text (a Markdown paragraph whose lines were re-joined still
  maps back; a character the decomposer inserted does not).

A sentence's span is `None` only when none of its tokens is placed. A `None` span is a
statement ("matra cannot point at this exactly"), not a bug to work
around by string search. The share of placed tokens is reported by the conformance tests,
so a regression shows as a number.

### Provenance

```json
{
  "provenance": {
    "matra_version": "0.3.0",
    "schema_version": 1,
    "parser": "udpipe:english-ewt-ud-2.5-191206:sha256:5a4e...",
    "decomposer": "markdown"
  },
  "sections": [ ... ]
}
```

`schema_version` is an integer that changes only when a JSON consumer would break: a
field removed, renamed, or changed in type. Adding a field does not change it. `parser` is
whatever `NlpProvider::identity()` returns; for the UDPipe adapter it names the model
file and its SHA-256, which the adapter already verifies before loading.

## Reference-level explanation

### Types (domain)

```rust
#[non_exhaustive]
pub struct SourceSpan { pub byte_start: usize, pub byte_end: usize,
                        pub char_start: usize, pub char_end: usize }

#[non_exhaustive]
pub struct Provenance { pub matra_version: String, pub schema_version: u32,
                        pub parser: String, pub decomposer: String }
```

`Paragraph`, `Sentence` and `Token` each gain `span: Option<SourceSpan>`. `Document` gains
`provenance: Provenance`. All are fields (RFC-0008: derivations cross as fields), so the
CLI's JSON and Python see them with no binding work. `SCHEMA_VERSION` is a `pub const` in
`domain`.

RFC-0006 reserved `SourceSpan` as "(byte_offset, byte_length, sentence_id, token_range)"
without committing to shape. This RFC fixes the shape and departs from that sketch in two
ways, deliberately: start and end rather than offset and length, because every consumer
slices with a range; and no `sentence_id` or `token_range`, because containment already
says which sentence a token is in, and a token range belongs on a `Finding` (which points
at several tokens) rather than on a span (which points at text). Character offsets are
added because Python, the main non-Rust consumer, indexes strings by code point.

### Where each offset comes from

Offsets compose through the two stages that already exist, and each stage owns its part:

1. **Decomposer: paragraph to source.** The `Decomposer` port returns, per paragraph, an
   offset map from positions in `Paragraph.text` to positions in the source: a sorted list
   of segments, each a run of paragraph text that is a verbatim copy of a run of source.
   Trimmed indentation, a stripped `>` marker and a `\r\n` line ending each start a new
   segment. `Paragraph.span` covers the first to the last mapped byte. This is a change to
   the port's return type; both adapters (`markdown`, `plain`) implement it.
2. **Parser: token to paragraph.** The UDPipe adapter aligns each token form against the
   paragraph text with a forward-only cursor: find the form at or after the cursor,
   skipping only whitespace; if it is not there, the token's span is `None` and the cursor
   does not move. Multiword tokens (`don't` as `do` + `n't`) are placed when each part is
   a substring in order and left `None` otherwise. Sentence spans run from their first
   placed token to their last.
3. **Engine: composition.** `annotate` maps each token's paragraph-relative span through
   the paragraph's offset map into source coordinates, computing character offsets from
   byte offsets in one pass over the source. A token whose ends fall in different
   segments, or outside every segment, gets `None`. Sentence and paragraph spans are then
   built from their mapped tokens, so a sentence wrapped across indented lines still gets
   a span.

UDPipe's own tokenizer can report ranges (`TokenRange=` in MISC with the `ranges`
option), but `udpipe-rs` 0.2.0 constructs its tokenizer with `model::DEFAULT` in its C++
wrapper and exposes no option. Alignment inside our adapter needs no upstream change and
is checked by the same invariant; an upstream option is a later improvement, not a
prerequisite.

### Invariants and how each is checked

- **Exactness.** For every token with `Some(span)`: `source[span.byte_start..span.byte_end]`
  equals the token's `text`, and the character offsets describe the same range.
- **Cover.** A sentence's span starts at its first placed token and ends at its last; a
  paragraph's likewise over its sentences. A debug assertion in `annotate`, a property test over generated Markdown and
  plain text (including CRLF, indentation, blockquotes, multibyte characters and emoji),
  and conformance fixtures in `spec/tests/`.
- **Order.** Token spans within a sentence are non-decreasing; sentence spans within a
  paragraph are non-decreasing. Property test.
- **Coverage floor.** The conformance corpus reports the share of tokens placed; the test
  fails below a threshold set from the first measured run, so a regression is a number.
- **Provenance present.** Every `Document` produced by `Engine` carries a non-empty
  `parser`; a test with a stub `NlpProvider` asserts its identity arrives verbatim.
- **Schema version discipline.** A conformance fixture pins `schema_version`; a JSON
  change that removes or retypes a field fails the fixture diff, and the reviewer decides
  whether the version moves.

### Ports

- `NlpProvider` gains `fn identity(&self) -> &str`, required, mirroring `Embedder`. A
  default implementation would let a provider ship without one and put `"unknown"` into
  every provenance stamp, which is the failure this RFC removes.
- `NlpProvider::parse` keeps its signature; the adapter fills `Token.span` and
  `Sentence.span` relative to the text it was given.
- `Decomposer::decompose` returns paragraphs with their offset maps (shape settled in the
  EP; a `Vec<Section>` whose `Paragraph`s carry a private-to-the-crate map, or a parallel
  structure).

Boundary rules are unchanged: the new types live in `domain` (serde, std only), the ports
import only `domain`, and the composition happens in `lib.rs`.

## Drawbacks

- **The `Decomposer` and `NlpProvider` ports change**, and anyone implementing them
  outside the crate must update. Pre-1.0, and no external implementor is known; the
  CHANGELOG names the change.
- **Output grows.** Four integers per token roughly adds a fifth to the JSON of a parse.
  Callers who do not want spans pay for them; a flag to omit them is possible later and
  not proposed now.
- **Alignment is heuristic in one place**: which occurrence of a form a token matches.
  The forward-only cursor makes that deterministic, and the exactness invariant means a
  wrong choice can only produce a span that still slices to the same text, never one that
  slices to different text.

## Rationale and alternatives

- **Byte offsets only** (RFC-0006's sketch). Correct in Rust, wrong in Python for any
  non-ASCII text unless every caller converts. Character offsets cost one pass.
- **Character offsets only.** Rust would need a conversion to slice. Both are cheap to
  carry and remove a class of off-by-a-multibyte bugs in every consumer.
- **String search in the caller.** Today's state. Fails on repeated sentences and on
  normalised text.
- **Patch `udpipe-rs` for `ranges` first.** Blocks on a third party; does not cover the
  decomposer half, which is where most of the difficulty is.
- **Offsets relative to the paragraph only.** Simpler, but every caller then needs the
  paragraph's position, which only matra knows.

## Prior art

- spaCy: `Token.idx` (character offset into the `Doc` text) and `Span.start_char` /
  `end_char`; offsets are always into the text given to the pipeline.
- Stanza: `start_char` / `end_char` on tokens, from its tokenizer.
- CoNLL-U: `TokenRange=start:end` in MISC, emitted by UDPipe's tokenizer with `ranges`.
- W3C Web Annotation's `TextPositionSelector` (start, end over code points).
- RFC-0006 (the reserved `SourceSpan` and the `Finding` contract); RFC-0008 (fields cross
  FFI); RFC-0010 (`Embedder::identity`, the precedent for `NlpProvider::identity`).

## Unresolved questions

- To resolve in review: whether `Provenance` belongs on `Document` or on `CorpusEntry`.
  This RFC puts it on `Document` because Python's `analyze` returns a `Document`.
- To resolve in the EP: the exact shape of the decomposer's offset map, and the coverage
  threshold (set from the first measured run).
- Out of scope: UTF-16 offsets for a future TypeScript surface (that crust can compute
  them from the character offsets); an option to omit spans from output.

## Future possibilities

`Finding` (RFC-0006) can now satisfy its provenance rule, which is what a detector
extension point and rule evaluation need; that is the subject of its own RFC. A caller can
render any result as a highlight over the original file, and a verifier can re-derive
each quoted claim from the source by position.
