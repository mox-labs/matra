# vaani

NLP library. Text in, structured analysis out.

vaani is an NLP library for Rust and Python. It provides:

- **UDPipe-based structured parse** — full CoNLL-U annotations (tokens, lemmas, POS tags, dependency trees) over plain text or markdown.
- **Base text metrics** — readability (Flesch-Kincaid), lexical density, compression ratio, vocabulary type-token ratio, nominalization ratio, passive ratio.
- **Summarization** — extractive summary via TF-IDF and TextRank.
- **Keyphrase extraction** — RAKE and YAKE.
- **Rule evaluation over parsed text structure** — *planned*, lands in a later iteration.

The Rust crate is the reference; Python bindings ship via PyO3 and `maturin`. A WASM/TypeScript crust is planned when a JavaScript consumer commits to using it.

## Who this is for

You build something that consumes structured information from prose, and you want a stable substrate that:

- Has the boundary rules of a substrate, not the surface of a framework.
- Won't tell you what good prose looks like — it gives you the trees and the scalars; your code judges.
- Has a small, locked public API that survives across crusts (Rust, Python, future TS).
- Survives bad inputs without aborting your host.

If that's the shape, this is the library.

## Design posture

vaani is built on two non-negotiable disciplines:

- **[ACES](./philosophy.md)** — Adaptable, Composable, Extensible. The structural design philosophy resisting the stasis / drag / opacity cycle every long-lived library faces.
- **Antifragility** — size caps at the gate, panic boundaries at the C/C++ FFI, atomic file writes, TOCTOU-closed model verification, cycle-safe graph walks. The operational discipline that makes vaani fail loud rather than silent.

The library is a public OSS package and an intended exemplar for both Claude-managed open-source repositories and human–AI collaborative intelligence. The bar is high because every public name is a contract across languages.

## How to read this book

If you want to use vaani right now: [Installation](./getting-started/installation.md) → [Quickstart](./getting-started/quickstart.md) → the language-specific page under [Usage](./usage/rust.md).

If you want to understand the shape before committing: [The pipeline](./concepts/pipeline.md) explains the five verbs (ingest, decompose, parse, measure, extract), and [Domain types](./concepts/domain-types.md) walks the data model.

If you want to extend vaani — adding a new format, a new NLP backend, a new metric: [Architecture](./architecture/hex.md) explains the hex layout, and [Writing a new adapter](./extending/new-adapter.md) is the recipe.

If you want to contribute or understand how the project is run: [How this repo is run](./contributing/how-it-works.md) and [The DAO](./contributing/dao.md).
