# What vaani illuminates

vaani illuminates the internal structural makeup of text, enabling effective higher-order reasoning on text.

That sentence is the product of deliberate word selection. Not "parses," not "analyzes," not "processes." Illuminates. Because the structure was already there: in the dependency arcs between tokens, in the passive constructions, in the nominalization ratio, in the section boundaries. vaani does not create it. vaani makes it accessible.

## The question behind the design

Why build a substrate at all? Why not build the application: the intent extractor, the voice analyzer, the document classifier?

Because the interpreter belongs to the consumer, not the library.

When a builder assembles a text-reasoning system, they bring a domain model: what counts as intent, what a "strong" voice signature looks like, what a relation extraction result should contain. vaani cannot know this. No library can. What vaani can do is expose every structural signal the text carries (dependency trees, POS tags, sentence boundaries, passive ratios, lexical density, keyphrases) in a typed, stable form that an interpreter can query.

This is the substrate-vs-interpreter line. vaani structures. The reasoning is yours.

## What "substrate" means here

A substrate has three properties that distinguish it from an application.

First, it is format-agnostic about the consumer's purpose. vaani does not know whether you are classifying intent, auditing passive voice, extracting relations, or grounding an LLM prompt. It exposes structure; you apply the purpose. The same `Document` (the same tokens, the same dependency arcs, the same metric fields) serves all of those consumers without modification.

Second, it does not contain judgment calls. Lexical density of 0.41 is a measurement. Whether 0.41 is "too low" for a given document is an application decision that depends on domain, audience, and purpose. vaani reports; you decide. This is not a limitation; it is a feature. A library that embeds judgment cannot be reused across domains with different standards.

Third, it is designed to be composed over, not extended into. Consumer applications sit above vaani and query its output. They do not inherit from vaani types or plug rules into the library's internal pipeline. The composition boundary is clean: vaani produces a `Document`, and everything downstream is yours.

## The committee sentence

Consider two sentences: "The committee approved the proposal without debate." and "Three amendments were submitted by the working group."

Both sentences parse. Both carry full CoNLL-U annotations. In the first sentence, `committee` is `nsubj` of `approved`: the agency is foregrounded. In the second, `amendments` is `nsubj:pass` of `submitted`: the agent is omitted entirely. vaani surfaces both structures. The passive ratio across these two sentences is 0.5.

What does a passive ratio of 0.5 mean? That is your question to answer. A legal drafter writing compliance documentation may find 0.5 appropriate; an editorial writing coach may flag it as too high for a persuasive piece; a political scientist studying committee language may compare it against a baseline of comparable procedural documents. vaani measured. You interpret.

This is the substrate-vs-interpreter distinction in practice. vaani does not tell you what 0.5 means. It tells you, with precision, that half the sentences in this document suppress the grammatical agent.

## What an interpreter is, and why vaani is not one

An interpreter takes vaani's structured output and applies reasoning to produce a judgment or action.

An LLM grounded on a vaani `Document` is one interpreter. The dependency arcs tell it exactly which noun is the subject of which verb. The passive detection flags tell it which sentences suppress the agent. The section boundaries tell it where argument structure shifts. The LLM's reasoning over that structured input is auditable in a way that pattern-matching over raw bytes is not, because the structure names what it found, not just where it landed.

A rule engine evaluating passive-ratio thresholds against a policy document is another interpreter. A human reading a rendered dependency tree is a third. A statistical model trained on vaani's metric fields as features is a fourth.

vaani does not know which interpreter you will use. It does not optimize for any of them. It exposes every structural signal it computes and hands the rest off.

## Why "illuminates"

The word matters. "Parse" names an operation: transforming text into a tree. vaani parses, but parsing is a means, not the purpose. "Analyze" names an act of interpretation: drawing conclusions. vaani does not analyze; you do. "Illuminate" names what actually happens: making visible what was already present.

A dependency tree is latent in any sentence. Every sentence in English has a subject, a main verb, and relations between them, whether or not any software ever reads it. The passive construction "The proposal was approved by the committee" contains an agent, a patient, and a suppressed-agent structure, regardless of whether a human or machine notices. vaani runs the parse, extracts the annotations, and surfaces those structures in a form that can be queried. The structure was there. vaani makes it accessible.

This distinction matters for the design. A library that "generates" structure must be trusted to get it right. A library that "illuminates" existing structure is grounded in what linguists, statisticians, and NLP researchers have established about how language works. The CoNLL-U annotation standard behind vaani's tokens is a community standard used across hundreds of research projects. vaani's dependency parser is UDPipe, trained on the Universal Dependencies treebank. The Flesch-Kincaid grade formula dates to 1975. vaani surfaces these established structures; it does not invent them.

## What vaani is not

Three misconceptions arise often enough to name explicitly.

**vaani is not a writing-quality tool.** It does not score writing or recommend improvements. Grammarly, Vale, and similar tools embed editorial judgment. vaani deliberately does not. `readability_grade = Some(11.4)` is a Flesch-Kincaid measurement; whether grade 11.4 is appropriate for your audience is an editorial judgment that belongs to your application.

**vaani is not embedding-based or generative NLP.** It does not use transformer models or vector representations. Its NLP is structural and rule-based: UDPipe produces CoNLL-U annotations from a trained parser. This means vaani's output is deterministic (same input, same output, always), auditable (every arc is labeled with a known relation), and reproducible (the model SHA is pinned). These are features for builders who need trustable structure, not probabilistic guesses.

**vaani does not replace LLM reasoning.** It provides the grounding layer for it. Without structure, an LLM reasoning about a document is pattern-matching over bytes. With vaani, the LLM has measurable handles: this sentence is passive, this paragraph's nominalization ratio is high, these are the top keyphrases, this section boundary marks a shift in argument. The reasoning becomes auditable because the inputs are named.

## Where vaani sits today

vaani ships two categories of capability:

**The record tier** (what ships in v0.1 ✅):
- Full CoNLL-U structured parse: every token gets lemma, POS tag, dependency relation, and head position
- Tokens nest into Sentences, Sentences into Paragraphs, Paragraphs into Sections, Sections into Document
- Document metrics: Flesch-Kincaid grade per paragraph, lexical density per paragraph, vocabulary type-token ratio, nominalization ratio, passive ratio, brotli compression ratio
- Summarization: TF-IDF and TextRank, both capped at 2,000 sentences
- Keyphrase extraction: RAKE and YAKE, both capped at 200,000 tokens

The record tier is what vaani illuminates today: the tokens, the dependency arcs, the measurements. It is the substrate from which all higher-order reasoning begins.

**The abstract tier** (planned v0.2+ 🛠️):
- Rule evaluation over parsed structure: relation extraction, schema detection, modality markers, speech act classification, voice signature analysis
- Rule evaluation is what closes the gap between the record tier and the conviction's "effective higher-order reasoning"

Rule evaluation will land inside vaani, not as a peer library. Consumers compose against one surface.

## The boundary that makes this real

The substrate-vs-interpreter line is not a metaphor. It is enforced in the type system and in the pipeline.

Metrics are instruments, not scores. `vocabulary_ttr = Some(0.54)` is a measurement. There is no API call that returns "this text is good" or "this writer should be more precise." Consumer applications supply those judgments.

Methods do not cross the FFI boundary. `Document::passive_ratio()` exists as a Rust method that computes a ratio from sentences. It is not a field on the `Document` struct and does not appear in the Python or (future) TypeScript surface. Values that need to be visible cross-language are materialized as fields. The consumer that wants passive ratio in Python reads the sentence-level data and computes it, or vaani materializes a summary field explicitly.

The domain depends only on `serde`, `thiserror`, and `std`. No inference engine, no model weights, no scoring rubric lives in the domain layer. The record is clean.

See [The pipeline](./pipeline.md) for how text moves through the five stages to produce a `Document`. See [Hex layout](./hex.md) for how the dependency boundaries are enforced in code. See [Four faces of voice](./four-faces.md) for the four structured views a consumer can read from a `Document`.
