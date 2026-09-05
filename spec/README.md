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
```

Each crust has a runner that loads every fixture in `spec/tests/` and asserts
the same expectations:

| Crust | Runner |
|---|---|
| Rust | `tests/conformance.rs` |
| Python | `python/tests/test_conformance.py` |

Run them with `just conformance`.

## Fixture format

```json
{
  "name": "passive-voice",
  "input": "The committee approved the proposal.",
  "format": "plain",
  "expect": {
    "total_sentences": 1,
    "total_words": 5,
    "paragraph_count": 1,
    "sentences": [
      {
        "text": "The committee approved the proposal.",
        "token_count": 6,
        "tokens": [
          { "id": 1, "text": "The", "lemma": "the", "pos": "DET", "head": 2, "dep": "det" }
        ]
      }
    ],
    "vocabulary_ttr": 1.0,
    "nominalization_ratio": 0.0
  }
}
```

`format` is `plain` or `markdown`, selecting which decomposer runs.

Token expectations cover the six fields that carry meaning across every
binding: `id`, `text`, `lemma`, `pos`, `head`, `dep`. The remaining CoNLL-U
columns are passed through unmodified and are not asserted, because a model
upgrade may legitimately change them without any binding being at fault.

Floating-point expectations compare within `1e-6`.

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
