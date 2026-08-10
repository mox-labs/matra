# matra — Architecture

NLP library. Text in, structured analysis out. A pure, performant, ACE-aligned NLP library in Rust with Python bindings via PyO3.

## Read in this order

1. **[architecture.md](architecture.md)** — the big picture: single-crate shape, hex layout, composition root, boundary rules, cross-language story.
2. **[domain-model.md](domain-model.md)** — the data shapes everything else depends on. Read this if you are touching any type.
3. **[ports.md](ports.md)** — the three boundary traits: `Source`, `Decomposer`, `NlpProvider`. Read this if you are adding a new format, NLP backend, or input shape.
4. **[adapters.md](adapters.md)** — concrete implementations of the ports.
5. **[evolution.md](evolution.md)** — what's locked, what's allowed to change, what's deferred.

## Pipeline at a glance

```mermaid
flowchart LR
    file[("file or directory")] --> ingest[ingest]
    text[("text")] --> ingest
    ingest --> decompose[decompose]
    decompose --> parse[parse]
    parse --> measure[measure]
    parse --> extract[extract]
    measure --> analysis[("Document")]
    extract --> selections[("ScoredSentence / Keyphrase")]
```

Five verbs. Four sequential, one peer.

- `ingest` reads bytes and produces a `RawDocument`.
- `decompose` breaks a document into structural units (sections, paragraphs).
- `parse` annotates text with linguistic structure (tokens, dep tree).
- `measure` produces scalars (readability, lexical density, compression ratio, TTR, nominalization, passive ratio).
- `extract` produces selections (top sentences via TF-IDF / TextRank, key phrases via RAKE / YAKE).

`measure` and `extract` are peers, not nested. They both consume parsed sentences and produce different kinds of artifact: aggregations vs selections.

## Invariants that hold today

These hold today. See [boundary-rules.md](boundary-rules.md) for how each is (and is not) enforced:

1. `domain.rs` has zero internal dependencies beyond `serde`, `thiserror`, and `std`.
2. Port modules (`source/mod.rs`, `decompose/mod.rs`, `nlp/mod.rs`) import only from `domain`. They do not import each other.
3. Each adapter implements one port. Adapters do not import each other.
4. `nlp/udpipe.rs` is the only file allowed to import `udpipe_rs`.
5. `metrics/` and `extraction/` import only from `domain` and `stopwords`.
6. `cargo check --no-default-features` must compile.
7. The composition root (`lib.rs`) is the only place that knows all adapters and all ports.
8. `tracing` is forbidden in `domain.rs` and port modules (Burner amendment, 2026-04-28).

Enforcement is thinner than this list suggests. Only rule 6 (`cargo check --no-default-features`) is gated in CI. `scripts/check-boundaries.sh` greps three rules (no cross-port import, single `udpipe_rs` importer, no `tracing` in domain or ports) and runs from `just check` and the opt-in pre-commit hook, never in CI. Everything else rests on review. The canonical list, with motivation and failure modes, is [boundary-rules.md](boundary-rules.md).

## ACE: the design discipline

matra is built around three forces, each countering a known decay mode in long-lived libraries.

- **Adaptable** — design for change. `#[non_exhaustive]` on every public enum and every public struct with public fields. Configuration over hardcoding. Feature flags are additive and orthogonal.
- **Composable** — discrete components, clear boundaries, swappable parts. Three ports, multiple adapters per port, one composition root.
- **Extensible** — clear interfaces that invite contribution without requiring full comprehension. Add a new `Source`, `Decomposer`, or `NlpProvider` adapter without reading the rest of the codebase.

matra is **pure**: it does not opine on what good prose looks like. It produces the trees, scalars, and selections; consumer code opines.

matra is **performant**: bounded memory, O(n) algorithms where possible, capped where not. No silent quadratic surprises.

matra is **enabling**: a substrate. Designed so other libraries do meaningful work on top without forking the core.

Names appear in three languages (Rust, Python, TypeScript via WASM when that crust lands) and become public field paths. They are contracts.
