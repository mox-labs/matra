# Future direction

Capabilities vaani intends to grow into, with the trigger condition each one waits on.

## Rule evaluation over parsed text structure

A sub-module inside vaani that lets consumers query parse trees with rule-like predicates: matchers over POS sequences, dependency relations, lemma sets, subtrees. The shape is "given a parsed sentence, does it satisfy this rule?" with the rules composable.

**Trigger:** the surface design is settled and a consumer (internal or external) commits to using it.

Until then, the work happens in design documents and prototype code in `.claude/`, not in `src/`.

## WASM / TypeScript crust

A `wasm-bindgen` crust that exposes the same domain types and surface to JavaScript and TypeScript via `serde-wasm-bindgen`. Same Rust core, same methods-don't-cross rule, same boundary checks.

**Trigger:** a TypeScript consumer commits to using it.

## Pdf / Docx decomposers

Behind feature flags (`pdf`, `docx`), implemented as `Decomposer` adapters. The reason they don't exist today is that PDF is a format family, not a format. Half-shipping a PDF decomposer would lock a bad shape into the public surface. The right shape only emerges when a concrete consumer needs PDF support and we can see what they actually need (text extraction? layout preservation? table parsing?).

**Trigger:** a consumer needs PDF or DOCX support with a clear shape we can support without regret.

## Recursive directory walk

`DirectorySource` is non-recursive today. Adding recursion with a depth cap and per-directory `.vaaniignore`-style filtering is a natural extension. The pattern follows ripgrep's `ignore::Walk`: bounded peak memory, per-thread `Ignore` clone if parallel, atomic termination protocol.

**Trigger:** a corpus-level consumer asks for it.

## Streaming source adapters

A `Source` that emits documents asynchronously: filesystem watch (notify), websocket, message queue. The shape requires async, which vaani doesn't have today. The integration point is the `Source` trait + a streaming variant.

**Trigger:** one of:

1. A consumer needs incremental re-analysis on file change (push semantics, not pull).
2. A corpus consumer reports more than 100k documents in regular use, forcing the buffered `analyze_directory` API to OOM.
3. A second `Source` arrives that is inherently push (filesystem watch, websocket, message queue).

Until any one of these fires, the synchronous pull-based pipeline is the correct shape and async/streaming would be premature.

## NlpProvider as a separately published crate

The `NlpProvider` port trait is structurally a candidate for extraction into a minimal `vaani-nlp-api` crate (Pattern 6 from the rust-mastery corpus). The criterion for extraction is the existence of an external implementor ecosystem who needs to pin the contract independently.

**Trigger:** a third-party crate ships a `NlpProvider` implementation (`vaani-stanza`, `vaani-spacy`, etc.) and depends on `vaani` solely for the trait.

When this fires, the migration is mechanical: extract `vaani-nlp-api` (domain types + the trait, no other deps), make `vaani` depend on `vaani-nlp-api`, and supersede [ADR-0004](https://github.com/mox-labs/vaani/blob/main/docs/decisions/0004-stay-single-crate.md) with a new ADR documenting the extraction.

## How triggers work

Each trigger is a falsifiable condition. If the trigger fires across the lifetime of the project, the capability lands. If it doesn't, the capability never ships and that is the correct answer.

This discipline is named in `.claude/arch/evolution.md` and `.claude/skills/aces/SKILL.md`. The shorthand: build the dirt road first, the cobblestone when traffic justifies it, the tarmac when traffic demands it. For consumers, this means the capabilities listed here are commitments, not marketing. Each one ships when the shape is clear and a real use case has pulled it into existence, not before.

## What is explicitly not coming

These have been considered and deliberately rejected, not deferred:

- **Opinionated prose quality scoring.** Vaani measures; consumer code judges. "Is this prose any good?" is not a question the substrate answers.
- **Pattern extractors as part of the default surface.** SVO, copular, prepositional, passive, nominal-modifier patterns are opinions. If they ship, they ship as a sub-module behind a clear "opinionated" boundary, not as part of the default surface.
- **Built-in LLM integration.** Vaani is an NLP substrate, not an LLM client. LLM integration is a consumer concern.
