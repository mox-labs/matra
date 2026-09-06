# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!--
Per-release sections follow this shape:

  ## [X.Y.Z] - YYYY-MM-DD

  ### Highlights

  Two to four prose entries explaining the load-bearing changes for
  this release. Each entry teaches the mental model the change is
  built on, not just what shipped. Reserved for architectural decisions,
  breaking changes, security-relevant fixes, and deferred-vs-shipped
  tradeoffs. Bug fixes and minor refactors live in the structured
  sections below, not here.

  ### Added / Changed / Deprecated / Removed / Fixed / Security

  Terse Keep-a-Changelog bullets for everything else.

Style: no em dashes (project convention).
Rollover: scripts/changelog-release.sh moves [Unreleased] -> [X.Y.Z]
at release time. It does not touch Cargo.toml or pyproject.toml; bump
those by hand.
-->

## [Unreleased]

### Added

- `config::Config`: a composition-root value that resolves where things are and what the defaults are, per key, from an explicit argument, then the environment, then the config file, then the defaults compiled into the crate. It carries locations and defaults and never behavior (ADR-0011): the model directory, the semantic threshold, and the default counts and algorithm names, all of which a caller could pass as arguments instead. Paths follow the XDG conventions on Linux and macOS (`$XDG_CONFIG_HOME/matra/config.toml`, `$XDG_DATA_HOME/matra`), overridable with `MATRA_CONFIG_FILE`, `MATRA_DATA_DIR` and `MATRA_MODEL_DIR`. An existing, non-empty `~/.matra/models` is used as the model directory when the new location does not exist: matra never creates `~/.matra`, but when an existing, non-empty legacy cache is selected it is used as the model directory, downloads and re-downloads included, and creating the new location or setting `MATRA_MODEL_DIR` moves off it. An empty leftover `~/.matra/models` is not selected. A missing config file is not an error; a malformed one, an unknown key, or an algorithm name this build does not know is `Error::InvalidInput` naming the file and the key or line. `Config::sources` reports which rung every value came from as a `ValueSource`, and `Config::from_sources` takes the environment and the file contents as arguments so no test reads the developer's home.
- `Config::with_model_dir`: the argument rung, and the only thing that produces `ValueSource::Argument`. It returns the same configuration with the model directory replaced and that one key's source changed, so a caller handed a directory (a command line's `--model-dir`, say) can layer it on top without losing the provenance of every other value.
- `config/default.toml`: the defaults, shipped inside the crate and embedded with `include_str!`. A user config file overrides it key by key.
- `Udpipe::from_config`, `Engine::from_config`, and `Engine::with_defaults`: the no-setup path, additive to the existing constructors, which are unchanged. `Engine::with_defaults()` is `Config::resolve()` followed by `Engine::from_config`.
- Python: `Matra.english()` takes the model directory optionally. With no argument it resolves through `Config` exactly as the Rust surface does.
- `toml` joins the default dependency tree, parse and serde features only. Pure Rust; the wasm32 job confirms the core and the model2vec adapter still compile for `wasm32-unknown-unknown` with it in the tree.
- `embed` port: the `Embedder` trait (one method, `embed`, batch in, vectors out, length- and dimension-uniform by contract) and the `domain::Embedding` carrier, a serde-transparent newtype over `Vec<f32>`. Tier 2 channel discipline per ADR-0010: nothing derived from embeddings becomes a field on the deterministic pipeline's types.
- `extraction::semantic_clusters` and the `SemanticClusters` / `SemanticCluster` / `SemanticEdge` domain types: connected-component clustering over precomputed sentence embeddings, edges carried in the type so co-membership is never mistaken for pairwise similarity, singletons excluded by construction, model identity and threshold carried as provenance. Pure over domain values; tested without any model. New `Error::InvalidInput` variant for contract violations, routed to Python `ValueError`.
- `embed_and_cluster`: the composition-root pairing of a `Document` with an `Embedder` (embed the sentences, cluster the vectors, attribute the scores to the embedder's own identity). `Embedder` gains the `identity` method that makes misattribution impossible.
- Python: `Model2Vec` (load a static model, read its hash and dimensions, `embed` texts directly for the UDPipe-free path), `Matra.semantic_clusters(text, threshold, model)`, and the vectors-in module function `semantic_clusters(embeddings, threshold, model_hash)`; wheels now build with the `model2vec` feature. The FFI shape fixture lands at `spec/tests/semantic/clusters.json` with Rust and Python runners, comparison done in f32 space.
- `model2vec` feature: a static-embedding adapter loading the model2vec artifact format (safetensors matrix, tokenizer.json, config.json), caller-supplied with no network, hashed on load for provenance. Pure-Rust closure verified on wasm32 (a new CI job holds the line). Inference is a gather, mean pool, and optional L2 normalize, parity-tested against the Python reference; f16/i8 artifacts are rejected loudly (f32 only in this build). Panics in the parsing paths convert to `Error::ModelInvalid` at the adapter boundary.

### Changed

- Docsite: an Explanation section (concepts, situation model, programming model, pragmatics) now precedes the guides; the capabilities page is a straight per-tier reference; architecture and boundary rules move under Contributing. Existing pages lose meta-commentary and restatement without losing facts. The domain-model page now records `passive_ratio` as a stored field that crosses to Python, which it has been since 0.1.0.

## [0.1.0] - 2026-08-21

First release. The surface this version freezes is the one pipeline
(ADR-0007) plus the five structural primitives (ADR-0008): everything
below happened pre-publish, which is why none of it carries a
deprecation.

### Highlights

**One pipeline replaced six entry points.** The old surface enumerated its calling conventions: `analyze`, `analyze_markdown`, `analyze_file`, `analyze_directory`, `parse`, and `analyze_from` were partial applications of one chain, and each restated invariants the compiler checked in none of them. Two live defects had that exact shape: the input size cap was bypassable from Python because four methods restated it and four did not, and `analyze_from` returned a half-populated `Document` because the metric suite carried the sentence set twice. The surface is now `Ingest` (a string is a stream of one, a directory is a stream of many) into `Engine` (`analyze`, or the stages `annotate` and `compose`). `annotate` is the only route from text to the parser, so the size cap is a property of the pipeline rather than of each entry point, and seven equivalence laws in the test suite pin the grains together. Streams are lazy: a directory holds one document's allocations at a time, and per-file failures travel as `DocumentError` items instead of aborting the walk.

**Structural primitives cross FFI as fields.** Five rule-substrate primitives landed: negation cues, modal auxiliaries with the bare-assertion discriminator, reporting constructions and root adverbials for evidentiality, and the six Hearst hypernymy patterns as span pairs. The question they forced, whether a derived structural fact is a method or a field, is settled by ADR-0008: a derivation is computed once in Rust at a pipeline choke point and crosses every FFI boundary as serialized data, while views over data already crossing (ADR-0009's `Token::feat` lookup) stay Rust-only methods. Before this, the Python CLI re-implemented passive detection against a parse Rust had already read; that duplication is deleted, and `spec/tests/` fixtures now pin each crossing primitive across crusts. Every primitive reports structure (the cue, the arc, the construction, the span pair) and leaves interpretation to the consumer, which is the substrate line the rule vocabulary will be designed against.

**The project is now called matra.** The previous name collided with an existing package on PyPI, which makes dual publishing to crates.io and PyPI under one name impossible. The name is the public contract across Rust, Python and a future TypeScript crust, so the collision had to be resolved before the first release rather than after. There are no consumers and nothing has been published, so the change carries no aliases, shims or deprecations.

**A command-line interface ships behind the `cli` feature.** The library returns typed errors and structured data; the binary decides rendering, exit codes, and what to do when input is missing. Exit codes follow the ripgrep convention, so nothing-found is 1 and a genuine failure is 2, and an empty document is not an error. `summarize` and `keyphrases` route through the same format detection `analyze` uses, so markdown headings and fenced code are never ranked as prose.

**Conformance fixtures now bind the crusts together.** matra ships one Rust core behind several bindings that all call the same parser, so a difference between them is never a difference of behaviour: it is a binding defect, a renamed field or a value that lost precision crossing over. `spec/tests/*.json` holds language-agnostic fixtures with one runner per language. The UDPipe model is part of the contract, so a model version change is a spec change.

### Added

- `Ingest`, `Engine`, `standard_decomposers`, `decompose::Decomposers`: the pipeline surface.
- `domain::DocumentError` (per-document failure with its path) and `domain::CorpusResult` (the partition of a result stream, constructed by `collect()`).
- `Format` derives `PartialEq` and `Eq`, which the decomposer table keys on.
- Equivalence laws L1 to L7 as tests: chain homomorphism, empty, singleton injection, stage composition, partition, Err passthrough, and the size-cap bound on everything the parser sees.
- `matra` binary behind the `cli` feature: `analyze`, `summarize`, `keyphrases`, each accepting `--json`.
- Conformance suite: `spec/tests/*.json` with Rust and Python runners.
- `tests/cli.rs` covering argument handling, format detection, output shape and exit codes.
- `rust-toolchain.toml` pinning stable. The MSRV claim is verified separately by CI.
- `ROADMAP.md`, the single register of unbuilt capability and its trigger conditions, rendered into the book.
- `book/src/plans/`, the iteration plans, with per-milestone rubrics.
- Docsite floor gate 5: no em dashes outside quoted material.
- `domain::Negation` and `Sentence.negations`: negation cues (`not`, `never`, `no`, `neither`, `nor`) detected from the dependency graph at sentence construction and serialized with the sentence, so every crust reads one Rust detection (ADR-0008).
- `Document.passive_ratio` as an `Option<f64>` slot filled by the metric suite, beside `vocabulary_ttr` and `nominalization_ratio`; the aggregate now crosses FFI as data, and the Python CLI reads it instead of re-deriving passive detection from raw tokens.
- ADR-0008: derived structural facts cross FFI as serde-visible fields with a single Rust implementation; zero-information accessors over data already on the wire stay Rust-only methods.
- Conformance fixture `spec/tests/negation.json` pinning per-sentence negation cues and `passive_ratio` across crusts.
- `domain::Modal` and `Sentence.modals`: modal auxiliaries detected at sentence construction and serialized with the sentence (ADR-0008 mechanism). The closed class is the ten UD English `MD` lemmas (`can`, `could`, `may`, `might`, `must`, `ought`, `shall`, `should`, `will`, `would`), matched by lemma on the `aux` relation or the `AUX` part of speech, which catches modals the model promotes to root or `conj` under VP ellipsis and coordination; the epistemic, deontic or dynamic reading stays with the consumer.
- `Sentence.bare_assertion`: the bare-assertion discriminator, true when the root clause is finite indicative (`Mood=Ind` on the root or on a `cop`/`aux`/`aux:pass` child of it, covering copular, do-support and passive clauses) and no modal auxiliary governs it. Reads the root clause via `Token::feat`.
- Conformance fixtures `spec/tests/modal.json` and `spec/tests/modal-coordination.json` pinning per-sentence modals and the bare-assertion discriminator across crusts, including VP ellipsis, coordinated auxiliaries and the copular bare assertion.
- `domain::Reporting` and `Sentence.reportings`: reporting constructions detected at sentence construction and serialized with the sentence (ADR-0008 mechanism). The construction is structural (a verb governing a `ccomp`, plus its `nsubj` when the sentence has one) and fires for every verb that fills it: reporting verbs are an open class, so matra ships no lexicon and `Sentence::reportings_in` takes the caller's as a parameter. The subject is optional because UDPipe splits "Smith et al. reported ..." at the period in "et al.", stranding the attribution in the previous sentence; the upstream defect is recorded in the test suite, not fixed here.
- `domain::RootAdverbial` and `Sentence.root_adverbials`: adverbial modifiers attached to the root, the arc sentence-scope adverbs ("Reportedly, ...") land on, detected at sentence construction and serialized with the sentence. The parse does not distinguish sentence scope from manner, so every root-attached `advmod` is reported and `Sentence::root_adverbials_in` filters by the caller-supplied lexicon; which lemmas read as evidential stays the consumer's call.
- Conformance fixture `spec/tests/evidentiality.json` pinning per-sentence reporting constructions and root adverbials across crusts: self-attribution ("We show that"), other-attribution ("Smith reported that"), impersonal ("These results suggest that"), and the hearsay adverb ("Reportedly, ...").
- `hearst` module, `domain::HearstPair`, `domain::HearstSpan`, `domain::HearstPattern`, and `Sentence.hearst_pairs`: the six Hearst (1992) hypernymy patterns (`NP such as NP`, `such NP as NP`, `NP, including NP`, `NP, especially NP`, `NP and other NP`, `NP or other NP`) detected as dependency-arc patterns over the parse, not regex over surface text. Each pair carries the pattern tag plus hypernym and hyponym spans referencing token ids, so provenance holds. Precision over recall: each detector requires the full arc shape verified against live parses, so clausal coordination, definite contrastive coordination ("the teacher and the other students"), and emphasized plain coordination do not fire and misparses are missed rather than misreported. The detector lives outside the domain and the pipeline fills the field at the annotate stage; the pair is a candidate, not an asserted taxonomy edge.
- Conformance fixture `spec/tests/hearst.json` pinning Hearst pairs across crusts: the `such_as` and `and_other` patterns plus a clausal-coordination hard negative pinned to an empty result.
- `Token::feat`: borrowed lookup of one morphological feature in the CoNLL-U `feats` string, first exact-key match, no allocation. Rust-only by design since `feats` already crosses FFI as a string (ADR-0009).
- ADR-0009: feats access is a lookup accessor, not an exhaustive enum and not a per-token map; derivations cross as fields, views over crossing data stay methods.

### Removed

- The six entry points: `analyze`, `analyze_markdown`, `analyze_file`, `analyze_directory`, `analyze_from`, `parse`. Variation lives in `Ingest`'s constructors and the decomposer table, not the function namespace. Pre-publish, so no consumer breaks.
- `Metric`'s sentence-slice parameter. Metrics read the one sentence set attached to the document's paragraphs, which removes the redundant representation behind the `analyze_from` half-population defect.
- `DirectorySource::read_collecting_errors`, orphaned by `analyze_directory`'s removal; `Ingest` yields per-file failures as stream items.

### Changed

- The four Python extraction methods route through the pipeline, so the 8 MiB input gate, plain-text decomposition, and paragraph-scoped parsing apply to them uniformly.
- Documentation rebuilt around what a reader needs first: what matra returns, the type graph, and how a call runs. The architecture page is written from source and organised by the call path rather than by the pattern it happens to instantiate.
- Diagrams are hand-authored inline SVG. Mermaid is not installed; `book/book.toml` records the rule for choosing between them and the command to restore it.
- Architecture prose consolidated into the book, which is gated, and out of `.claude/`, which is not.

### Fixed

- `cargo metadata` reported seven public features where three were intended. Bare optional-dependency names in `[features]` mint public features implicitly; the `dep:` prefix closes that.
- `summarize` and `keyphrases` read files raw and parsed them as plain text, so markdown headings and fenced code were ranked as prose. Regression test added.
- Documentation described `analyze_from` as taking a `Source` and a `Decomposer`. It takes neither: it is the parse-once entry point.
- `metrics::default_suite` claimed to return metrics in dependency order. There is no inter-metric dependency; each reads only `Document` state and writes distinct slots.
