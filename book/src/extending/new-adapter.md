# Writing a new adapter

The recipe for adding a new adapter to an existing port.

## 1. Pick the port

| Port | When |
|---|---|
| `Source` | You want a new way to feed bytes into vaani (filesystem watch, HTTP, archive expansion, etc.). |
| `Decomposer` | You want to parse a new structural format (DocBook, AsciiDoc, RST, etc.). |
| `NlpProvider` | You want a different NLP backend (Stanza, spaCy via FFI, a pure-Rust tagger, etc.). |

If your need doesn't fit any of the three, you probably do not need a new adapter. You may need a function in `metrics/` or `extraction/`, or a layer above vaani. Adding a new port is a higher bar; see [Future direction](./future-direction.md).

## 2. Create the file

In the port's module:

```
src/source/my_new_source.rs
src/decompose/my_new_decomposer.rs
src/nlp/my_new_backend.rs
```

Add a `pub mod my_new_source;` line to the port's `mod.rs`.

## 3. Implement the port trait

```rust,ignore
use crate::domain::{self, RawDocument};
use super::Source;
use std::path::Path;

pub struct MyNewSource { /* ... */ }

impl Source for MyNewSource {
    fn read(&self, input: &Path) -> domain::Result<Vec<RawDocument>> {
        // Translate external errors to domain::Error variants.
        // Do not propagate foreign error types through the boundary.
        unimplemented!()
    }

    fn accepts(&self, input: &Path) -> bool {
        // Cheap check the composition root uses to pick the right adapter.
        unimplemented!()
    }
}
```

## 4. Don't import other adapters

Your adapter file imports from `crate::domain` and `super::` (the port module). It does **not** import from sibling adapters. If your adapter needs functionality from another adapter, the composition root composes them. Your file does not.

This is [boundary rule 3](../architecture/boundary-rules.md): no port module imports another port module. The same principle applies to adapters within a port.

## 5. Translate external errors

Foreign errors (e.g., from a third-party crate) become `domain::Error` variants at the adapter boundary. Existing variants might suffice; if not, add a new `Error` variant (a minor-version change, since `Error` is `#[non_exhaustive]`) and update the PyO3 boundary's `From<VaaniError> for PyErr` to route it.

## 6. Document contract overrides

If your adapter's behavior differs from the port's documented postconditions, document the override inline. Examples in the existing codebase:

- `DirectorySource` sorts paths lexicographically (documented in `directory.rs`).
- `Udpipe` wraps panics via `catch_unwind` (documented in `udpipe.rs`).

## 7. Add unit tests

Adapter tests live in `#[cfg(test)] mod tests` in the same file. Cover:

- Happy path on a typical input.
- Edge cases: empty input, oversized input, malformed input.
- Failure modes: what variants of `domain::Error` your adapter can return, and when.
- Contract preservation: verify the port's postconditions hold for your adapter's output.

If your adapter touches I/O, follow the resilience-floor patterns:

- Size cap *before* reading.
- Symlink rejection for filesystem adapters.
- Atomic file writes (per-process temp + rename).
- TOCTOU-closed verify: return the verified bytes, don't re-read.
- `catch_unwind` if you're wrapping a C/C++ library that might panic.

## 8. Wire it into the composition root (if applicable)

If your adapter should be available via the convenience API (e.g., `analyze_file` should detect a new format), wire it in `lib.rs`. Otherwise leave it as a manually-composed building block; consumers can use it directly.

## 9. Update the docs

- Add an entry to `.claude/arch/adapters.md`.
- Add a CHANGELOG entry under `[Unreleased]` describing the new adapter.
- If the adapter changes the public surface, add an ADR.
- If the adapter requires a new feature flag or dep, update `Cargo.toml` and document in the book.

## Example: writing a third-party NlpProvider

The shape an external `vaani-stanza` crate (third-party hypothetical) would take:

```rust,ignore
// In an external crate vaani-stanza:
use vaani::nlp::NlpProvider;
use vaani::domain::{Result, Sentence, Token};

pub struct StanzaProvider {
    pipeline: /* ... */,
}

impl NlpProvider for StanzaProvider {
    fn parse(&self, text: &str) -> Result<Vec<Sentence>> {
        // Call Stanza, translate its output into vaani's Sentence/Token shape.
        // Postconditions to preserve:
        //   - sentences in document order
        //   - tokens id-sorted ascending within each sentence
        //   - exactly one head==0 per sentence
        unimplemented!()
    }
}
```

When enough external `NlpProvider` implementors exist, the `NlpProvider` trait gets extracted into a separately-published `vaani-nlp-api` crate (per [ADR-0004](https://github.com/mox-labs/vaani/blob/main/docs/decisions/0004-stay-single-crate.md)'s re-open conditions). That migration is mechanical because the trait is already minimal and depends only on the domain types.
