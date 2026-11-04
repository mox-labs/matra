# Conformance spec

Language-agnostic fixtures that every matra crust must satisfy identically.

matra ships one Rust core behind several bindings: the Rust library, the Python
wheel via PyO3 and pythonize, and (later) a TypeScript package via
wasm-bindgen. They all call the same parser, so a difference between them is
never a difference of behaviour. It is a binding defect: a renamed field, a
dropped value, a number that lost precision on the way across, an ordering that
was stable on one side and not the other.

These fixtures exist to make that class of defect loud.

## Layout

```
spec/
  README.md          this file
  tests/*.json       one fixture per file (parse conformance)
  tests/semantic/    embedding-tier fixtures: the FFI shape fixture
                     (modelless) and the pinned reference-model
                     conformance (potion-base-8M by artifact digest)
  tests/corpus/      the corpus item shapes and the error kind vocabulary
  tests/cli/         the command line's JSON envelope
  schema/v2/         the JSON Schema for schema 2.x documents, and the
                     schema of the `blocks` input format
```

Each crust has a runner that loads every fixture in `spec/tests/` and asserts
the same expectations:

| Crust | Parse runner | Semantic runner | Corpus runner | CLI runner |
|---|---|---|---|---|
| Rust | `tests/conformance.rs` | `tests/semantic_conformance.rs` | `tests/corpus_conformance.rs` | `tests/cli.rs` |
| Python | `python/tests/test_conformance.py` | `python/tests/test_semantic_conformance.py` | `python/tests/test_corpus_conformance.py` | `python/tests/test_cli.py` |

Run them with `just conformance`. Both model lanes fetch on first use and
load from disk after: the parse lane the UDPipe model, the semantic
reference lane potion-base-8M through `Model2Vec::potion_base_8m`, into
the resolved model directory or into `MATRA_MODEL2VEC_DIR` when that is
set. Each download is pinned by a digest in the source and verified
before anything loads, so what a lane can obtain is exactly the artifact
its fixture names. The semantic shape fixture runs everywhere with no
model at all.

## Fixture format

```json
{
  "name": "passive-voice",
  "input": "The committee approved the proposal.",
  "format": "plain_text",
  "expect": {
    "metrics": {
      "total_sentences": 1,
      "total_words": 5,
      "paragraph_count": 1,
      "vocabulary_ttr": 1.0,
      "nominalization_ratio": 0.0
    },
    "sentences": [
      {
        "text": "The committee approved the proposal.",
        "token_count": 6,
        "root_mood": { "root_id": 3, "mood": "Ind" },
        "tokens": [
          { "id": 1, "text": "The", "lemma": "the", "pos": "DET", "head": 2, "dep": "det" }
        ]
      }
    ]
  }
}
```

`format` is `plain_text` or `markdown`, the format vocabulary every surface shares, selecting which reader runs. `expect.metrics` mirrors the document's `metrics`, and a sentence pins only the detections it names: an absent key is not checked, and a `root_mood` of `null` pins "no mood".

The optional `expect.sections` (each heading's `heading`, `level` and `span`), `expect.paragraphs` (each paragraph's `kind` and `span`) and `expect.skipped` pin the reader's output, and a `span` on a sentence or a token pins where it lies. Every span counts UTF-8 bytes of `input`. Whatever a fixture pins, every runner also checks two things over every fixture: each token's span slices `input` to the token's text, and the accounting law L9, that every byte of `input` that is not whitespace lies in exactly one heading, paragraph or skipped span. `markdown-structure.json` is the markdown fixture: front matter, a `## References` heading, inline markup, a quote, a list item and a table.

## The schema

`schema/v2/document.schema.json` is the published JSON Schema for a document whose `provenance.schema` is `2.x`. A minor version adds optional fields only, so the schema leaves objects open: a reader of `2.0` accepts a `2.1` document. Every runner that produces a document validates it against the schema with every object closed, so a field matra emits but the schema does not list fails the run that emitted it: `tests/document_schema/mod.rs` for the Rust lanes and `python/tests/document_schema.py` for the Python ones. `tests/schema.rs` does the same with no model, so the check runs in every `cargo test`.

`schema/v2/blocks.schema.json` is the schema of an input, not an output: the JSON array of headings and paragraphs the `blocks` format reads. It is closed, since matra refuses a `kind` or a field it does not know; `tests/schema.rs` holds the reader and the schema to the same refusals.

## The CLI lane

There is one command line, compiled into the library and reached from two
launchers: the Rust binary and the Python entry point. They cannot drift in
behaviour, because there is only one implementation. What they can drift on is
the handoff: an exit code lost on the way back, output written to the wrong
stream, a launcher that mangles its arguments.

`tests/cli/envelope.json` pins the `--json` envelope for a fixed input. Each
runner writes the fixture's `input` to a file named by `filename`, runs the
`args` against it, and asserts the envelope's four keys, its `format_version`,
its `command`, its `input` (the path it passed), and the structure of `result`:
which keys the domain value carries, how many sections, how many sentences.
`result` is compared structurally rather than byte for byte, because its
contents are the parse, which the parse fixtures already pin.

Token expectations cover the six fields that carry meaning across every
binding: `id`, `text`, `lemma`, `pos`, `head`, `dep`. The remaining CoNLL-U
columns are passed through unmodified and are not asserted, because a model
upgrade may legitimately change them without any binding being at fault.

Floating-point expectations compare within `1e-6`.

## The corpus lane

`tests/corpus/items.json` pins what one document's result looks like when it
crosses a binding: the two item shapes a walk yields (the document itself, or
an error with its path), and the kind vocabulary a consumer branches on instead
of reading a message. `DocumentError` has no serde
form, because it wraps `std::io::Error`, so every binding materializes those
fields by hand, and hand-written projections are exactly where a key gets
renamed on one side only.

Each runner asserts the vocabulary against its own surface, `Error::kind` in
Rust and `matra.ERROR_KINDS` in Python, and then walks a directory holding one
readable file and one whose bytes are not UTF-8, which is an
`unsupported_format` item. The Rust runner stubs the
parser with a provider that returns no sentences, so it needs no model; what it
pins is the shape and the order, not the parse, which the parse fixtures
already cover. The Python runner reaches the walk through `analyze_path`, a
method on a loaded engine, so its directory expectation carries the `model`
marker while the vocabulary and shape assertions run everywhere.

## The model is part of the contract

Every expectation here was produced by UDPipe `english-ewt-ud-2.5-191206`,
verified by SHA-256 at load time. A different model produces a different parse
and these fixtures will fail, correctly. When the pinned model version changes,
regenerate the fixtures in the same commit that changes the pin, and say so in
the commit message.

## Adding a fixture

1. Write the input and the case you want to protect.
2. Generate the expectations from the implementation, then read them. A
   generated expectation you have not checked is not a test, it is a snapshot
   of a bug.
3. Confirm every crust passes before committing.

A fixture earns its place by covering a behaviour a binding could plausibly
break: a field name that differs between languages, a nested structure that
could flatten, a number that could round, an ordering that could drift.
