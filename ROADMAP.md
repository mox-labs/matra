# Roadmap

What matra does not do yet, why, and the condition that would change that.

Everything documented in `book/src/` describes what ships today. This file is the only place that describes what does not.


matra follows a discipline borrowed from road-building: dirt road first, then cobblestone when traffic justifies it, then tarmac when the route is established. Each capability below is deferred not because it is unimportant, but because adding it before the trigger condition fires would add complexity that nobody needs yet, or that requires design decisions that need real consumer patterns to make correctly.

The discipline is Chesterton's fence applied to features: before you add a capability, be able to state what problem it solves for which consumer today. If the answer is "we might need this eventually," the fence stays up.

Every entry below names the trigger condition. When the condition is met, write an ADR and proceed.

## Rule evaluation over parsed structure

**What it is.** Declarative rules expressed as predicates over `Document`. A `Rule` names a structural pattern (a dependency arc, a token sequence, a metric threshold) and fires when the pattern is present. The output is a `Finding` with a `SourceSpan` that points back to the bytes in the original text where the rule matched.

This is the capability that bridges the record tier (what matra exposes today) and the abstract tier: relation extraction, modality detection, speech act classification, voice signature analysis. The record tier gives you the tokens and arcs. Rule evaluation gives you the named patterns over them.

**Where it lands.** Inside matra, in a new `src/rules/` module. The vocabulary is locked by ADR-0006: `Rule`, `Predicate`, `Finding`, `SourceSpan`. Rule evaluation is not a separate crate; consumers compose against one surface. The `Finding` type's shape (trait vs enum) is deferred to Phase 2; the name is locked.

**Trigger condition.** At least one concrete consumer pattern that requires rule evaluation cannot be adequately served by direct `Document` field access and application-side logic. The design of `Rule` and `Predicate` must be pulled from real use, not anticipated.

## WASM crust for TypeScript/browser

**What it is.** A `wasm-bindgen` surface that exposes the same `Document` types to TypeScript running in a browser or Node.js environment. The same names cross; the same methods-don't-cross constraint applies. This makes matra usable in frontend applications, browser-based document editors, and Node.js pipelines without requiring a Python or Rust runtime.

**What is blocking it.** The current `NlpProvider` is UDPipe, which uses C FFI compiled to native machine code. C FFI cannot run in a WASM sandbox. A WASM `NlpProvider` requires either a WASM-compiled variant of UDPipe (which has been explored by the upstream project but is not production-ready), a smaller Rust-native model, or a network-backed provider that performs parsing server-side and returns annotated output.

The `NlpProvider` trait requires no change to support a WASM implementation. Any Rust type that can implement `NlpProvider::parse` without C FFI can be used as the WASM backend.

**Trigger condition.** A browser-side or Node.js consumer with a documented requirement for matra analysis without a Python or Rust runtime, paired with an `NlpProvider` implementation that compiles to WASM and delivers sufficient parse quality for the target use case.

## PDF and DOCX decomposers

**What they are.** Adapter implementations of `Decomposer` for `.pdf` and `.docx` source files. The variants already exist in `domain::Format`; `FileSource` already detects `.pdf` and `.docx` extensions correctly. The composition root returns `Error::UnsupportedFormat` for those formats today; there is no decomposer to hand the bytes to.

**What is blocking them.** The Rust PDF extraction ecosystem. PDF text extraction requires handling layout ordering (columns, reading order), encoding diversity (Type1, CFF, CID fonts), and embedded-font subsetting. No current Rust crate handles all of these consistently and without correctness holes on real-world documents. DOCX extraction is more tractable but has not been required by any consumer yet.

**Trigger condition.** A documented consumer requirement for PDF or DOCX analysis, paired with a Rust extraction crate that is stable (post-1.0, maintained, no known correctness holes on the document shapes the consumer needs). When the adapter lands, no port trait changes; only the composition root grows a branch.

## Recursive directory walk

**What it is.** A `DirectorySource` variant that descends into subdirectories rather than stopping at depth one. This enables processing nested document collections (a repository of markdown files organized by directory, for example) without the caller having to walk the tree manually.

**What is blocking it.** No consumer has required it. Recursive walks have non-obvious behavior: symlink cycles, permission errors mid-walk, and large directory trees with heterogeneous content all require explicit policy decisions. What is the depth limit? How are symlinks handled at nested levels? How do per-file errors propagate: do they abort the entire walk or continue? Building these policies into the API without a consumer to validate them risks building the wrong policies.

**Trigger condition.** A consumer that documents a requirement to process nested directory trees, with enough specificity to make the symlink, error-tolerance, and depth-limit policy decisions correctly.

## Streaming source

**What it is.** A `Source` variant that yields documents one at a time rather than collecting all documents into a `Vec<RawDocument>` before returning. This enables processing document collections that exceed available memory, and supports push-semantics integrations where documents arrive from a queue rather than a filesystem.

**What is blocking it.** The current `Source::read` signature returns `Vec<RawDocument>`. A streaming variant requires a different return type: an iterator, a channel, or a `Stream` in the async sense. The async reactor decision (deferred per ADR-0004) is a prerequisite for the most useful form of streaming: async document processing. Without an async reactor, a synchronous iterator-based streaming source is possible but limited in value.

**Trigger condition.** A consumer requiring processing of more documents than can be held in memory simultaneously, or a push-semantics integration (a message queue feeding documents into matra in real time) where the `Vec<RawDocument>` return type is genuinely blocking.

## Separate `matra-nlp-api` port crate

**What it is.** Extraction of the `NlpProvider` trait into a separately published minimal crate, so third-party NLP provider implementors can depend on the port contract without depending on all of matra. This follows the Pattern 6 criterion from the rust-mastery corpus: separately publish a minimal port crate if and only if an external implementor ecosystem exists that needs to pin the contract independently of the main crate's version churn.

**What is blocking it.** No third-party NLP provider implementor crate exists today. The criterion is ecosystem size, not architectural elegance. Extracting `matra-nlp-api` before any external implementor exists would add a publishing burden (two crates to version, two changelogs, two sets of CI) for zero current benefit.

**Trigger condition.** A published Rust crate that depends on matra solely for `NlpProvider` (a `matra-stanza`, `matra-spacy`, or similar, shipped by a non-matra maintainer). At that point, extract `matra-nlp-api` as a minimal port crate, have `matra` depend on it, and keep `matra` as the consumer-facing surface.

---

The pattern across all of these: capability waits for the consumer that justifies it. Adding structure in advance of use is complexity that the next collaborator pays for. The trigger conditions are the guard against that. When one fires, write the ADR, explain why this trigger and not the next one, and proceed.

## HTML report

**What it is.** A self-contained HTML rendering of a `Document`: the parse, the metrics, and the extracted phrases in a form suitable for visual inspection, notebook display, and paper supplementary material. The surface would be `Document::to_html_report()` in Rust, `report(text, format="html")` in Python, and `matra report` on the CLI.

**What is blocking it.** Nothing structural. It is unbuilt because no consumer has needed it yet.

**Trigger condition.** A consumer that needs to eyeball a parse rather than consume it programmatically, most likely a researcher working in a notebook.

## Browser playground

**What it is.** A page where text can be pasted and its structure rendered live, with no install.

**What is blocking it.** It depends on the WASM crust above.

**Trigger condition.** The WASM crust ships.
