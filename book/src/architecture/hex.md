# Hex layout

Hexagonal architecture has one rule: dependency direction flows inward. The domain sits at the center. Everything else depends on the domain; the domain depends on nothing except standard library types and a small set of serialization utilities.

The name "hexagonal" is historical: Alistair Cockburn's original diagram happened to have six sides. The shape that matters is concentric: the domain is the innermost ring, ports are the next ring out, adapters are the outer ring, and the composition root is the assembly point that knows all rings simultaneously.

In vaani, this plays out across four concrete rings.

## Ring 1: Domain

`src/domain.rs`. The substrate. All core types live here: `Token`, `Sentence`, `Paragraph`, `Section`, `Document`, `RawDocument`, `Corpus`, `CorpusEntry`, `Keyphrase`, `ScoredSentence`, `Format`, `Error`. The module-level doc comment states the constraint directly: "Dependencies are bounded to serde, thiserror, and std."

The only imports are `serde` for serialization (every domain type is `Serialize + Deserialize`), `thiserror` for the `Error` enum, and `std`. No adapter crate, no NLP library, no HTTP client, no async runtime ever appears here. This is enforced not just by convention but by `cargo check --no-default-features`, which compiles the domain layer with no feature flags active and fails if a disallowed dependency enters.

Why this strictness? Because the domain is the contract between the library and its consumers. A `Document` that depends on `tokio` would force every Rust consumer to take an async runtime, even if they are building a synchronous CLI. A domain that imports `udpipe-rs` would mean swapping the NLP backend requires domain changes. Keeping the domain pure makes it stable and swap-friendly.

## Ring 2: Ports

`src/source/mod.rs`, `src/decompose/mod.rs`, `src/nlp/mod.rs`. Port modules define traits. Each port imports from domain and from nothing else; no port imports another port module. The three port traits are:

- `Source`: reads documents from a path, returns `domain::Result<Vec<domain::RawDocument>>`
- `Decomposer`: splits raw text into sections, returns `Vec<domain::Section>` (infallible)
- `NlpProvider`: parses text into annotated sentences, returns `domain::Result<Vec<domain::Sentence>>`

Ports are the contracts. They express what the domain needs from the outside world, stated entirely in domain terms. `NlpProvider::parse` returns `domain::Sentence`, not `udpipe_rs::Word`. The port does not know that UDPipe exists. The adapter translates.

## Ring 3: Adapters

`src/source/file.rs`, `src/source/directory.rs`, `src/decompose/markdown.rs`, `src/decompose/plain.rs`, `src/nlp/udpipe.rs`, `src/metrics/`, `src/extraction/`. Adapters implement ports and may import external crates. The UDPipe adapter is the only file in the codebase that imports `udpipe_rs`; the boundary-check script (`scripts/check-boundaries.sh`) enforces this in CI and fails if any other file violates the rule.

Adapters import from domain; they do not import from other adapters. A `MarkdownDecomposer` does not import from `FileSource`. Each adapter has exactly the scope it needs to do its job.

Why enforce this so rigidly? Because the UDPipe C library holds non-`Send` C-side state and can panic. The `catch_unwind` call that converts C panics into `Error::ParseFailed` lives inside `nlp/udpipe.rs`. If any other file could import `udpipe_rs`, a C panic anywhere in the codebase could kill the host process. The enforcement keeps the panic boundary at the adapter seam, where it can be handled.

## Ring 4: Composition root

`src/lib.rs`. The only place that knows all adapters and all ports simultaneously. Public functions like `analyze()`, `analyze_markdown()`, `analyze_file()`, and `analyze_directory()` assemble adapters, route through ports, and return domain types. The PyO3 bindings also live here, behind the `python` feature flag. If you want to understand how a call flows from `Vaani.analyze(text)` in Python through the Rust pipeline to a `Document` struct, `src/lib.rs` is the file that shows the full assembly.

The composition root is also the only place where the `udpipe` feature flag appears in assembly logic. When `udpipe` is disabled, the `Udpipe` type is not available; the `no-default-features` build still compiles because the composition root only references `Udpipe` inside `#[cfg(feature = "udpipe")]` blocks.

## Why this matters for the substrate claim

A library that calls itself a substrate must actually be separable from its infrastructure.

The UDPipe adapter holds C-side state. It is the only file that imports `udpipe_rs`. If you want to swap UDPipe for a different NLP provider (a cloud API, a local Rust-native model, a test double like the `DotSplitNlp` used in `src/lib.rs` tests), you implement `NlpProvider` and pass your type to `analyze()`. The domain layer, the port trait, and the composition root's routing logic do not change.

The same logic applies to decomposers. A future `PdfDecomposer` will implement `Decomposer` and live at `src/decompose/pdf.rs`. The domain types, the port trait, and every other adapter are unchanged. The composition root grows one branch.

This is what composable means in practice: the pieces are designed to be swapped independently. The domain is the stable center. The adapters are the replaceable edges.

## Dependency direction at a glance

```
lib.rs (composition root)
  |--- domain.rs                   (serde, thiserror, std only)
  |--- source/mod.rs (port)        (imports domain only)
  |       |--- file.rs (adapter)   (imports domain + std)
  |       |--- directory.rs        (imports domain + file.rs)
  |--- decompose/mod.rs (port)     (imports domain only)
  |       |--- markdown.rs         (imports domain)
  |       |--- plain.rs            (imports domain)
  |--- nlp/mod.rs (port)           (imports domain only)
  |       |--- udpipe.rs           (imports domain + udpipe-rs; ONLY file permitted to)
  |--- metrics/ (adapters)         (import domain + stopwords)
  |--- extraction/ (adapters)      (import domain + stopwords)
```

Arrows point inward. `lib.rs` is the only node that points at multiple rings simultaneously. Every other node has exactly the scope it needs and no more.

See [Ports and adapters](./ports-adapters.md) for the trait signatures and adapter contracts in detail. See [Boundary rules](../reference/boundary-rules.md) for the enforced rules and the CI gates that check them.
