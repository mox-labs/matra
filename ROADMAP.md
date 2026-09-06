# Roadmap

What matra does not do yet, why, and the condition that would change that.

Every other page of this documentation describes what ships today. This page is the only one that describes what does not, which is what lets you trust the rest without auditing each claim.


matra follows a discipline borrowed from road-building: dirt road first, then cobblestone when traffic justifies it, then tarmac when the route is established. Each capability below is deferred not because it is unimportant, but because adding it before the trigger condition fires would add complexity that nobody needs yet, or that requires design decisions that need real usage patterns to make correctly.

The discipline is Chesterton's fence applied to features: before you add a capability, be able to state what problem it solves for which caller today. If the answer is "we might need this eventually," the fence stays up.

Every entry below names the trigger condition. When the condition is met, write an ADR and proceed.

## The scoping principle: lowest verifiable tier

A survey of the computational landscape (internal, 2026-08-21) sorts NLP affordances into three tiers: deterministic tooling whose output is checkable against the source (tokenizers, morphology, dependency parsers, document-structure extraction), specialist models (coreference, discourse relations), and general model judgment, which has the widest coverage and the lowest verifiability. The engineering rule it supports: route each operation to the lowest tier that meets the accuracy and verifiability requirement, and escalate only when the cheaper tier provably fails.

matra is the first tier, deliberately. Everything it ships is deterministic and grounds back to the bytes it came from, which is what makes its output usable as data rather than as interpretation. Two consequences bound the roadmap:

**Capabilities with no reliable affordance stay out, and the gap is recorded rather than simulated.** Pragmatic enrichment (implicature, purport, illocutionary force) and claim or fallacy certification have no deterministic path and no reliable model path either; the honest output there is a structural gap marker, not simulated confidence. matra's idiom already encodes this (`Option` slots that mean "declined to compute", the `usize::MAX` cycle sentinel, `UnsupportedFormat`, `DocumentError`), and new capabilities inherit it.

**Capabilities that need a specialist model are adapter work, not extensions of the parse.** Coreference is the clearest case: it sits a tier above UDPipe, the research synthesis places it above matra beside SRL and NLI, and pretending otherwise would put unverifiable output behind a surface that promises verifiability. If a specialist coreference adapter ever lands, it arrives as a new `NlpProvider` implementation behind its own feature flag, with its tier stated plainly.

## Rule evaluation over parsed structure

**What it is.** Declarative rules expressed as predicates over `Document`. A `Rule` names a structural pattern (a dependency arc, a token sequence, a metric threshold) and fires when the pattern is present. The output is a `Finding` with a `SourceSpan` that points back to the bytes in the original text where the rule matched.

This is the capability that bridges the record tier (what matra exposes today) and the abstract tier: relation extraction, modality detection, speech act classification, voice signature analysis. The record tier gives you the tokens and arcs. Rule evaluation gives you the named patterns over them.

**Where it lands.** Inside matra, in a new `src/rules/` module, occupying the `abstract` extension point ADR-0007 reserves between structure and purpose-fitted output. The vocabulary is locked by ADR-0006: `Rule`, `Predicate`, `Finding`, `SourceSpan`. Rule evaluation is not a separate crate; callers compose against one surface. The `Finding` type's shape (trait vs enum) is deferred to Phase 2; the name is locked. Rules are deterministic predicates over structure, so their findings stay at the verifiable tier: a `Finding` names the pattern and points at the bytes, and whether the pattern matters remains the caller's judgment.

**Trigger condition. FIRED, 2026-05-23.** The condition was at least one concrete caller pattern that direct `Document` field access and application-side logic cannot adequately serve.

An internal research synthesis dated 2026-05-23 names five such patterns and states the case directly: "these five primitives *are* the consumer sites. The deferral has fired." They are negation, modal classification, evidentiality, Hearst patterns, and typed morphological features, each grounded in a distinct literature (FactBank polarity, CoNLL-2010 hedges, Aikhenvald evidentiality, Hearst 1992).

A second, sharper piece of evidence sat inside this repository. `Sentence::is_passive` was a method, and methods do not cross FFI, so `python/matra/cli.py` re-implemented passive detection over raw tokens: matra's own language binding duplicated its own primitive. ADR-0008 settled the channel (derived structural facts cross FFI as fields, computed once in Rust at a single point in the pipeline) and the Python re-implementation is deleted.

**The five primitives shipped, 2026-08-21**, in [`book/src/plans/i7-structural-primitives.md`](https://github.com/mox-labs/matra/blob/main/book/src/plans/i7-structural-primitives.md), which carries the milestones, their rubrics, and the reasoning trail. `Rule` and `Predicate` are designed after them, from the shape the five actually took, which is what "pulled from real use, not anticipated" asked for.

**What the five revealed about that shape.** Written from the landed code, not from anticipation. A `Predicate` must be able to name:

- **A single dependency arc, selected by relation and lemma.** Negation is one `advmod` arc carrying a closed-class cue; a modal is one `aux` arc whose lemma sits in the ten-item UD closed class, widened by the `AUX` part of speech to catch modals the parser promotes under ellipsis and coordination. Both landed as structs of token ids and lemmas (`Negation`, `Modal`). Arc-plus-lemma is the smallest pattern unit, and two of the five primitives are exactly one of them.
- **A morphological feature at a tree position.** The bare-assertion discriminator reads `Mood` through `Token::feat` on the root or on a `cop` or `aux` child of it. A predicate vocabulary with arcs but without feats lookups cannot express the discriminator that separates asserted from modalized clauses.
- **A multi-arc construction with optional participants.** The reporting construction is a verb governing a `ccomp`, plus its `nsubj` when the sentence has one (`Reporting`). The subject slot is optional because sentence segmentation strands attribution across boundaries; a construction predicate must match with an absent argument rather than fail.
- **A lexicon as a parameter, for open classes.** Reporting verbs and evidential adverbs have no closed list, so the detectors report every structural match and `Sentence::reportings_in` and `Sentence::root_adverbials_in` filter by a caller-supplied lexicon. A `Rule` over an open class carries its lexicon as data; embedding one would ship an incomplete list that looks authoritative.
- **Span pairs with token-id traceability, as output.** A Hearst match returns a pattern tag plus two spans, each a head token and a token-id range (`HearstPair`, `HearstSpan`). `SourceSpan` therefore has to address a range, not a single token, and a `Finding` has to be able to carry more than one of them.

Two mechanism facts carry over. Predicates read materialized fields, not the raw parse: ADR-0008 computes each derivation once at a single point in the pipeline and serializes it, so rule evaluation is field access over `Document` before it is graph walking. And that line held under real pressure: every discriminant that landed names a construction (`HearstPattern` variants, modal lemmas, the reporting shape), never a semantic judgment, which is exactly the Frame-3 contract ADR-0006 imposes on `Finding`.

## WASM binding for TypeScript/browser

**What it is.** A `wasm-bindgen` surface that exposes the same `Document` types to TypeScript running in a browser or Node.js environment. The same names cross; the same methods-don't-cross constraint applies. This makes matra usable in frontend applications, browser-based document editors, and Node.js pipelines without requiring a Python or Rust runtime.

**What is blocking it.** The current `NlpProvider` is UDPipe, which uses C FFI compiled to native machine code. C FFI cannot run in a WASM sandbox. A WASM `NlpProvider` requires either a WASM-compiled variant of UDPipe (which has been explored by the upstream project but is not production-ready), a smaller Rust-native model, or a network-backed provider that performs parsing server-side and returns annotated output.

The `NlpProvider` trait requires no change to support a WASM implementation. Any Rust type that can implement `NlpProvider::parse` without C FFI can be used as the WASM backend.

**Trigger condition.** A browser-side or Node.js caller with a documented requirement for matra analysis without a Python or Rust runtime, paired with an `NlpProvider` implementation that compiles to WASM and delivers sufficient parse quality for the target use case.

## PDF and DOCX decomposers

**What they are.** Adapter implementations of `Decomposer` for `.pdf` and `.docx` source files. The variants already exist in `domain::Format`; `FileSource` already detects `.pdf` and `.docx` extensions correctly. The composition root returns `Error::UnsupportedFormat` for those formats today; there is no decomposer to hand the bytes to.

**What is blocking them.** The Rust PDF extraction ecosystem. PDF text extraction requires handling layout ordering (columns, reading order), encoding diversity (Type1, CFF, CID fonts), and embedded-font subsetting. No current Rust crate handles all of these consistently and without correctness holes on real-world documents. DOCX extraction is more tractable but has not been required by any caller yet.

**Trigger condition.** A documented caller requirement for PDF or DOCX analysis, paired with a Rust extraction crate that is stable (post-1.0, maintained, no known correctness holes on the document shapes the caller needs). When the adapter lands, no port trait changes; only the composition root grows a branch.

## Recursive directory walk

**What it is.** A `DirectorySource` variant that descends into subdirectories rather than stopping at depth one. This enables processing nested document collections (a repository of markdown files organized by directory, for example) without the caller having to walk the tree manually.

**What is blocking it.** No caller has required it. Recursive walks have non-obvious behavior: symlink cycles, permission errors mid-walk, and large directory trees with heterogeneous content all require explicit policy decisions. What is the depth limit? How are symlinks handled at nested levels? How do per-file errors propagate: do they abort the entire walk or continue? Building these policies into the API without a caller to validate them risks building the wrong policies.

**Trigger condition.** A caller that documents a requirement to process nested directory trees, with enough specificity to make the symlink, error-tolerance, and depth-limit policy decisions correctly.

## Streaming source

**What it is.** A `Source` variant that yields documents one at a time rather than collecting all documents into a `Vec<RawDocument>` before returning. This enables processing document collections that exceed available memory, and supports push-semantics integrations where documents arrive from a queue rather than a filesystem.

**What is blocking it.** The current `Source::read` signature returns `Vec<RawDocument>`. A streaming variant requires a different return type: an iterator, a channel, or a `Stream` in the async sense. The async reactor decision (deferred per ADR-0004) is a prerequisite for the most useful form of streaming: async document processing. Without an async reactor, a synchronous iterator-based streaming source is possible but limited in value.

**Trigger condition.** A caller requiring processing of more documents than can be held in memory simultaneously, or a push-semantics integration (a message queue feeding documents into matra in real time) where the `Vec<RawDocument>` return type is genuinely blocking.

## Separate `matra-nlp-api` port crate

**What it is.** Extraction of the `NlpProvider` trait into a separately published minimal crate, so third-party NLP provider implementors can depend on the port contract without depending on all of matra. This follows the Pattern 6 criterion: separately publish a minimal port crate if and only if an external implementor ecosystem exists that needs to pin the contract independently of the main crate's version churn.

**What is blocking it.** No third-party NLP provider implementor crate exists today. The criterion is ecosystem size, not architectural elegance. Extracting `matra-nlp-api` before any external implementor exists would add a publishing burden (two crates to version, two changelogs, two sets of CI) for zero current benefit.

**Trigger condition.** A published Rust crate that depends on matra solely for `NlpProvider` (a `matra-stanza`, `matra-spacy`, or similar, shipped by a non-matra maintainer). At that point, extract `matra-nlp-api` as a minimal port crate, have `matra` depend on it, and keep `matra` as the caller-facing surface.

---

The pattern across all of these: capability waits for the caller that justifies it. Adding structure in advance of use is complexity that the next collaborator pays for. The trigger conditions are the guard against that. When one fires, write the ADR, explain why this trigger and not the next one, and proceed.

## HTML report

**What it is.** A self-contained HTML rendering of a `Document`: the parse, the metrics, and the extracted phrases in a form suitable for visual inspection, notebook display, and paper supplementary material. The surface would be `Document::to_html_report()` in Rust, `report(text, format="html")` in Python, and `matra report` on the CLI.

**What is blocking it.** Nothing structural. It is unbuilt because no caller has needed it yet.

**Trigger condition.** A caller that needs to eyeball a parse rather than consume it programmatically, most likely a researcher working in a notebook.

## Browser playground

**What it is.** A page where text can be pasted and its structure rendered live, with no install.

**What is blocking it.** It depends on the WASM binding above.

**Trigger condition.** The WASM binding ships.

## Voice fingerprint metrics

**What it is.** Three additions that make matra's existing metrics usable as a stylometric signature rather than as per-document readouts: a length-normalized lexical diversity measure, burstiness as a first-class value, and a contraction count.

**Why these three specifically.** An internal note dated 2026-04-21 records a writing-assistant agent blocked on a calibration loop against a measured baseline: 26.6 word mean sentence length, 36.9 percent passive, 0 percent contractions, 7.7 percent nominalisation, 0.76 burstiness. Checked against what ships, three of those five are already `Document` methods or fields. Burstiness is derivable from `sentence_length_std` and the mean but is not exposed. Contractions are not counted at all. That note estimated the remaining work as a small configuration layer rather than a rewrite.

**One of them is a defect, not an addition.** `vocabulary_ttr` is a raw type-token ratio and TTR falls mechanically as text grows. Measured on this repository: README scores 0.690 against `architecture/design.md` at 0.227, but README has 35 sentences and the other has 266, so most of that gap is length rather than voice. As a cross-document feature it is currently unsound, which matters most for `analyze_directory`, whose whole purpose invites the comparison. Either a length-normalized measure lands beside it (MTLD, MATTR, or standardized TTR) or the limitation is documented on the type. Doing neither leaves a trap.

**Why this is the strongest candidate of anything on this page.** The four faces of voice all map onto shipping output with no hole: agentive onto `nsubj` and `nsubj:pass`, modal onto `aux` and `feats`, structural onto section hierarchy and sentence length, stylistic onto lexical density and compression. That is unlike claim atomization, which needs an LLM, and unlike deriving a text's situation model, for which no shipping output is a foundation at all.

**Trigger condition.** Met in substance: a named caller with a documented baseline exists and has been blocked for months. What is missing is confirmation that the caller still wants matra rather than the thin-wrapper alternative it also considered. Confirm that, then proceed.

## Self-similarity and redundancy metrics

**What it is.** Structural detection of a document repeating itself: clusters of sentences restating the same content, with quantitative measures over them. The concrete caller pattern is auditing LLM-generated text, whose characteristic failure is high-lexical-overlap restatement.

The shape, all deterministic and traceable back to the source bytes:

| Output | Kind |
|---|---|
| Similarity clusters: sentence groups whose pairwise content-lemma overlap (Jaccard or TF-IDF cosine) exceeds a caller-supplied threshold, returned as span sets with the shared lemmas as evidence | qualitative |
| Redundancy ratio: share of sentences in clusters of size above one | quantitative |
| rep-n and distinct-n: n-gram repetition rates, the established NLG-literature measures | quantitative |
| Skeleton repetition: sentences sharing root-verb lemma plus subject and object lemmas | qualitative |
| Opener formulae: repeated sentence-initial lemma sequences | quantitative |
| Document-scope compression ratio, closing the per-paragraph measure's cross-paragraph blind spot | quantitative |
| POS-sequence compression ratio: the same compression measure over the tag sequence, catching structural repetition independent of vocabulary (Shaib et al. 2024) | quantitative |
| Syntactic template rate: recurring POS n-grams, the published operationalization of templated LLM output (Shaib et al., EMNLP 2024) | quantitative |
| Span recurrence: long token spans repeating verbatim, the loop-degeneration signature | qualitative |

matra reports the clusters and the numbers; whether they constitute fluff is the caller's reading. The word never appears in the output. That restraint is now empirical as well as principled: the 2026 slop-measurement literature found repetition and templatedness are weak predictors of human quality judgments on their own, so these are structural facts for a caller's argument, never a judgment. Synonym-level paraphrase (different vocabulary, same meaning) is out of reach of lexical overlap by design; catching it needs semantic similarity, which sits above the verifiable tier. That half shipped 2026-09-05 via [i9](https://github.com/mox-labs/matra/blob/main/book/src/plans/i9-embeddings-adapter.md): the `Embedder` port, the model2vec static adapter, and `semantic_clusters` (see the [semantic clusters guide](https://github.com/mox-labs/matra/blob/main/book/src/guides/semantic-clusters.md)). The deterministic family above remains the open half of this entry. Published similarity thresholds for that task span 0.67 to 0.9 with no consensus, which is why every threshold here is caller-supplied.

**Where it lands.** Mostly paid for already: `extraction/textrank.rs` builds a pairwise sentence-similarity matrix and projects it down to centrality ranks; re-projecting the same matrix as clusters is the core of this capability. Thresholds are caller-supplied parameters, not constants matra pretends to know.

**Trigger condition. FIRED, 2026-08-21.** A concrete caller pattern was named that field access cannot serve: quantifying restatement across a document for LLM-output auditing. Design against I7's primitives and the rule-vocabulary shape; an ADR settles whether this is a metric family, an extractor, or the first rule pack.

## Information density

**What it is.** Deterministic measures of how much a text says per word, the complement of the redundancy family: redundancy catches the same thing said twice, density catches little said at length. The anchor is propositional idea density, a validated psycholinguistics measure (propositions per word) with two published computational forms. CPIDR counts propositions from Penn Treebank tags, which UDPipe already emits in `Token::xpos`; DEPID counts them from dependency relations, which is matra's core output, and agrees with human raters slightly better than CPIDR does. DEPID-R, the variant counting distinct relation-lemma triples rather than tokens of them, catches fluent-but-repetitive text and costs nothing extra once lemmas are in hand. Around the anchor sit smaller settled measures that are direct tree walks: mean dependency distance, modifiers per noun phrase, words before the main verb, content-to-function ratio.

**Where it lands.** Metric functions over `Document`, same family as the shipping metrics suite. The DEPID relation inventory is Stanford-style, so an explicit mapping to UD relations is part of the design; the CPIDR path additionally holds only for English models, since `Token::xpos` carries whatever tagset the provider's model emits and PTB tags are the English case. Three counting rules the literature leaves genuinely unsettled (direct objects, coordinating conjunctions, the denominator) get settled by ADR rather than inherited silently, and the ADR also states the non-English story. The reference implementations are GPL, so the derivation is clean-room from the published rules, and exact numeric parity with them is a non-goal. No implementation of idea density exists for CoNLL-U input in any language; this would be the first.

**What stays out, and why.** Hedging and booster density need a lexicon, and no canonical machine-readable hedge list exists; per the open-class precedent (`reportings_in`), any such lexicon is caller-supplied data, never shipped as if authoritative. Surprisal needs frequency norms or a language model, and sycophancy detection has no deterministic path at all; both gaps are recorded rather than simulated.

**Trigger condition. FIRED, 2026-09-04.** The same LLM-output-auditing caller pattern that fired the redundancy entry names low density as the second failure mode, and the 2026-09-04 landscape survey (internal, not in this repository) established implementability from matra's existing parse output. Design together with the redundancy family; the same ADR should place both.

## Configuration-driven invocation

**What it is.** A configuration file selecting which metrics run, which extractors run, and how output is shaped, so both the library and the CLI are driven by declaration rather than by argument lists.

**What is blocking it.** Nothing structural, and the extension point already exists. `default_suite()` returns `Vec<Metric>` and `run_suite` is public, so choosing a suite is already choosing the contents of a vector. What is absent is the surface: a format, a resolution order, and a decision about whether configuration is a library concern or belongs only to the application tier.

That last question is the real one. matra's discipline is that the library returns typed data and the binary decides presentation. A config format that reaches into the library risks putting policy where the composition root belongs.

**Shipped, 2026-09-05**, via [i10](https://github.com/mox-labs/matra/blob/main/book/src/plans/i10-foundations.md): `Config` resolves locations and defaults per key from argument, environment, config file, then the defaults compiled into the crate; `Engine::with_defaults()`, `Matra.english()` and the `from_config` constructors are the no-setup path on every surface; `matra config show` and `matra config init` are the command-line half. The question this entry asks is answered narrowly: the file carries locations and defaults, never which metrics run or how output is shaped. Selecting behavior from a file remains unbuilt, and reopening it needs its own ADR rather than a wider schema here.

**Trigger condition. FIRED, 2026-09-05.** The condition was a caller running matra repeatedly with the same non-default selection, with agent-driven use the likely first instance. It fired as owner direction: matra works with no setup on every surface, follows developer-tool conventions for config and paths, and keeps Rust as the core with Python and TypeScript as thin reach layers. [ADR-0011](https://github.com/mox-labs/matra/blob/main/docs/decisions/0011-out-of-the-box.md) settles the library-or-application question the paragraph above asks (a resolver for locations and defaults in the library, behavior selection in the application), and [`book/src/plans/i10-foundations.md`](https://github.com/mox-labs/matra/blob/main/book/src/plans/i10-foundations.md) carries the milestones.

## Agent surface

**What it is.** A `--skill` flag on the CLI that prints a self-contained description of matra's semantics for an agent: what it is for, when to reach for it, the incantations with their JSON shapes, how to read the numbers, and the limits. Progressive disclosure follows the shape of a skill on disk: `--skill` prints the short top level, `--skill -r <name>` prints one deeper reference. `--help` stays the framework-generated reference for humans. The same file is what a plugin marketplace distributes.

**Why a flag and not only a docs page.** Human attention is the scarce input. The people who would read the whole docsite are few; the agents that will run matra on their behalf are many, and an agent that can print the semantics it needs, from the tool it is about to run, needs no link and no prior knowledge. The docs stay for human comprehension, with citations and readable benchmarks; the skill is the other door, derived from the same code and tested against it.

**Precedent.** A survey of seventeen tools (`docs/surveys/2026-09-05-conventions.md`) found one that prints its own agent-facing instructions with a second tier of detail, Vercel Labs' `agent-browser`, whose stated reason is that instructions served from the installed binary always match its version. The same survey records the surrounding conventions this entry adopts alongside the flag: an `llms.txt` on the docsite, an `AGENTS.md` in the repository, and a `CITATION.cff` so the research behind each measure is citable from the repository page.

**Trigger condition. FIRED, 2026-09-05**, by owner direction, sequenced after I10: the skill documents the CLI contract, so the CLI has to be one implementation with a pinned JSON shape first.

## Terminal UI for the Rust CLI

**What it is.** An interactive terminal interface over the same `cli` module: browse a parsed document, its sections, sentences, and dependency trees, and the metrics beside them, without leaving the terminal.

**Trigger condition.** I10 and the agent surface have met their acceptance gates. The TUI is a renderer over a contract that has to be stable first; building it before that means re-doing it.


## Record traceability accessor

**What it is.** A single accessor returning the source path, the detected format, and a record of how the document was analyzed, as one value rather than as fields scattered across `CorpusEntry.path` and `RawDocument.format`.

**Why it is worth doing.** The information is already present and already reconstructable; it is the assembly that is missing. Callers building a grounding chain need it as one thing, and `Sentence.text` and `Paragraph.text` being verbatim by design means the chain from token to source is unambiguous once the anchor is reachable. That guarantee is currently undocumented, which is tracked separately.

**Trigger condition.** A caller that stores matra output and must later prove which bytes a value came from. Raised by the bidirectional research report as ask 4.
